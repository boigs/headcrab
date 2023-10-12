use axum::extract::ws::{Message, WebSocket};
use serde::Serialize;
use tokio::select;

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
                socket_message = self.websocket.recv() => {
                    match socket_message {
                        Some(message) => log::info!("Got message from player '{}'.", message.unwrap_or(Message::Text("<Empty>".to_string())).into_text().unwrap_or_default()),
                        None => {
                            log::info!("WebSocket with player's client closed. Removing player from game and closing player actor.");
                            if let Err(error) = self.game.remove_player(self.player).await {
                                println!("{error}");
                            };
                            return;
                        },
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
