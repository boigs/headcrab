mod controller;
mod game;
mod player;

use axum::{
    routing::{delete, get, post},
    Router,
};
use game::Game;
use std::sync::Arc;
use std::{net::SocketAddr, sync::Mutex};
use tower_http::cors::CorsLayer;

use controller::game as GameController;

#[tokio::main]
async fn main() {
    let game = Arc::new(Mutex::new(Game::new()));

    // build our application with a route
    let app = Router::new()
        .route("/game/players", get(GameController::get_players))
        .route("/game/players", post(GameController::add_player))
        .route("/game/players/:id", delete(GameController::remove_player))
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
