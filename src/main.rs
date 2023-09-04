mod controller;
mod domain;

use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use std::{net::SocketAddr, sync::Mutex};
use tower_http::cors::CorsLayer;

use controller::game as GameController;

use crate::domain::game_manager::GameManager;

#[tokio::main]
async fn main() {
    let game = Arc::new(Mutex::new(GameManager::new()));

    // build our application with a route
    let app = Router::new()
    .route("/game", post(GameController::create_game))
    .route("/game/:game_id/players", get(GameController::get_players))
    .route("/game/:game_id/players", post(GameController::add_player))
    .route("/game/:game_id/players/:id", delete(GameController::remove_player))
    .with_state(game)
    .layer(CorsLayer::permissive());

    // run it
    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
