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

pub async fn create(
    State(sender): State<Arc<Sender<GameFactoryCommand>>>,
) -> (StatusCode, Json<CreateGameResponse>) {
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
            (StatusCode::OK, Json(CreateGameResponse { id: game_id }))
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
    let (tx, rx): (
        OneshotSender<GameFactoryResponse>,
        OneshotReceiver<GameFactoryResponse>,
    ) = oneshot::channel();

    sender
        .send(GameFactoryCommand::GetGameActor {
            game_id,
            response_channel: tx,
        })
        .await
        .unwrap();

    match rx.await {
        Ok(GameFactoryResponse::GameActor { game_channel }) => {
            match PlayerActor::new(nickname, game_channel).await {
                Ok(player_actor) => {
                    websocket.on_upgrade(move |socket| player_actor.handler(socket))
                }
                Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
            }
        }
        Ok(GameFactoryResponse::GameNotFound) => {
            (StatusCode::NOT_FOUND, "Game not found").into_response()
        }
        Err(error) => {
            println!("ERROR: The Game channel is closed. Error: {error}");
            StatusCode::INTERNAL_SERVER_ERROR .into_response()
        }
        Ok(unexpected_response) => {
            println!(
                "ERROR: Received an unexpected GameFactoryResponse, error {:?}",
                unexpected_response
            );
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
