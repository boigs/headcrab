use axum::extract::ws::{Message, WebSocket};
use serde::Serialize;
use tokio::select;

use crate::actor::game::{GameClient, GameWideEventReceiver};
use crate::domain::player::Player;

use crate::actor::game::GameWideEvent;
use crate::websocket::send_error_and_close;

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
                        Ok(GameWideEvent::GameState { players }) => self.websocket.send(Message::Text(serde_json::to_string(&GameState { players }).unwrap())).await.unwrap(),
                        Err(error) => {
                            send_error_and_close(self.websocket, &error).await;
                            return;
                        },
                    }
                },
                socket_message = self.websocket.recv() => {
                    match socket_message {
                        Some(message) => println!("INFO: Got message from player '{}'.", message.unwrap_or(Message::Text("<Empty>".to_string())).into_text().unwrap_or_default()),
                        None => {
                            println!("INFO: WebSocket with player's client closed. Removing player from game and closing player actor.");
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
