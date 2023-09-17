use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;

use crate::domain::message::Message as DomainMessage;
use crate::domain::message::Message::{CreateGame, GameCreated};

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
    State(sender): State<Arc<Sender<DomainMessage>>>,
) -> (StatusCode, Json<CreateGameResponse>) {
    let (tx, rx): (
        oneshot::Sender<DomainMessage>,
        oneshot::Receiver<DomainMessage>,
    ) = oneshot::channel();
    sender.send(CreateGame { sender: tx }).await.unwrap();
    match rx.await.unwrap() {
        GameCreated { game_id } => (StatusCode::OK, Json(CreateGameResponse { id: game_id })),
        _ => panic!("error at receiving game created"),
    }
}
