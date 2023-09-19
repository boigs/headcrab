use std::sync::Arc;

use axum::extract::{Path, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::{self, Receiver as OneshotReceiver, Sender as OneshotSender};
use tower_http::cors::CorsLayer;

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

pub fn create() -> Router<Arc<Sender<GameFactoryCommand>>> {
    Router::new()
        .route("/health", get(health))
        .route("/game", post(create_game))
        .route(
            "/game/:game_id/player/:nickname/ws",
            get(player_connecting_ws),
        )
        .layer(CorsLayer::permissive())
}

async fn health() -> (StatusCode, String) {
    (StatusCode::OK, "healthy".to_string())
}

async fn create_game(
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

async fn player_connecting_ws(
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
