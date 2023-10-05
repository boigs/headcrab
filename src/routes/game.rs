use std::sync::Arc;

use crate::actor::game::GameCommand;
use crate::actor::game_factory::{GameFactoryCommand, GameFactoryResponse};
use crate::actor::player::PlayerActor;
use crate::websocket::send_error_and_close;
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
    websocket.on_upgrade(move |socket| async move {
        match get_game(&game_id, sender).await {
            Ok(game_tx) => PlayerActor::create(nickname, game_tx, socket).await,
            Err(error) => send_error_and_close(socket, &error).await,
        }
    })
}

async fn get_game(
    game_id: &str,
    sender: Arc<Sender<GameFactoryCommand>>,
) -> Result<Sender<GameCommand>, String> {
    let (tx, rx): (
        OneshotSender<GameFactoryResponse>,
        OneshotReceiver<GameFactoryResponse>,
    ) = oneshot::channel();

    if sender
        .send(GameFactoryCommand::GetGameActor {
            game_id: game_id.to_string(),
            response_channel: tx,
        })
        .await
        .is_err()
    {
        return Err("ERROR: The GameFactory channel is closed.".to_string());
    }

    match rx.await {
        Ok(GameFactoryResponse::GameActor { game_channel }) => Ok(game_channel),
        Ok(GameFactoryResponse::GameNotFound) => Err("Game not found.".to_string()),
        Err(error) => {
            println!("ERROR: The Game channel is closed. Error: {error}.");
            Err(format!(
                "ERROR: The Game channel is closed. Error: {error}."
            ))
        }
        Ok(unexpected_response) => {
            println!(
                "ERROR: Received an unexpected GameFactoryResponse. Response: {unexpected_response}.",
            );
            Err(format!(
                "ERROR: Received an unexpected GameFactoryResponse. Error {unexpected_response}."
            ))
        }
    }
}
