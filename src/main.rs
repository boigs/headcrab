mod controller;
mod lobby;
mod player;

use axum::{
    http::Method,
    routing::{delete, get, post},
    Router,
};
use lobby::Lobby;
use std::sync::Arc;
use std::{net::SocketAddr, sync::Mutex};
use tower_http::cors::{Any, CorsLayer};

use controller::lobby as LobbyController;

#[tokio::main]
async fn main() {
    let lobby = Arc::new(Mutex::new(Lobby::new()));

    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        // allow requests from any origin
        .allow_origin(Any);

    // build our application with a route
    let app = Router::new()
        .route("/lobby", get(LobbyController::get))
        .route("/lobby/players", post(LobbyController::add_player))
        .route("/lobby/players/:id", delete(LobbyController::remove_player))
        .with_state(lobby)
        .layer(cors);

    // run it
    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
