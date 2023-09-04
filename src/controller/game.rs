use std::sync::{Arc, Mutex};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{game_manager::GameManager, player::Player};

#[derive(Deserialize)]
pub struct AddPlayerRequest {
    nickname: String,
}

#[derive(Deserialize)]
pub struct CreateGameRequest {
    nickname: String,
}

#[derive(Serialize)]
pub struct CreateGameResponse {
    id: String,
}

#[derive(Serialize)]
pub struct AddPlayerResponse {
    id: Uuid,
}

pub async fn create_game(
    State(manager): State<Arc<Mutex<GameManager>>>,
    Json(request): Json<CreateGameRequest>,
) -> (StatusCode, Json<CreateGameResponse>) {
    let host_nickname = request.nickname;
    let mut manager = manager.lock().unwrap();
    let game_id = manager.create_new_game(&host_nickname);

    (StatusCode::OK, Json(CreateGameResponse { id: game_id }))
}

pub async fn get_players(
    State(manager): State<Arc<Mutex<GameManager>>>,
    Path(game_id): Path<String>,
) -> (StatusCode, Json<Vec<Player>>) {
    let manager = manager.lock().unwrap();
    let players = manager.get_game(&game_id).unwrap().players();

    (StatusCode::OK, Json(players.to_vec()))
}

pub async fn add_player(
    State(manager): State<Arc<Mutex<GameManager>>>,
    Path(game_id): Path<String>,
    Json(request): Json<AddPlayerRequest>,
) -> (StatusCode, Json<AddPlayerResponse>) {
    let mut manager = manager.lock().unwrap();
    let player_id = manager.add_player(&game_id, &request.nickname);

    (StatusCode::OK, Json(AddPlayerResponse { id: player_id }))
}

pub async fn remove_player(
    State(game): State<Arc<Mutex<GameManager>>>,
    Path((game_id, player_id)): Path<(String, Uuid)>,
) -> (StatusCode, Json<Option<Player>>) {
    let removed = game.lock().unwrap().remove_player(&game_id, &player_id);
    match removed {
        Some(removed) => (StatusCode::OK, Json(Some(removed))),
        None => (StatusCode::NOT_FOUND, Json(None)),
    }
}
