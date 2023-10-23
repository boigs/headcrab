mod message;

use axum::extract::ws::{Message, WebSocket};

use crate::domain::player::Player;
use message::WsMessage;

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
        log::error!("Sent Error '{message}' to the browser but the WebSocket is closed.")
    }
    if websocket.close().await.is_err() {
        log::error!("Could not close WebSocket after sending an error.")
    }
}

pub async fn send_game_state(websocket: &mut WebSocket, players: Vec<Player>) {
    if websocket
        .send(Message::Text(
            serde_json::to_string(&WsMessage::GameState {
                players: players.into_iter().map(|player| player.into()).collect(),
            })
            .unwrap(),
        ))
        .await
        .is_err()
    {
        log::error!("Sent GameState to the browser but the WebSocket is closed.")
    }
}
