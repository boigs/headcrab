use std::{sync::{Arc, Mutex}, ops::ControlFlow};

use axum::{
    extract::{Path, State},
    extract::ws::{WebSocketUpgrade, WebSocket, Message},
    http::StatusCode,
    Json, response::Response,
};
use serde::{Deserialize, Serialize};

use crate::domain::{game_manager::GameManager, player::Player};

#[derive(Deserialize)]
pub struct AddPlayerRequest {
    nickname: String,
}

#[derive(Deserialize)]
pub struct CreateGameRequest { }

#[derive(Serialize)]
pub struct CreateGameResponse {
    id: String,
}

#[derive(Serialize)]
pub struct AddPlayerResponse {
    nickname: String,
}

pub async fn create_game(
    State(manager): State<Arc<Mutex<GameManager>>>,
    Json(_): Json<CreateGameRequest>,
) -> (StatusCode, Json<CreateGameResponse>) {
    let mut manager = manager.lock().unwrap();
    let game_id = manager.create_new_game();

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
    let nickname = manager.add_player(&game_id, &request.nickname);

    (StatusCode::OK, Json(AddPlayerResponse { nickname }))
}

pub async fn remove_player(
    State(game): State<Arc<Mutex<GameManager>>>,
    Path((game_id, nickname)): Path<(String, String)>,
) -> (StatusCode, Json<Option<Player>>) {
    let removed = game.lock().unwrap().remove_player(&game_id, &nickname);
    match removed {
        Some(removed) => (StatusCode::OK, Json(Some(removed))),
        None => (StatusCode::NOT_FOUND, Json(None)),
    }
}

pub async fn websocket_handler(
    // Upgrade the request to a WebSocket connection where the server sends
    // messages of type `ServerMsg` and the clients sends `ClientMsg`
    State(state): State<Arc<Mutex<GameManager>>>,
    Path((game_id, nickname)): Path<(String, String)>,
    websocket: WebSocketUpgrade,
) -> Response {
    websocket.on_upgrade(move |socket| handle_socket(socket, game_id, nickname, state))
}

async fn handle_socket(mut socket: WebSocket, game_id: String, nickname: String, state: Arc<Mutex<GameManager>>) {
    while let Some(message) = socket.recv().await {
        if let Ok(message) = message {
            if process_message(message, &nickname).is_break() {
                return;
            }
        } else {
            println!("client {nickname} abruptly disconnected");
            return;
        }
        if socket.send(Message::Text(format!("Hi {nickname} times!"))).await.is_err() {
            println!("error")
        }
    }
}

fn process_message(message: Message, nickname: &str) -> ControlFlow<(), ()> {
    match message {
        Message::Text(t) => {
            println!(">>> {} sent str: {:?}", nickname, t);
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    ">>> {} sent close with code {} and reason `{}`",
                    nickname, cf.code, cf.reason
                );
            } else {
                println!(">>> {} somehow sent close message without CloseFrame", nickname);
            }
            return ControlFlow::Break(());
        }
        _ => println!("received something else")
    }
    ControlFlow::Continue(())
}