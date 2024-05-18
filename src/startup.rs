use tokio::net::TcpListener;

use crate::config::Config;
use crate::game_factory::actor::GameFactoryActor;
use crate::routes;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::Arc;

pub async fn create_web_server(
    config: Config,
    listener: TcpListener,
) -> Result<(), std::io::Error> {
    let words = read_words_from_file(&config.words_file);
    log::info!(
        "Words loaded. File: '{}', Words: '{}'.",
        config.words_file,
        words.join(",")
    );
    let game_factory = Arc::new(GameFactoryActor::spawn(config.game.clone(), words));

    let router = routes::create_router(config).with_state(game_factory);

    log::info!(
        "Listening on {}",
        listener
            .local_addr()
            .expect("Can't get the local address of the listener.")
    );

    axum::serve(listener, router).await
}

fn read_words_from_file(file_path: &str) -> Vec<String> {
    let file = File::open(file_path).unwrap_or_else(|error| {
        panic!("Could not load words file. File: '{file_path}', Error: '{error}'.")
    });
    BufReader::new(file)
        .lines()
        .map(|line| {
            line.expect("Could not parse one of the word lines.")
                .trim()
                .to_lowercase()
        })
        .filter(|word| !word.is_empty())
        .collect()
}
