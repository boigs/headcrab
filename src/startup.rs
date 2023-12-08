use crate::actor::game_factory::GameFactoryActor;
use crate::config::Config;
use crate::routes;
use axum::routing::IntoMakeService;
use axum::Router;
use hyper::server::conn::AddrIncoming;
use hyper::Server;
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

pub fn create_web_server(
    config: Config,
    listener: TcpListener,
) -> Result<Server<AddrIncoming, IntoMakeService<Router>>, hyper::Error> {
    let game_factory = Arc::new(GameFactoryActor::spawn(Duration::from_secs(
        config.inactivity_timeout_seconds,
    )));

    let router = routes::create_router(config).with_state(game_factory);

    log::info!(
        "Listening on {}",
        listener
            .local_addr()
            .expect("Can't get the local address of the listener.")
    );
    let server = axum::Server::from_tcp(listener)?.serve(router.into_make_service());
    Ok(server)
}
