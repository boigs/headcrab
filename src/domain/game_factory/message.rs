use tokio::sync::oneshot::Sender;

#[derive(Debug)]
pub enum GameFactoryCommand {
    CreateGame {
        response_channel: Sender<GameFactoryResponse>,
    },
    AddPlayer {
        nickname: String,
    },
}

#[derive(Debug)]
pub enum GameFactoryResponse {
    GameCreated { game_id: String },
    PlayerAdded,
}
