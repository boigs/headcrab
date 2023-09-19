use std::sync::Arc;

use axum::extract::{Path, WebSocketUpgrade};
use axum::response::{IntoResponse, Response};
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::{self, Receiver as OneshotReceiver, Sender as OneshotSender};

use crate::actor;
use crate::actor::game_factory::{
    GameFactoryCommand::{self, *},
    GameFactoryResponse::{self, *},
};

#[derive(Deserialize)]
pub struct CreateGameRequest {}

#[derive(Serialize)]
pub struct CreateGameResponse {
    id: String,
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

    sender
        .send(GetGameActor {
            game_id,
            response_channel: tx,
        })
        .await
        .unwrap();

    match rx.await {
        Ok(GameActor { game_channel }) => websocket
            .on_upgrade(move |socket| actor::player::handler(socket, nickname, game_channel)),

        Ok(GameNotFound) => StatusCode::NOT_FOUND.into_response(),
        _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
