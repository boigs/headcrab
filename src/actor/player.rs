use axum::extract::ws::{Message, WebSocket};
use serde::Serialize;
use tokio::sync::broadcast;
use tokio::{
    select,
    sync::mpsc::{self, Receiver, Sender},
};

use crate::actor::game::{GameCommand, GameEvent};
use crate::domain::player::Player;

use crate::actor::game::GameWideEvent;

pub struct PlayerActor {
    player: Player,
    // player_tx will be used in the future, it's important to keep a reference to it so that the channel is not closed
    #[allow(dead_code)]
    player_tx: Sender<GameEvent>,
    player_rx: Receiver<GameEvent>,
    game_tx: Sender<GameCommand>,
    broadcast_rx: broadcast::Receiver<GameWideEvent>,
}

impl PlayerActor {
    pub async fn new(nickname: String, game_tx: Sender<GameCommand>) -> Result<Self, String> {
        let player = Player::new(&nickname);
        let (player_tx, mut player_rx): (Sender<GameEvent>, Receiver<GameEvent>) =
            mpsc::channel(32);

        if game_tx
            .send(GameCommand::AddPlayer {
                player: player.clone(),
                player_actor: player_tx.clone(),
            })
            .await
            .is_err()
        {
            // This line has been reached because:
            //  1. the game exists in the GameFactory actor
            //  2. but the (individual) Game actor has been dropped
            //  3. the user navigated to this game's URL in an attempt to re-join (and there aren't any other players in the game).
            // We need:
            //  1. When the game is closed (on game actor), delete the game from the game factory as well.
            //  2. send message through WS telling the client that this game does not exist.
            println!("ERROR: The Game is not alive. Can't add Player to Game.");
            return Err("ERROR: The Game is not alive. Can't add Player to Game.".to_string());
        }

        let broadcast_rx = match player_rx.recv().await {
            Some(GameEvent::PlayerAdded { broadcast_channel }) => broadcast_channel,
            Some(GameEvent::PlayerAlreadyExists) => {
                return Err("ERROR: The Player already exists.".to_string());
            }
            _ => {
                println!(
                    "ERROR: Player sent a GameCommand::AddPlayer to Game, but Game channel died"
                );
                return Err(
                    "ERROR: Player sent a GameCommand::AddPlayer to Game, but Game channel died."
                        .to_string(),
                );
            }
        };

        Ok(PlayerActor {
            player,
            player_tx,
            player_rx,
            game_tx,
            broadcast_rx,
        })
    }

    pub async fn handler(mut self, mut socket: WebSocket) {
        loop {
            select! {
                game_wide_message = self.broadcast_rx.recv() => {
                    match game_wide_message {
                        Ok(GameWideEvent::GameState { players }) => socket.send(Message::Text(serde_json::to_string(&GameState { players }).unwrap())).await.unwrap(),
                        Err(_) => {
                            println!("ERROR: The broadcast channel with the Game has been closed.");
                            send_error_and_close(socket, "ERROR: Internal Server Error.").await;
                            return;
                        },
                    }
                },
                game_event = self.player_rx.recv() => {
                    match game_event {
                        None => {
                            println!("ERROR: Private channel with Game closed. How did this happen? Did somebody forget calling clone on tx?.");
                            send_error_and_close(socket, "ERROR: Internal Server Error.").await;
                            return;
                        },
                        _ => println!("INFO: Received message from GameActor on the private channel."),
                    }
                },
                socket_message = socket.recv() => {
                    match socket_message {
                        Some(message) => println!("INFO: Got message from player {}", message.unwrap_or(Message::Text("<Empty>".to_string())).into_text().unwrap_or_default()),
                        None => {
                            println!("INFO: WebSocket with player's client closed. Removing player from game and closing player actor.");
                            if self.game_tx.send(GameCommand::RemovePlayer {
                                player: self.player,
                            }).await.is_err() {
                                println!("ERROR: Tried to send GameCommand:RemovePlayer but GameActor is not listening");
                            };
                            return;
                        },
                    }
                },
            }
        }
    }
}

async fn send_error_and_close(mut websocket: WebSocket, message: &str) {
    if websocket
        .send(Message::Text(message.to_string()))
        .await
        .is_err()
    {
        println!("ERROR: Sent Error '{message}' to the browser but the WebSocket is closed.")
    }
    if websocket.close().await.is_err() {
        println!("ERROR: Could not close WebSocket after sending an error.")
    }
}

#[derive(Serialize)]
struct GameState {
    players: Vec<Player>,
}
