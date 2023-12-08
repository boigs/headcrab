use crate::actor::game_factory::client::GameFactoryClient;
use crate::config::Config;
use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

mod game;
mod health;
mod metrics;

pub fn create_router(config: Config) -> Router<Arc<GameFactoryClient>> {
    Router::new()
        .route("/health", get(health::get))
        .route("/metrics", get(metrics::metrics_handler))
        .route("/game", post(game::create))
        .route(
            "/game/:game_id/player/:nickname/ws",
            get(game::connect_player_to_websocket),
        )
        .layer(if config.allow_cors {
            log::info!("CorsLayer Permissive");
            CorsLayer::permissive()
        } else {
            CorsLayer::default()
        })
}
