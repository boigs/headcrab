use axum::Server;

mod controller;
mod domain;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::IntoMakeService;
use axum::{
    routing::{delete, get, post},
    Router,
};
use hyper::server::conn::AddrIncoming;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;

use controller::game as GameController;

use crate::domain::game_manager::GameManager;

pub fn create_web_server(
    listener: TcpListener,
) -> Result<Server<AddrIncoming, IntoMakeService<Router>>, hyper::Error> {
    let game = Arc::new(Mutex::new(GameManager::new()));

    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/game", post(GameController::create_game))
        .route("/game/:game_id/players", get(GameController::get_players))
        .route("/game/:game_id/player", post(GameController::add_player))
        .route(
            "/game/:game_id/player/:nickname",
            delete(GameController::remove_player),
        )
        .route(
            "/ws/game/:game_id/player/:nickname",
            get(GameController::websocket_handler),
        )
        .with_state(game)
        .layer(CorsLayer::permissive());

    println!("listening on {}", listener.local_addr().unwrap());
    let server = axum::Server::from_tcp(listener)?.serve(app.into_make_service());
    Ok(server)
}

async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}
