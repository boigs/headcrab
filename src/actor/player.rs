use axum::extract::ws::{Message, WebSocket};
use serde::Serialize;
use tokio::{
    select,
    sync::mpsc::{self, Receiver, Sender},
};

use crate::actor::game::{GameCommand, GameEvent};
use crate::domain::player::Player;

use crate::actor::game::GameWideEvent;

pub async fn handler(mut socket: WebSocket, nickname: String, game_actor: Sender<GameCommand>) {
    let player = Player::new(&nickname);
    let (tx, mut rx): (Sender<GameEvent>, Receiver<GameEvent>) = mpsc::channel(32);

    if game_actor
        .send(GameCommand::AddPlayer {
            player: player.clone(),
            player_actor: tx.clone(),
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
        send_error_and_close(
            socket,
            "ERROR: The Game is not alive. Can't add Player to Game.",
        )
        .await;
        return;
    }

    let mut broadcast_receiver = match rx.recv().await {
        Some(GameEvent::PlayerAdded { broadcast_channel }) => broadcast_channel,
        Some(GameEvent::PlayerAlreadyExists) => {
            send_error_and_close(socket, "ERROR: The Player already exists.").await;
            return;
        }
        _ => {
            println!("ERROR: Player sent a GameCommand::AddPlayer to Game, but Game channel died");
            send_error_and_close(
                socket,
                "ERROR: Player sent a GameCommand::AddPlayer to Game, but Game channel died.",
            )
            .await;
            return;
        }
    };

    loop {
        select! {
            game_wide_message = broadcast_receiver.recv() => {
                match game_wide_message {
                    Ok(GameWideEvent::GameState { players }) => socket.send(Message::Text(serde_json::to_string(&GameState { players }).unwrap())).await.unwrap(),
                    Err(_) => {
                        println!("ERROR: The broadcast channel with the Game has been closed.");
                        send_error_and_close(socket, "ERROR: The broadcast channel with the Game has been closed.").await;
                        return;
                    },
                }
            },
            game_event = rx.recv() => {
                match game_event {
                    None => {
                        println!("ERROR: Private channel with Game closed. How did this happen? Did somebody forget calling clone on tx?.");
                        send_error_and_close(socket, "ERROR: Private channel with Game closed. How did this happen? Did somebody forget calling clone on tx?.").await;
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
                        if game_actor.send(GameCommand::RemovePlayer {
                            player,
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
