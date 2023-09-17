use tokio::sync::oneshot::Sender;

#[derive(Debug)]
pub enum Message {
    CreateGame { sender: Sender<Message> },
    GameCreated { game_id: String },
}
