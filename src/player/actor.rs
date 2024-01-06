use axum::extract::ws::{Message, WebSocket};
use std::time::Duration;
use tokio::select;
use tokio::time::error::Elapsed;
use tokio::time::timeout;

use crate::error::Error;
use crate::game::actor::GameWideEvent;
use crate::game::actor_client::GameClient;
use crate::game::actor_client::GameWideEventReceiver;
use crate::metrics::CONNECTED_PLAYERS;
use crate::websocket::close;
use crate::websocket::message::state_to_string;
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
                send_error(&mut websocket, error).await;
                close(websocket).await;
            }
        }
    }

    async fn start(mut self) {
        CONNECTED_PLAYERS.inc();

        loop {
            select! {
                game_wide_message = self.game_wide_event_receiver.next() => {
                    if self.receive_game_wide_message(game_wide_message).await.is_err() {
                        break;
                    }
                },
                websocket_message = timeout(self.inactivity_timeout, self.websocket.recv()) => {
                    if self.receive_websocket_message(websocket_message).await.is_err() {
                        break;
                    }
                },
            }
        }

        let _ = self.game.remove_player(&self.nickname).await;
        close(self.websocket).await;
        CONNECTED_PLAYERS.dec();
    }

    async fn receive_game_wide_message(
        &mut self,
        game_wide_message: Result<GameWideEvent, Error>,
    ) -> Result<(), ()> {
        match game_wide_message {
            Ok(GameWideEvent::GameState {
                state,
                players,
                rounds,
            }) => {
                if let Err(error) = send_message(
                    &mut self.websocket,
                    &WsMessageOut::GameState {
                        state: state_to_string(state),
                        players: players.into_iter().map(|player| player.into()).collect(),
                        rounds: rounds.into_iter().map(|round| round.into()).collect(),
                    },
                )
                .await
                {
                    send_error(&mut self.websocket, error).await;
                    return Err(());
                }
            }
            Ok(GameWideEvent::ChatMessage { sender, content }) => {
                if let Err(error) = send_message(
                    &mut self.websocket,
                    &WsMessageOut::ChatMessage {
                        sender: sender.to_string(),
                        content: content.to_string(),
                    },
                )
                .await
                {
                    send_error(&mut self.websocket, error).await;
                    return Err(());
                }
            }
            Err(error) => {
                send_error(&mut self.websocket, error).await;
                return Err(());
            }
        }
        Ok(())
    }

    async fn receive_websocket_message(
        &mut self,
        websocket_message: Result<Option<Result<Message, axum::Error>>, Elapsed>,
    ) -> Result<(), ()> {
        match websocket_message {
            Ok(Some(Ok(Message::Text(txt)))) => match txt.as_str() {
                "ping" => {
                    if let Err(error) = send_message_string(&mut self.websocket, "pong").await {
                        send_error(&mut self.websocket, error).await;
                        return Err(());
                    }
                }
                message => match parse_message(message) {
                    Ok(WsMessageIn::StartGame { amount_of_rounds }) => {
                        if let Err(error) = self.game.start_game(&self.nickname).await {
                            send_error(&mut self.websocket, error).await;
                            return Err(());
                        } else {
                            log::info!("Started game with amount of rounds {amount_of_rounds}");
                        }
                    }
                    Ok(WsMessageIn::ChatMessage { content }) => {
                        if let Err(error) =
                            self.game.send_chat_message(&self.nickname, &content).await
                        {
                            send_error(&mut self.websocket, error).await;
                            return Err(());
                        }
                    }
                    Err(error) => {
                        send_error(&mut self.websocket, error).await;
                    }
                },
            },
            // browser said "close"
            Ok(Some(Ok(Message::Close(_)))) => {
                self.log_connection_lost_with_player("browser sent 'Close' websocket frame");
                return Err(());
            }
            // websocket was closed
            Ok(None) => {
                self.log_connection_lost_with_player("other end of websocket was closed abruptly");
                return Err(());
            }
            // timeout without receiving anything from player
            Err(_) => {
                self.log_connection_lost_with_player(
                    "connection timed out; missing 'Ping' messages",
                );
                return Err(());
            }
            Ok(Some(Err(error))) => {
                send_error(
                    &mut self.websocket,
                    Error::UnprocessableMessage(
                        "Message cannot be loaded".to_string(),
                        error.to_string(),
                    ),
                )
                .await;
            }
            Ok(Some(Ok(_))) => {
                send_error(
                    &mut self.websocket,
                    Error::UnprocessableMessage(
                        "Unsupported message type".to_string(),
                        "Unsupported message type".to_string(),
                    ),
                )
                .await;
            }
        }
        Ok(())
    }

    fn log_connection_lost_with_player(&self, reason: &str) {
        log::info!(
            "Connection with player {} lost due to: {}. Stopping player actor.",
            &self.nickname,
            reason,
        );
    }
}
