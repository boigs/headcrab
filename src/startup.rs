use tokio::net::TcpListener;

use crate::config::Config;
use crate::game_factory::actor::GameFactoryActor;
use crate::routes;
use std::sync::Arc;

pub async fn create_web_server(
    config: Config,
    listener: TcpListener,
) -> Result<(), std::io::Error> {
    let game_factory = Arc::new(GameFactoryActor::spawn(config.game.clone()));

    let router = routes::create_router(config).with_state(game_factory);

    log::info!(
        "Listening on {}",
        listener
            .local_addr()
            .expect("Can't get the local address of the listener.")
    );

    axum::serve(listener, router).await
}
