use axum::extract::ws::{Message, WebSocket};
use serde::Serialize;
use std::sync::Arc;
use tokio::{
    select,
    sync::{
        broadcast,
        mpsc::{self, Receiver, Sender},
        oneshot::{self, Receiver as OneshotReceiver, Sender as OneshotSender},
    },
};

use crate::actor::game::{GameCommand, GameEvent};
use crate::domain::player::Player;

use crate::actor::game::GameWideEvent;
use crate::actor::game_factory::{GameFactoryCommand, GameFactoryResponse};

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
    pub async fn create(
        nickname: String,
        game_id: String,
        game_factory_tx: Arc<Sender<GameFactoryCommand>>,
        socket: WebSocket,
    ) {
        match PlayerActor::create_internal(&nickname, &game_id, game_factory_tx).await {
            Ok(player_actor) => player_actor.start(socket).await,
            Err(error) => send_error_and_close(socket, &error).await,
        }
    }

    async fn create_internal(
        nickname: &str,
        game_id: &str,
        game_factory_tx: Arc<Sender<GameFactoryCommand>>,
    ) -> Result<Self, String> {
        let game_tx = get_game(game_factory_tx, game_id).await?;
        let player = Player::new(&nickname);
        let (player_tx, player_rx, broadcast_rx) = add_player_to_game(&player, &game_tx).await?;
        Ok(PlayerActor {
            player,
            player_tx,
            player_rx,
            game_tx,
            broadcast_rx,
        })
    }

    async fn start(mut self, mut socket: WebSocket) {
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

async fn get_game(
    sender: Arc<Sender<GameFactoryCommand>>,
    game_id: &str,
) -> Result<Sender<GameCommand>, String> {
    let (tx, rx): (
        OneshotSender<GameFactoryResponse>,
        OneshotReceiver<GameFactoryResponse>,
    ) = oneshot::channel();

    if sender
        .send(GameFactoryCommand::GetGameActor {
            game_id: game_id.to_string(),
            response_channel: tx,
        })
        .await
        .is_err()
    {
        return Err("ERROR: The GameFactory channel is closed.".to_string());
    }

    match rx.await {
        Ok(GameFactoryResponse::GameActor { game_channel }) => Ok(game_channel),
        Ok(GameFactoryResponse::GameNotFound) => Err("Game not found.".to_string()),
        Err(error) => {
            println!("ERROR: The Game channel is closed. Error: {error}.");
            Err(format!(
                "ERROR: The Game channel is closed. Error: {error}."
            ))
        }
        Ok(unexpected_response) => {
            println!(
                "ERROR: Received an unexpected GameFactoryResponse. Response: {unexpected_response}.",
            );
            Err(format!(
                "ERROR: Received an unexpected GameFactoryResponse. Error {unexpected_response}."
            ))
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
