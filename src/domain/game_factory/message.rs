use tokio::sync::{mpsc::Sender, oneshot::Sender as OneshotSender};

use crate::domain::game::message::GameCommand;

#[derive(Debug)]
pub enum GameFactoryCommand {
    CreateGame {
        response_channel: OneshotSender<GameFactoryResponse>,
    },
    GetGameActor {
        game_id: String,
        response_channel: OneshotSender<GameFactoryResponse>,
    },
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum GameFactoryResponse {
    GameCreated { game_id: String },
    GameActor { game_channel: Sender<GameCommand> },
    GameNotFound,
}
