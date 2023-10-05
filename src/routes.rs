use crate::actor::game_factory::GameFactoryActor;
use crate::config::Config;
use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

mod game;
mod health;

pub fn create_router(config: Config) -> Router<Arc<GameFactoryActor>> {
    Router::new()
        .route("/health", get(health::get))
        .route("/game", post(game::create))
        .route(
            "/game/:game_id/player/:nickname/ws",
            get(game::connect_player_to_websocket),
        )
        .layer(if config.allow_cors {
            println!("INFO: CorsLayer Permissive");
            CorsLayer::permissive()
        } else {
            CorsLayer::default()
        })
}
