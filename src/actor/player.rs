use axum::extract::ws::{Message, WebSocket};
use serde::Serialize;
use std::time::Duration;
use tokio::select;
use tokio::time::timeout;

use crate::actor::game::client::GameClient;
use crate::actor::game::client::GameWideEventReceiver;
use crate::actor::game::GameWideEvent;
use crate::domain::player::Player;

use crate::websocket::{send_error_and_close, send_game_state};

pub struct PlayerActor {
    player: Player,
    game: GameClient,
    game_wide_event_receiver: GameWideEventReceiver,
    websocket: WebSocket,
}

impl PlayerActor {
    pub async fn create(nickname: String, game: GameClient, websocket: WebSocket) {
        let player = Player::new(&nickname);
        match game.add_player(player.clone()).await {
            Ok(game_wide_event_receiver) => {
                PlayerActor {
                    player,
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
                        Ok(GameWideEvent::GameState { players }) => send_game_state(&mut self.websocket, players).await,
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
                                    if let Err(error) = self.game.remove_player(self.player).await {
                                        log::error!("{error}");
                                    };
                                    return;
                                }
                            },
                            _ => log::info!("message"),
                        },
                        Ok(Some(Ok(Message::Close(_)))) | // browser said "close"
                        Ok(Some(Err(_))) | // unprocessable message
                        Ok(None) | // websocket was closed
                        Err(_) // timeout was met
                        => {
                            log::info!("WebSocket with player's client closed. Removing player from game and closing player actor.");
                            if let Err(error) = self.game.remove_player(self.player).await {
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

#[derive(Serialize)]
struct GameState {
    players: Vec<Player>,
}
