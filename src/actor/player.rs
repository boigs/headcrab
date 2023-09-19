use axum::extract::ws::WebSocket;
use std::time::Duration;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::actor::message::game::{
    GameCommand::{self, *},
    GameResponse,
};
use crate::domain::player::Player;

pub async fn handler(_socket: WebSocket, nickname: String, game_channel: Sender<GameCommand>) {
    let (tx, mut rx): (Sender<GameResponse>, Receiver<GameResponse>) = mpsc::channel(32);

    game_channel
        .send(AddPlayer {
            player: Player::new(&nickname),
            response_channel: tx,
        })
        .await
        .unwrap();

    match rx.recv().await {
        Some(GameResponse::PlayerAdded) => (),
        Some(GameResponse::_PlayerAlreadyExists) => panic!("Player already exists"),
        _ => panic!("Channel closed or something"),
    }

    loop {
        tokio::time::sleep(Duration::new(5, 0)).await;
    }
}
