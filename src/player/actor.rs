use axum::extract::ws::{Message, WebSocket};
use std::collections::HashMap;
use std::time::Duration;
use tokio::select;
use tokio::time::error::Elapsed;
use tokio::time::timeout;

use crate::error::domain_error::DomainError;
use crate::error::external_error::ExternalError;
use crate::error::Error;
use crate::game::actor::GameWideEvent;
use crate::game::actor_client::GameClient;
use crate::game::actor_client::GameWideEventReceiver;
use crate::game::game_fsm::GameFsmState;
use crate::game::nickname::Nickname;
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
    nickname: Nickname,
    game: GameClient,
    game_wide_event_receiver: GameWideEventReceiver,
    websocket: WebSocket,
    inactivity_timeout: Duration,
}

impl PlayerActor {
    pub async fn create(nickname: Nickname, game: GameClient, mut websocket: WebSocket) {
        match game.add_player(&nickname).await {
            Ok(game_wide_event_receiver) => {
                PlayerActor {
                    nickname,
                    game,
                    game_wide_event_receiver,
                    websocket,
                    inactivity_timeout: Duration::from_millis(5000),
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

        let _ = self.game.remove_player(self.nickname.as_slice()).await;
        close(self.websocket).await;
        CONNECTED_PLAYERS.dec();
    }

    fn should_close_websocket(error: Error) -> bool {
        matches!(
            error,
            Error::Domain(DomainError::GameAlreadyInProgress(_))
                | Error::Domain(DomainError::GameDoesNotExist(_))
                | Error::Domain(DomainError::PlayerAlreadyExists(_))
                | Error::External(ExternalError::WebsocketClosed(_))
                | Error::Internal(_)
        )
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
                amount_of_rounds,
            }) => {
                send_message(
                    &mut self.websocket,
                    &PlayerActor::serialize_game_state(
                        self.nickname.as_slice(),
                        state,
                        players,
                        rounds,
                        amount_of_rounds,
                    ),
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
        amount_of_rounds: Option<u8>,
    ) -> WsMessageOut {
        let rounds: Option<Vec<RoundDto>> = rounds.split_last().map(|(last_round, rest)| {
            let last_round = last_round.clone();
            let current_voting_player_nickname = last_round
                .voting_item
                .clone()
                .map(|voting_item| voting_item.player_nickname);
            let filtered_words: HashMap<String, Vec<WordDto>> = last_round
                .player_words
                .iter()
                .map(|(nickname, words)| {
                    let words: Vec<Word> = if our_nickname == nickname
                        || current_voting_player_nickname == Some(nickname.to_string())
                    {
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
            let mut last_round: RoundDto = last_round.into();
            last_round.player_words = filtered_words;
            let mut rest: Vec<RoundDto> = rest.iter().map(|round| round.clone().into()).collect();
            rest.push(last_round);
            rest
        });

        WsMessageOut::GameState {
            state: state_to_string(state),
            players: players.into_iter().map(|player| player.into()).collect(),
            rounds: rounds.unwrap_or_default(),
            amount_of_rounds,
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
                        self.game
                            .start_game(self.nickname.as_slice(), amount_of_rounds)
                            .await?;
                        log::info!("Started game with amount of rounds {amount_of_rounds}");
                        Ok(())
                    }
                    Ok(WsMessageIn::ChatMessage { content }) => {
                        self.game
                            .send_chat_message(self.nickname.as_slice(), &content)
                            .await
                    }
                    Ok(WsMessageIn::PlayerWords { words }) => {
                        self.game
                            .add_player_words(self.nickname.as_slice(), words)
                            .await
                    }
                    Ok(WsMessageIn::PlayerVotingWord { word }) => {
                        self.game
                            .add_player_voting_word(self.nickname.as_slice(), word)
                            .await
                    }
                    Ok(WsMessageIn::AcceptPlayersVotingWords) => {
                        self.game
                            .accept_players_voting_words(self.nickname.as_slice())
                            .await
                    }
                    Ok(WsMessageIn::ContinueToNextRound) => {
                        self.game
                            .continue_to_next_round(self.nickname.as_slice())
                            .await
                    }
                    Ok(WsMessageIn::PlayAgain) => {
                        self.game.play_again(self.nickname.as_slice()).await
                    }
                    Ok(WsMessageIn::RejectMatchedWord {
                        rejected_player,
                        rejected_word,
                    }) => {
                        self.game
                            .reject_matched_word(
                                self.nickname.as_slice(),
                                rejected_player,
                                rejected_word,
                            )
                            .await
                    }
                    Err(error) => Err(error),
                },
            },
            // browser said "close"
            Ok(Some(Ok(Message::Close(_)))) => {
                self.log_connection_lost_with_player("browser sent 'Close' websocket frame");
                Err(Error::External(ExternalError::WebsocketClosed(
                    "browser sent 'Close' websocket frame".to_string(),
                )))
            }
            // websocket was closed
            Ok(None) => {
                self.log_connection_lost_with_player("other end of websocket was closed abruptly");
                Err(Error::External(ExternalError::WebsocketClosed(
                    "other end of websocket was closed abruptly".to_string(),
                )))
            }
            // timeout without receiving anything from player
            Err(_) => {
                self.log_connection_lost_with_player(
                    "connection timed out; missing 'Ping' messages",
                );
                Err(Error::External(ExternalError::WebsocketClosed(
                    "connection timed out; missing 'Ping' messages".to_string(),
                )))
            }
            Ok(Some(Err(error))) => Err(Error::External(
                ExternalError::UnprocessableWebsocketMessage(
                    "Message cannot be loaded".to_string(),
                    error.to_string(),
                ),
            )),
            Ok(Some(Ok(_))) => Err(Error::External(
                ExternalError::UnprocessableWebsocketMessage(
                    "Unsupported message type".to_string(),
                    "Unsupported message type".to_string(),
                ),
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
    use crate::error::domain_error::DomainError;
    use crate::error::external_error::ExternalError;
    use crate::error::Error;
    use crate::player::actor::PlayerActor;

    #[test]
    fn should_close_websocket_is_false() {
        assert!(!PlayerActor::should_close_websocket(Error::Domain(
            DomainError::NonHostPlayerCannotContinueToNextRound("".to_string())
        )));
        assert!(!PlayerActor::should_close_websocket(Error::Domain(
            DomainError::NotEnoughRounds(0, 0)
        )));
        assert!(!PlayerActor::should_close_websocket(Error::Domain(
            DomainError::NotEnoughPlayers(0, 0)
        )));
        assert!(!PlayerActor::should_close_websocket(Error::External(
            ExternalError::UnprocessableWebsocketMessage("".to_string(), "".to_string())
        )));
    }

    #[test]
    fn should_close_websocket_is_true() {
        assert!(PlayerActor::should_close_websocket(Error::Domain(
            DomainError::GameDoesNotExist("".to_string())
        )));
        assert!(PlayerActor::should_close_websocket(Error::Domain(
            DomainError::PlayerAlreadyExists("".into())
        )));
        assert!(PlayerActor::should_close_websocket(Error::Domain(
            DomainError::GameAlreadyInProgress("".to_string())
        )));
        assert!(PlayerActor::should_close_websocket(Error::Internal(
            "".to_string()
        )));
        assert!(PlayerActor::should_close_websocket(Error::External(
            ExternalError::WebsocketClosed("".to_string())
        )));
    }
}
