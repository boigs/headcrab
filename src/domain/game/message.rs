use crate::domain::player::Player;
use tokio::sync::mpsc::Sender;

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
