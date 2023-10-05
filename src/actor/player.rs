use axum::extract::ws::{Message, WebSocket};
use serde::Serialize;
use tokio::{
    select,
    sync::{
        broadcast,
        mpsc::{self, Receiver, Sender},
    },
};

use crate::actor::game::{GameCommand, GameEvent};
use crate::domain::player::Player;

use crate::actor::game::GameWideEvent;
use crate::websocket::send_error_and_close;

pub struct PlayerActor {
    player: Player,
    // player_tx will be used in the future, it's important to keep a reference to it so that the channel is not closed
    #[allow(dead_code)]
    player_tx: Sender<GameEvent>,
    player_rx: Receiver<GameEvent>,
    game_tx: Sender<GameCommand>,
    broadcast_rx: broadcast::Receiver<GameWideEvent>,
    websocket: WebSocket,
}

impl PlayerActor {
    pub async fn create(nickname: String, game_tx: Sender<GameCommand>, websocket: WebSocket) {
        let player = Player::new(&nickname);
        match add_player_to_game(&player, &game_tx).await {
            Ok((player_tx, player_rx, broadcast_rx)) => {
                PlayerActor {
                    player,
                    player_tx,
                    player_rx,
                    game_tx,
                    broadcast_rx,
                    websocket,
                }
                .start()
                .await
            }
            Err(error) => send_error_and_close(websocket, error).await,
        }
    }

    async fn start(mut self) {
        loop {
            select! {
                game_wide_message = self.broadcast_rx.recv() => {
                    match game_wide_message {
                        Ok(GameWideEvent::GameState { players }) => self.websocket.send(Message::Text(serde_json::to_string(&GameState { players }).unwrap())).await.unwrap(),
                        Err(_) => {
                            println!("ERROR: The broadcast channel with the Game has been closed.");
                            send_error_and_close(self.websocket, "ERROR: Internal Server Error.").await;
                            return;
                        },
                    }
                },
                game_event = self.player_rx.recv() => {
                    match game_event {
                        None => {
                            println!("ERROR: Private channel with Game closed. How did this happen? Did somebody forget calling clone on tx?.");
                            send_error_and_close(self.websocket, "ERROR: Internal Server Error.").await;
                            return;
                        },
                        _ => println!("INFO: Received message from GameActor on the private channel."),
                    }
                },
                socket_message = self.websocket.recv() => {
                    match socket_message {
                        Some(message) => println!("INFO: Got message from player '{}'.", message.unwrap_or(Message::Text("<Empty>".to_string())).into_text().unwrap_or_default()),
                        None => {
                            println!("INFO: WebSocket with player's client closed. Removing player from game and closing player actor.");
                            if self.game_tx.send(GameCommand::RemovePlayer {
                                player: self.player,
                            }).await.is_err() {
                                println!("ERROR: Tried to send GameCommand:RemovePlayer but GameActor is not listening.");
                            };
                            return;
                        },
                    }
                },
            }
        }
    }
}

async fn add_player_to_game(
    player: &Player,
    game_tx: &Sender<GameCommand>,
) -> Result<
    (
        Sender<GameEvent>,
        Receiver<GameEvent>,
        broadcast::Receiver<GameWideEvent>,
    ),
    &'static str,
> {
    let (player_tx, mut player_rx): (Sender<GameEvent>, Receiver<GameEvent>) = mpsc::channel(32);

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
        return Err("ERROR: The Game is not alive. Can't add Player to Game.");
    }

    let broadcast_rx = match player_rx.recv().await {
        Some(GameEvent::PlayerAdded { broadcast_channel }) => broadcast_channel,
        Some(GameEvent::PlayerAlreadyExists) => {
            return Err("ERROR: The Player already exists.");
        }
        _ => {
            println!("ERROR: Player sent a GameCommand::AddPlayer to Game, but Game channel died.");
            return Err(
                "ERROR: Player sent a GameCommand::AddPlayer to Game, but Game channel died.",
            );
        }
    };

    Ok((player_tx, player_rx, broadcast_rx))
}

#[derive(Serialize)]
struct GameState {
    players: Vec<Player>,
}
