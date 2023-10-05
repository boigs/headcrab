use std::sync::Arc;

use crate::actor::game_factory::{GameFactoryCommand, GameFactoryResponse};
use crate::actor::player::PlayerActor;
use axum::extract::{Path, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::{self, Receiver as OneshotReceiver, Sender as OneshotSender};

#[derive(Deserialize)]
pub struct CreateGameRequest {}

#[derive(Serialize)]
pub struct CreateGameResponse {
    id: String,
}

pub async fn create(State(sender): State<Arc<Sender<GameFactoryCommand>>>) -> Response {
    let (tx, rx): (
        OneshotSender<GameFactoryResponse>,
        OneshotReceiver<GameFactoryResponse>,
    ) = oneshot::channel();
    sender
        .send(GameFactoryCommand::CreateGame {
            response_channel: tx,
        })
        .await
        .unwrap();
    match rx.await.unwrap() {
        GameFactoryResponse::GameCreated { game_id } => {
            (StatusCode::OK, Json(CreateGameResponse { id: game_id })).into_response()
        }
        _ => panic!("error at receiving game created"),
    }
}

pub async fn connect_player_to_websocket(
    // Upgrade the request to a WebSocket connection where the server sends
    // messages of type `ServerMsg` and the clients sends `ClientMsg`
    State(sender): State<Arc<Sender<GameFactoryCommand>>>,
    Path((game_id, nickname)): Path<(String, String)>,
    websocket: WebSocketUpgrade,
) -> Response {
    websocket.on_upgrade(move |socket| PlayerActor::create(nickname, game_id, sender, socket))
}
