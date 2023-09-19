use crate::domain::{game::Game, player::Player};
use tokio::sync::mpsc::{Receiver, Sender};

pub enum GameCommand {
    AddPlayer {
        player: Player,
        response_channel: Sender<GameResponse>,
    },
}

pub enum GameResponse {
    PlayerAdded,
    _PlayerAlreadyExists,
}

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
