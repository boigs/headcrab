use axum::extract::ws::{Message, WebSocket};
use std::collections::HashMap;
use std::time::Duration;
use tokio::select;
use tokio::time::error::Elapsed;
use tokio::time::timeout;

use crate::error::Error;
use crate::game::actor::GameWideEvent;
use crate::game::actor_client::GameClient;
use crate::game::actor_client::GameWideEventReceiver;
use crate::game::game_fsm::GameFsmState;
use crate::metrics::CONNECTED_PLAYERS;
use crate::player::Player;
use crate::round::Round;
use crate::round::Word;
use crate::websocket::close;
use crate::websocket::message::state_to_string;
use crate::websocket::message::RoundDto;
use crate::websocket::message::WordDto;
use crate::websocket::message::WsMessageIn;
use crate::websocket::message::WsMessageOut;
use crate::websocket::parse_message;
use crate::websocket::send_error;
use crate::websocket::send_message;
use crate::websocket::send_message_string;

pub struct PlayerActor {
    nickname: String,
    game: GameClient,
    game_wide_event_receiver: GameWideEventReceiver,
    websocket: WebSocket,
    inactivity_timeout: Duration,
}

impl PlayerActor {
    pub async fn create(nickname: String, game: GameClient, mut websocket: WebSocket) {
        match game.add_player(&nickname).await {
            Ok(game_wide_event_receiver) => {
                PlayerActor {
                    nickname,
                    game,
                    game_wide_event_receiver,
                    websocket,
                    inactivity_timeout: Duration::from_millis(2500),
                }
                .start()
                .await
            }
            Err(error) => {
                send_error(&mut websocket, &error).await;
                close(websocket).await;
            }
        }
    }

    async fn start(mut self) {
        CONNECTED_PLAYERS.inc();

        loop {
            select! {
                game_wide_message = self.game_wide_event_receiver.next() => {
                    if let Err(error) = self.receive_game_wide_message(game_wide_message).await {
                        send_error(&mut self.websocket, &error).await;
                        if PlayerActor::should_close_websocket(error) {
                            break;
                        }
                    }
                },
                websocket_message = timeout(self.inactivity_timeout, self.websocket.recv()) => {
                    if let Err(error) = self.receive_websocket_message(websocket_message).await {
                        send_error(&mut self.websocket, &error).await;
                        if PlayerActor::should_close_websocket(error) {
                            break;
                        }
                    }
                },
            }
        }

        let _ = self.game.remove_player(&self.nickname).await;
        close(self.websocket).await;
        CONNECTED_PLAYERS.dec();
    }

    fn should_close_websocket(error: Error) -> bool {
        match error {
            Error::Internal(_) => true,
            Error::WebsocketClosed(_) => true,
            Error::UnprocessableMessage(_, _) => false,
            Error::CommandNotAllowed(_, _) => false,
            Error::NotEnoughPlayers => false,
            Error::GameDoesNotExist(_) => false,
            Error::PlayerAlreadyExists(_) => false,
            Error::RepeatedWords => false,
            Error::GameAlreadyInProgress => true,
        }
    }

    async fn receive_game_wide_message(
        &mut self,
        game_wide_message: Result<GameWideEvent, Error>,
    ) -> Result<(), Error> {
        match game_wide_message {
            Ok(GameWideEvent::GameState {
                state,
                players,
                rounds,
            }) => {
                send_message(
                    &mut self.websocket,
                    &PlayerActor::serialize_game_state(&self.nickname, state, players, rounds),
                )
                .await
            }
            Ok(GameWideEvent::ChatMessage { sender, content }) => {
                send_message(
                    &mut self.websocket,
                    &WsMessageOut::ChatMessage {
                        sender: sender.to_string(),
                        content: content.to_string(),
                    },
                )
                .await
            }
            Err(error) => Err(error),
        }
    }

