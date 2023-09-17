use tokio::sync::oneshot::Sender;

#[derive(Debug)]
pub enum GameManagerCommand {
    CreateGame {
        response_channel: Sender<GameManagerResponse>,
    },
}

#[derive(Debug)]
pub enum GameManagerResponse {
    GameCreated { game_id: String },
}
