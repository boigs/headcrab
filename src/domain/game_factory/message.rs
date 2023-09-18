use tokio::sync::{mpsc::Sender, oneshot::Sender as OneshotSender};

use crate::domain::game::message::GameCommand;

#[derive(Debug)]
pub enum GameFactoryCommand {
    CreateGame {
        response_channel: OneshotSender<GameFactoryResponse>,
    },
    GetGameActor {
        response_channel: OneshotSender<GameFactoryResponse>,
    },
}

#[derive(Debug)]
pub enum GameFactoryResponse {
    GameCreated { game_id: String },
    GameActor { send_channel: Sender<GameCommand> },
}