    fn serialize_game_state(
        our_nickname: &str,
        state: GameFsmState,
        players: Vec<Player>,
        rounds: Vec<Round>,
    ) -> WsMessageOut {
        let rounds: Option<Vec<RoundDto>> = rounds.split_last().map(|(last_round, rest)| {
            let last_round = last_round.clone();
            let filtered_words: HashMap<String, Vec<WordDto>> = last_round
                .player_words
                .iter()
                .map(|(nickname, words)| {
                    let words: Vec<Word> = if our_nickname == nickname {
                        words.to_vec()
                    } else {
                        words.iter().filter(|word| word.is_used).cloned().collect()
                    };
                    (
                        nickname.to_string(),
                        words.into_iter().map(|word| word.into()).collect(),
                    )
                })
                .collect();
            let mut rest: Vec<RoundDto> = rest.iter().map(|round| round.clone().into()).collect();
            rest.push(RoundDto {
                word: last_round.word,
                score: last_round.score.into(),
                player_words: filtered_words,
            });
            rest
        });

        WsMessageOut::GameState {
            state: state_to_string(state),
            players: players.into_iter().map(|player| player.into()).collect(),
            rounds: rounds.unwrap_or_default(),
        }
    }

    async fn receive_websocket_message(
        &mut self,
        websocket_message: Result<Option<Result<Message, axum::Error>>, Elapsed>,
    ) -> Result<(), Error> {
        match websocket_message {
            Ok(Some(Ok(Message::Text(txt)))) => match txt.as_str() {
                "ping" => send_message_string(&mut self.websocket, "pong").await,
                message => match parse_message(message) {
                    Ok(WsMessageIn::StartGame { amount_of_rounds }) => {
                        self.game.start_game(&self.nickname).await?;
                        log::info!("Started game with amount of rounds {amount_of_rounds}");
                        Ok(())
                    }
                    Ok(WsMessageIn::ChatMessage { content }) => {
                        self.game.send_chat_message(&self.nickname, &content).await
                    }
                    Ok(WsMessageIn::PlayerWords { words }) => {
                        self.game.add_player_words(&self.nickname, words).await
                    }
                    Ok(WsMessageIn::PlayerWordSubmission { word }) => {
                        self.game
                            .add_player_word_submission(&self.nickname, Some(word))
                            .await
                    }
                    Err(error) => Err(error),
                },
            },
            // browser said "close"
            Ok(Some(Ok(Message::Close(_)))) => {
                self.log_connection_lost_with_player("browser sent 'Close' websocket frame");
                Err(Error::WebsocketClosed(
                    "browser sent 'Close' websocket frame".to_string(),
                ))
            }
            // websocket was closed
            Ok(None) => {
                self.log_connection_lost_with_player("other end of websocket was closed abruptly");
                Err(Error::WebsocketClosed(
                    "other end of websocket was closed abruptly".to_string(),
                ))
            }
            // timeout without receiving anything from player
            Err(_) => {
                self.log_connection_lost_with_player(
                    "connection timed out; missing 'Ping' messages",
                );
                Err(Error::WebsocketClosed(
                    "connection timed out; missing 'Ping' messages".to_string(),
                ))
            }
            Ok(Some(Err(error))) => Err(Error::UnprocessableMessage(
                "Message cannot be loaded".to_string(),
                error.to_string(),
            )),
            Ok(Some(Ok(_))) => Err(Error::UnprocessableMessage(
                "Unsupported message type".to_string(),
                "Unsupported message type".to_string(),
            )),
        }
    }

    fn log_connection_lost_with_player(&self, reason: &str) {
        log::info!(
            "Connection with player {} lost due to: {}. Stopping player actor.",
            &self.nickname,
            reason,
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::error::Error;
    use crate::player::actor::PlayerActor;

    #[test]
    fn should_close_websocket_is_false() {
        assert!(!PlayerActor::should_close_websocket(
            Error::GameDoesNotExist("".to_owned())
        ));
        assert!(!PlayerActor::should_close_websocket(
            Error::PlayerAlreadyExists("".to_owned())
        ));
        assert!(!PlayerActor::should_close_websocket(
            Error::NotEnoughPlayers
        ));
        assert!(!PlayerActor::should_close_websocket(
            Error::CommandNotAllowed("".to_owned(), "".to_owned())
        ));
        assert!(!PlayerActor::should_close_websocket(
            Error::UnprocessableMessage("".to_string(), "".to_string())
        ));
    }

    #[test]
    fn should_close_websocket_is_true() {
        assert!(PlayerActor::should_close_websocket(Error::Internal(
            "".to_owned()
        )));
        assert!(PlayerActor::should_close_websocket(Error::WebsocketClosed(
            "".to_owned()
        )));
    }
}
