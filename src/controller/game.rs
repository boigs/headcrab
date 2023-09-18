use std::sync::Arc;

use axum::extract::ws::WebSocket;
use axum::extract::{Path, WebSocketUpgrade};
use axum::response::{IntoResponse, Response};
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::{self, Receiver as OneshotReceiver, Sender as OneshotSender};

use crate::domain::game::message::GameCommand;
use crate::domain::game_factory::message::{
    GameFactoryCommand::{self, *},
    GameFactoryResponse::{self, *},
};

#[derive(Deserialize)]
pub struct AddPlayerRequest {
    nickname: String,
}

#[derive(Deserialize)]
pub struct CreateGameRequest {}

#[derive(Serialize)]
pub struct CreateGameResponse {
    id: String,
}

#[derive(Serialize)]
pub struct AddPlayerResponse {
    nickname: String,
}

pub async fn create_game(
    State(sender): State<Arc<Sender<GameFactoryCommand>>>,
) -> (StatusCode, Json<CreateGameResponse>) {
    let (tx, rx): (
        OneshotSender<GameFactoryResponse>,
        OneshotReceiver<GameFactoryResponse>,
    ) = oneshot::channel();
    sender
        .send(CreateGame {
            response_channel: tx,
        })
        .await
        .unwrap();
    match rx.await.unwrap() {
        GameCreated { game_id } => (StatusCode::OK, Json(CreateGameResponse { id: game_id })),
        _ => panic!("error at receiving game created"),
    }
}

pub async fn player_connecting_ws(
    // Upgrade the request to a WebSocket connection where the server sends
    // messages of type `ServerMsg` and the clients sends `ClientMsg`
    State(sender): State<Arc<Sender<GameFactoryCommand>>>,
    Path((game_id, nickname)): Path<(String, String)>,
    websocket: WebSocketUpgrade,
) -> Response {
    let (tx, rx): (
        OneshotSender<GameFactoryResponse>,
        OneshotReceiver<GameFactoryResponse>,
    ) = oneshot::channel();

    sender.send(GetGameActor {
        game_id,
        response_channel: tx,
    });

    match rx.await {
        Ok(GameActor { game_channel }) => {
            websocket.on_upgrade(move |socket| handle_socket(socket, nickname, game_channel))
        }
        _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn handle_socket(mut socket: WebSocket, nickname: String, game_channel: Sender<GameCommand>) {
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
