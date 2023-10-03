use crate::config::Config;
use crate::{actor, routes};
use axum::routing::IntoMakeService;
use axum::Router;
use hyper::server::conn::AddrIncoming;
use hyper::Server;
use std::net::TcpListener;
use std::sync::Arc;

pub fn create_web_server(
    listener: TcpListener,
) -> Result<Server<AddrIncoming, IntoMakeService<Router>>, hyper::Error> {
    let config = Config::get().expect("Unable to get the Config.");
    let game_factory_channel = actor::game_factory::start();
    let game_factory_channel = Arc::new(game_factory_channel);

    let router = routes::create_router(config).with_state(game_factory_channel);

    println!("INFO: Listening on {}", listener.local_addr().unwrap());
    let server = axum::Server::from_tcp(listener)?.serve(router.into_make_service());
    Ok(server)
}
