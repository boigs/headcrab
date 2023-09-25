use crate::actor::game_factory::GameFactoryCommand;
use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tower_http::cors::CorsLayer;

mod game;
mod health;

pub fn create_router() -> Router<Arc<Sender<GameFactoryCommand>>> {
    Router::new()
        .route("/health", get(health::get))
        .route("/game", post(game::create))
        .route(
            "/game/:game_id/player/:nickname/ws",
            get(game::connect_player_to_websocket),
        )
        .layer(CorsLayer::permissive())
}
