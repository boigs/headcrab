use std::sync::{Arc, Mutex};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{game::Game, player::Player};

pub async fn get_players(
    State(game): State<Arc<Mutex<Game>>>,
) -> (StatusCode, Json<Vec<Player>>) {
    let game = match game.lock() {
        Ok(game) => game.clone(),
        Err(_) => panic!("no game"),
    };

    (StatusCode::OK, Json(game.players().to_vec()))
}

pub async fn add_player(
    State(game): State<Arc<Mutex<Game>>>,
    Json(request): Json<AddPlayerRequest>,
) -> (StatusCode, Json<Player>) {
    let player = Player::new(&request.name);
    match game.lock() {
        Ok(mut game) => game.add_player(player.clone()),
        Err(_) => panic!("can't add"),
    };
    (StatusCode::OK, Json(player))
}

pub async fn remove_player(
    State(game): State<Arc<Mutex<Game>>>,
    Path(id): Path<Uuid>,
) -> (StatusCode, Json<Option<Player>>) {
    let removed = game.lock().unwrap().remove_player(&id);
    match removed {
        Some(removed) => (StatusCode::OK, Json(Some(removed))),
        None => (StatusCode::NOT_FOUND, Json(None)),
    }
}

#[derive(Deserialize)]
pub struct AddPlayerRequest {
    name: String,
}
