use axum::extract::ws::WebSocket;
use tokio::sync::mpsc::Sender;

use crate::domain::game::message::GameCommand;

pub async fn handler(mut socket: WebSocket, nickname: String, game_channel: Sender<GameCommand>) {
    while let Some(message) = socket.recv().await {
        if let Ok(message) = message {
            if process_message(message, &nickname).is_break() {
                return;
            }
        } else {
            println!("client {nickname} abruptly disconnected");
            return;
        }
        if socket
            .send(Message::Text(format!("Hi {nickname} times!")))
            .await
            .is_err()
        {
            println!("error")
        }
    }
}
