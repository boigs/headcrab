use crate::actor::game_factory::GameFactoryCommand;
use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tower_http::cors::CorsLayer;

mod game;
mod health;

pub fn create_router() -> Router<Arc<Sender<GameFactoryCommand>>> {
    let is_dev_environment = match std::env::var("ENVIRONMENT").as_ref().map(String::as_ref) {
        Ok("dev") => true,
        _ => false,
    };

    Router::new()
        .route("/health", get(health::get))
        .route("/game", post(game::create))
        .route(
            "/game/:game_id/player/:nickname/ws",
            get(game::connect_player_to_websocket),
        )
        .layer(if is_dev_environment {
            CorsLayer::permissive()
        } else {
            CorsLayer::default()
        })
}
