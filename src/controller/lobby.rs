use std::sync::{Arc, Mutex};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{lobby::Lobby, player::Player};

pub async fn get(State(lobby): State<Arc<Mutex<Lobby>>>) -> (StatusCode, Json<Lobby>) {
    let lobby = match lobby.lock() {
        Ok(lobby) => lobby.clone(),
        Err(_) => panic!("no lobby"),
    };

    (StatusCode::OK, Json(lobby))
}

pub async fn add_player(
    State(lobby): State<Arc<Mutex<Lobby>>>,
    Json(request): Json<AddPlayerRequest>,
) -> (StatusCode, Json<Player>) {
    let player = Player::new(&request.name);
    match lobby.lock() {
        Ok(mut lobby) => lobby.add_player(player.clone()),
        Err(_) => panic!("can't add"),
    };
    (StatusCode::OK, Json(player))
}

pub async fn remove_player(
    State(lobby): State<Arc<Mutex<Lobby>>>,
    Path(id): Path<Uuid>,
) -> StatusCode {
    lobby.lock().unwrap().remove_player(&id);
    StatusCode::OK
}

#[derive(Deserialize)]
pub struct AddPlayerRequest {
    name: String,
}
