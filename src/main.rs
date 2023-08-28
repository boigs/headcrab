use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use lobby::Lobby;
use player::Player;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{net::SocketAddr, sync::Mutex};

mod lobby;
mod player;

#[tokio::main]
async fn main() {
    let lobby = Arc::new(Mutex::new(Lobby::new()));

    // build our application with a route
    let app = Router::new()
        .route("/add_player", post(add_player))
        .route("/lobby", get(get_lobby))
        .with_state(lobby);

    // run it
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_lobby(State(lobby): State<Arc<Mutex<Lobby>>>) -> (StatusCode, Json<String>) {
    let lobby = match lobby.lock() {
        Ok(lobby) => format!("{:?}", lobby),
        Err(_) => panic!("no lobby"),
    };

    (StatusCode::OK, Json(lobby))
}

async fn add_player(
    State(lobby): State<Arc<Mutex<Lobby>>>,
    Json(input): Json<Input>,
) -> (StatusCode, Json<String>) {
    let player = Player::new(&input.name);
    match lobby.lock() {
        Ok(mut lobby) => (*lobby).add_player(player),
        Err(_) => (),
    };
    (StatusCode::OK, Json("player".to_string()))
}

#[derive(Serialize)]
struct Person {
    id: u64,
    name: String,
}

#[derive(Deserialize)]
struct Input {
    name: String,
}
