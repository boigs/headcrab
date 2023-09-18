use tokio::sync::mpsc::Sender;

use crate::domain::player::player::Player;

pub enum GameCommand {
    AddPlayer {
        player: Player,
        response_channel: Sender<GameResponse>,
    },
}

pub enum GameResponse {
    PlayerAdded,
    PlayerAlreadyExists,
}
