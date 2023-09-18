use tokio::sync::mpsc::Receiver;

use super::message::GameCommand;

pub async fn actor_handler(mut rx: Receiver<GameCommand>) {
    while let Some(command) = rx.recv().await {
        match command {
            GameCommand::AddPlayer { nickname } => todo!(),
        }
    }
}
