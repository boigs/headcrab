use axum::Server;

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
use tokio::sync::mpsc::{self, Receiver, Sender};
use tower_http::cors::CorsLayer;

use crate::controller::game::create_game;
use crate::domain::game_manager;

use crate::domain::message::Message;

pub fn create_web_server(
    listener: TcpListener,
) -> Result<Server<AddrIncoming, IntoMakeService<Router>>, hyper::Error> {
    let (sender, receiver): (Sender<Message>, Receiver<Message>) = mpsc::channel(512);

    tokio::spawn(game_manager::actor(receiver));

    let sender = Arc::new(sender);

    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/game", post(create_game))
        .with_state(sender)
        .layer(CorsLayer::permissive());

    println!("listening on {}", listener.local_addr().unwrap());
    let server = axum::Server::from_tcp(listener)?.serve(app.into_make_service());
    Ok(server)
}

async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}
