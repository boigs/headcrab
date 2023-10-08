use axum::extract::ws::{Message, WebSocket};
use serde::{Deserialize, Serialize};

use crate::domain::player::Player;

pub async fn send_error_and_close(mut websocket: WebSocket, message: &str) {
    if websocket
        .send(Message::Text(
            serde_json::to_string(&WsMessage::Error {
                message: message.to_string(),
            })
            .unwrap(),
        ))
        .await
        .is_err()
    {
        println!("ERROR: Sent Error '{message}' to the browser but the WebSocket is closed.")
    }
    if websocket.close().await.is_err() {
        println!("ERROR: Could not close WebSocket after sending an error.")
    }
}

pub async fn send_game_state(websocket: &mut WebSocket, players: Vec<Player>) {
    if websocket
        .send(Message::Text(
            serde_json::to_string(&WsMessage::GameState { players }).unwrap(),
        ))
        .await
        .is_err()
    {
        println!("ERROR: Sent GameState to the browser but the WebSocket is closed.")
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum WsMessage {
    Error { message: String },
    GameState { players: Vec<Player> },
}
