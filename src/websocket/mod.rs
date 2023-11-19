pub mod message;

use axum::extract::ws::{Message, WebSocket};

use crate::domain::error::Error;
use crate::domain::{game_fsm::GameFsmState, player::Player};
use message::WsMessageOut;

use self::message::{state_to_string, WsMessageIn};

pub async fn send_error_and_close(mut websocket: WebSocket, error: Error) {
    if websocket
        .send(Message::Text(
            // TODO: send a proper error
            serde_json::to_string(&error_to_ws_error(error.clone())).unwrap(),
        ))
        .await
        .is_err()
    {
        log::error!("Sent Error '{error}' to the browser but the WebSocket is closed.")
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

pub async fn send_chat_message(websocket: &mut WebSocket, sender: &str, content: &str) {
    if websocket
        .send(Message::Text(
            serde_json::to_string(&WsMessageOut::ChatMessage {
                sender: sender.to_string(),
                content: content.to_string(),
            })
            .unwrap(),
        ))
        .await
        .is_err()
    {
        log::error!("Sent ChatMessage to the browser but the WebSocket is closed.")
    }
}

pub fn parse_message(message: &str) -> Result<WsMessageIn, serde_json::Error> {
    serde_json::from_str(message)
}

fn error_to_ws_error(error: Error) -> WsMessageOut {
    match error {
        Error::GameDoesNotExist(_) => WsMessageOut::Error {
            r#type: "GAME_DOES_NOT_EXIST".to_string(),
            title: "The game does not exist".to_string(),
            detail: error.to_string(),
        },
        Error::PlayerAlreadyExists(_) => WsMessageOut::Error {
            r#type: "PLAYER_ALREADY_EXISTS".to_string(),
            title: "The player already exists".to_string(),
            detail: error.to_string(),
        },
        Error::Internal(_) => WsMessageOut::Error {
            r#type: "INTERNAL_SERVER".to_string(),
            title: "Internal Server error".to_string(),
            detail: error.to_string(),
        },
    }
}
