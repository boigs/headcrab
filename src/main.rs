use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use lobby::Lobby;
use player::Player;
use serde::Deserialize;
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

async fn get_lobby(State(lobby): State<Arc<Mutex<Lobby>>>) -> (StatusCode, Json<Lobby>) {
    let lobby = match lobby.lock() {
        Ok(lobby) => lobby.clone(),
        Err(_) => panic!("no lobby"),
    };

    (StatusCode::OK, Json(lobby))
}

async fn add_player(
    State(lobby): State<Arc<Mutex<Lobby>>>,
    Json(input): Json<Input>,
) -> (StatusCode, Json<Player>) {
    let player = Player::new(&input.name);
    match lobby.lock() {
        Ok(mut lobby) => lobby.add_player(player.clone()),
        Err(_) => panic!("can't add"),
    };
    (StatusCode::OK, Json(player))
}

#[derive(Deserialize)]
struct Input {
    name: String,
}
