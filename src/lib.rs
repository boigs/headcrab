use axum::Server;

mod actor;
mod controller;
mod domain;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::IntoMakeService;
use axum::{
    routing::{get, post},
    Router,
};
use hyper::server::conn::AddrIncoming;
use std::net::TcpListener;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

pub fn create_web_server(
    listener: TcpListener,
) -> Result<Server<AddrIncoming, IntoMakeService<Router>>, hyper::Error> {
    let game_factory_channel = actor::game_factory::start();
    let game_factory_channel = Arc::new(game_factory_channel);

    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/game", post(controller::routes::create_game))
        .route(
            "/game/:game_id/player/:nickname/ws",
            get(controller::routes::player_connecting_ws),
        )
        .with_state(game_factory_channel)
        .layer(CorsLayer::permissive());

    println!("listening on {}", listener.local_addr().unwrap());
    let server = axum::Server::from_tcp(listener)?.serve(app.into_make_service());
    Ok(server)
}

async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}
