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

    if let Err(_) = game_actor
        .send(GameCommand::AddPlayer {
            player: player.clone(),
            player_actor: tx.clone(),
        })
        .await
    {
        println!("The game actor is not alive. Can't add player to game.");
        todo!("This line has been reached because:
        1. the game exists in the GameFactory actor
        2. but the (individual) Game actor has been dropped
        3. the user navigated to this game's URL in an attempt to re-join (and there aren't any other players in the game).
        We need:
        1. When the game is closed (on game actor), delete the game from the game factory as well.
        ");
        // return; // uncomment when todo!() is implemented
    }

    let mut broadcast_receiver = match rx.recv().await {
        Some(GameEvent::PlayerAdded { broadcast_channel }) => broadcast_channel,
        Some(GameEvent::PlayerAlreadyExists) => {
            socket
                .send(Message::Text(
                    "Loco que ya existe un jugador con ese nombre".to_string(),
                ))
                .await
                .unwrap();
            socket.close().await.unwrap();
            return;
        }
        _ => panic!("Channel closed or something"), // TODO @sergi
    };

    loop {
        select! {
            game_wide_message = broadcast_receiver.recv() => {
                match game_wide_message {
                    Ok(GameWideEvent::GameState { players }) => socket.send(Message::Text(serde_json::to_string(&GameState { players }).unwrap())).await.unwrap(),
                    Err(_) => panic!("aaaaaaaa"),
                }
            },
            socket_message = socket.recv() => {
                match socket_message {
                    Some(message) => println!("Got message from player {}", message.unwrap_or(Message::Text("".to_string())).into_text().unwrap_or_default()),
                    None => {
                        println!("WebSocket with player's client closed. Removing player from game and closing player actor.");
                        let _ = game_actor.send(GameCommand::RemovePlayer {
                            player: player.clone(),
                         }).await;
                         return;
                    },
                }
            },
            game_event = rx.recv() => {
                match game_event {
                    None => panic!("Channel with game actor closed. How did this happen? Did somebody forget calling clone on tx?"),
                    _ => todo!("Other branches"),
                }
            },
        }
    }
}

#[derive(Serialize)]
struct GameState {
    players: Vec<Player>,
}
