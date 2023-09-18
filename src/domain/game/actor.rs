use crate::domain::game::Game;
use tokio::sync::mpsc::Receiver;

use super::message::{GameCommand, GameResponse};

pub async fn handler(mut rx: Receiver<GameCommand>) {
    let mut game = Game::new();

    while let Some(command) = rx.recv().await {
        match command {
            GameCommand::AddPlayer {
                player,
                response_channel,
            } => {
                game.add_player(player);

                response_channel
                    .send(GameResponse::PlayerAdded)
                    .await
                    .unwrap();
            }
        }
    }
}
