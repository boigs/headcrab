mod controller;
mod lobby;
mod player;

use axum::{
    routing::{delete, get, post},
    Router,
};
use lobby::Lobby;
use std::sync::Arc;
use std::{net::SocketAddr, sync::Mutex};

use controller::lobby as LobbyController;

#[tokio::main]
async fn main() {
    let lobby = Arc::new(Mutex::new(Lobby::new()));

    // build our application with a route
    let app = Router::new()
        .route("/add_player", post(LobbyController::add_player))
        .route("/lobby", get(LobbyController::get))
        .route("/remove_player", delete(LobbyController::remove_player))
        .with_state(lobby);

    // run it
    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
