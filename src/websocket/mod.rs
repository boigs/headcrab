pub mod message;

use axum::extract::ws::{Message, WebSocket};

use crate::domain::{game_fsm::GameFsmState, player::Player};
use message::WsMessageOut;

use self::message::{state_to_string, WsMessageIn};

pub async fn send_error_and_close(mut websocket: WebSocket, message: &str) {
    if websocket
        .send(Message::Text(
            serde_json::to_string(&WsMessageOut::Error {
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

pub async fn send_game_state(websocket: &mut WebSocket, state: GameFsmState, players: Vec<Player>) {
    if websocket
        .send(Message::Text(
            serde_json::to_string(&WsMessageOut::GameState {
                state: state_to_string(state),
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

pub fn parse_message(message: &str) -> Result<WsMessageIn, serde_json::Error> {
    serde_json::from_str(message)
}
