use axum::extract::ws::{Message, WebSocket};
use std::time::Duration;
use tokio::select;
use tokio::time::timeout;

use crate::actor::game::client::GameClient;
use crate::actor::game::client::GameWideEventReceiver;
use crate::actor::game::GameWideEvent;

use crate::websocket::message::WsMessageIn;
use crate::websocket::parse_message;
use crate::websocket::send_chat_message;
use crate::websocket::{send_error_and_close, send_game_state};

pub struct PlayerActor {
    nickname: String,
    game: GameClient,
    game_wide_event_receiver: GameWideEventReceiver,
    websocket: WebSocket,
}

impl PlayerActor {
    pub async fn create(nickname: String, game: GameClient, websocket: WebSocket) {
        match game.add_player(&nickname).await {
            Ok(game_wide_event_receiver) => {
                PlayerActor {
                    nickname,
                    game,
                    game_wide_event_receiver,
                    websocket,
                }
                .start()
                .await
            }
            Err(error) => send_error_and_close(websocket, &error).await,
        }
    }

    async fn start(mut self) {
        loop {
            select! {
                game_wide_message = self.game_wide_event_receiver.next() => {
                    match game_wide_message {
                        Ok(GameWideEvent::GameState { state, players }) => send_game_state(&mut self.websocket, state, players).await,
                        Ok(GameWideEvent::ChatMessage { sender, content }) => send_chat_message(&mut self.websocket, &sender, &content).await,
                        Err(error) => {
                            send_error_and_close(self.websocket, &error).await;
                            return;
                        },
                    }
                },
                timeout_result = timeout(Duration::from_millis(2500), self.websocket.recv()) => {
                    match timeout_result {
                        Ok(Some(Ok(Message::Text(txt)))) => match txt.as_str() {
                            "ping" => {
                                if self.websocket.send(Message::Text("pong".to_string())).await.is_err() {
                                    log::info!("WebSocket with player's client closed. Removing player from game and closing player actor.");
                                    if let Err(error) = self.game.remove_player(&self.nickname).await {
                                        log::error!("{error}");
                                    };
                                    return;
                                }
                            },
                            message => {
                                match parse_message(message) {
                                    Ok(WsMessageIn::StartGame {amount_of_rounds}) => if let Err(error) = self.game.start_game(&self.nickname).await {
                                        send_error_and_close(self.websocket, &error).await;
                                        return;
                                    } else {
                                        log::info!("Started game with amount of rounds {amount_of_rounds}");
                                    },
                                    Ok(WsMessageIn::ChatMessage {content}) => if self.game.send_chat_message(&self.nickname, &content).await.is_err() {
                                        log::info!("Could not send chat message to game {content}");
                                    },
                                    Err(err) => log::error!("Unprocessable message '{message}, error: {err}'"),
                                }
                            },
                        },
                        Ok(Some(Ok(Message::Close(_)))) | // browser said "close"
                        Ok(Some(Err(_))) | // unprocessable message
                        Ok(None) | // websocket was closed
                        Err(_) // timeout was met
                        => {
                            log::info!("WebSocket with player's client closed. Removing player from game and closing player actor.");
                            if let Err(error) = self.game.remove_player(&self.nickname).await {
                                log::error!("{error}");
                            };
                            return;
                        },
                        Ok(_) => log::warn!("Unexpected type of message received. How did this happen?"),
                    }
                },
            }
        }
    }
}
