use axum::extract::ws::WebSocket;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::domain::game::message::{
    GameCommand::{self, *},
    GameResponse,
};

use super::player::Player;

pub async fn handler(_socket: WebSocket, nickname: String, game_channel: Sender<GameCommand>) {
    let (tx, mut rx): (Sender<GameResponse>, Receiver<GameResponse>) = mpsc::channel(32);

    game_channel
        .send(AddPlayer {
            player: Player { nickname },
            response_channel: tx,
        })
        .await
        .unwrap();

    match rx.recv().await {
        Some(GameResponse::PlayerAdded) => (),
        Some(GameResponse::PlayerAlreadyExists) => panic!("Player already exists"),
        _ => panic!("Channel closed or something"),
    }

    loop {}
}
