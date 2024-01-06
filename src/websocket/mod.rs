pub mod message;

use axum::extract::ws::{Message, WebSocket};
use serde::Serialize;

use crate::error::Error;
use crate::websocket::message::WsMessageOut;

use self::message::WsMessageIn;

pub async fn send_error(websocket: &mut WebSocket, error: Error) {
    // We are closing the websocket, ignore if there's any error sending the last message
    let _ = send_message(websocket, &error_to_ws_error(error.clone())).await;
}

pub async fn close(websocket: WebSocket) {
    // The websocket might already be closed, if so, ignore the error
    let _ = websocket.close().await;
}

pub fn parse_message(message: &str) -> Result<WsMessageIn, Error> {
    serde_json::from_str(message)
        .map_err(|error| Error::UnprocessableMessage(message.to_string(), error.to_string()))
}

pub async fn send_message<T>(websocket: &mut WebSocket, value: &T) -> Result<(), Error>
where
    T: ?Sized + Serialize,
{
    let message = serde_json::to_string(value).map_err(|error| {
        Error::log_and_create_internal(&format!(
            "Could not serialize the message. Error: '{error}'."
        ))
    })?;
    send_message_string(websocket, &message).await
}

pub async fn send_message_string(websocket: &mut WebSocket, value: &str) -> Result<(), Error> {
    websocket
        .send(Message::Text(value.to_string()))
        .await
        .map_err(|error| Error::WebsocketClosed(error.to_string()))
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
        Error::WebsocketClosed(_) => WsMessageOut::Error {
            r#type: "WEBSOCKET_CLOSED".to_string(),
            title: "The player websocket is closed".to_string(),
            detail: error.to_string(),
        },
        Error::CommandNotAllowed(_, _) => WsMessageOut::Error {
            r#type: "COMMAND_NOT_ALLOWED".to_string(),
            title: "The player cannot execute this command".to_string(),
            detail: error.to_string(),
        },
        Error::UnprocessableMessage(_, _) => WsMessageOut::Error {
            r#type: "UNPROCESSABLE_WEBSOCKET_MESSAGE".to_string(),
            title: "Received an invalid message".to_string(),
            detail: error.to_string(),
        },
    }
}
