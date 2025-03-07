use std::sync::Arc;

use axum::extract::{Path, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::game_factory::actor_client::GameFactoryClient;
use crate::player::actor::PlayerActor;
use crate::websocket::{close, send_error};

#[derive(Deserialize)]
pub struct CreateGameRequest {}

#[derive(Serialize)]
pub struct CreateGameResponse {
    id: String,
}

pub async fn create(State(game_factory): State<Arc<GameFactoryClient>>) -> Response {
    match game_factory.create_game().await {
        Ok(game_id) => (StatusCode::OK, Json(CreateGameResponse { id: game_id })).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn connect_player_to_websocket(
    State(game_factory): State<Arc<GameFactoryClient>>,
    Path((game_id, nickname)): Path<(String, String)>,
    websocket_upgrade: WebSocketUpgrade,
) -> Response {
    websocket_upgrade.on_upgrade(move |mut websocket| async move {
        match game_factory.get_game(&game_id).await {
            Ok(game) => PlayerActor::create(nickname, game, websocket).await,
            Err(error) => {
                send_error(&mut websocket, &error).await;
                close(websocket).await;
            }
        }
    })
}
