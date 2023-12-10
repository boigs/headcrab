use axum::extract::ws::{Message, WebSocket};
use std::time::Duration;
use tokio::select;
use tokio::time::timeout;

use crate::actor::game::client::GameClient;
use crate::actor::game::client::GameWideEventReceiver;
use crate::actor::game::GameWideEvent;

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
                    match game_wide_message {
                        Ok(GameWideEvent::GameState { state, players, rounds }) => if let Err(error) = send_message(&mut self.websocket, &WsMessageOut::GameState {
                            state: state_to_string(state),
                            players: players.into_iter().map(|player| player.into()).collect(),
                            rounds: rounds.into_iter().map(|round| round.into()).collect(),
                        }).await {
                            send_error(&mut self.websocket, error).await;
                            break;
                        },
                        Ok(GameWideEvent::ChatMessage { sender, content }) => if let Err(error) = send_message(&mut self.websocket, &WsMessageOut::ChatMessage {
                            sender: sender.to_string(),
                            content: content.to_string(),
                        }).await {
                            send_error(&mut self.websocket, error).await;
                            break;
                        },
                        Err(error) => {
                            send_error(&mut self.websocket, error).await;
                            break;
                        },
                    }
                },
                timeout_result = timeout(Duration::from_millis(2500), self.websocket.recv()) => {
                    match timeout_result {
                        Ok(Some(Ok(Message::Text(txt)))) => match txt.as_str() {
                            "ping" => {
                                if let Err(error) = send_message_string(&mut self.websocket, "pong").await {
                                    send_error(&mut self.websocket, error).await;
                                    break;
                                }
                            },
                            message => {
                                match parse_message(message) {
                                    Ok(WsMessageIn::StartGame { amount_of_rounds }) => if let Err(error) = self.game.start_game(&self.nickname).await {
                                        send_error(&mut self.websocket, error).await;
                                        break;
                                    } else {
                                        log::info!("Started game with amount of rounds {amount_of_rounds}");
                                    },
                                    Ok(WsMessageIn::ChatMessage { content }) => if let Err(error) = self.game.send_chat_message(&self.nickname, &content).await {
                                        send_error(&mut self.websocket, error).await;
                                        break;
                                    },
                                    Err(_) => {}
                                }
                            },
                        },
                        Ok(Some(Ok(Message::Close(_)))) | // browser said "close"
                        Ok(Some(Err(_))) | // unprocessable message TODO fix me in issue #78
                        Ok(None) | // websocket was closed
                        Err(_) // timeout was met
                        => {
                            log::info!("WebSocket with player's client closed. Removing player from game and closing player actor.");
                            break;
                        },
                        Ok(_) => log::warn!("Unexpected type of message received. How did this happen?"),
                    }
                },
            }
        }

        let _ = self.game.remove_player(&self.nickname).await;
        close(self.websocket).await;
        CONNECTED_PLAYERS.dec();
    }
}
