use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::oneshot::Sender as OneshotSender;

use crate::actor::game::GameCommand;
use crate::domain::game_factory::GameFactory;

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

/// Runs the GameFactory actor and returns the sender channel to communicate with it.
pub fn start() -> Sender<GameFactoryCommand> {
    let (sender, receiver): (Sender<GameFactoryCommand>, Receiver<GameFactoryCommand>) =
        mpsc::channel(512);

    tokio::spawn(handler(receiver));

    sender
}

async fn handler(mut rx: Receiver<GameFactoryCommand>) {
    let mut game_factory = GameFactory::new();

    while let Some(message) = rx.recv().await {
        match message {
            GameFactoryCommand::CreateGame { response_channel } => {
                let game_id = game_factory.create_new_game();
                let game_created = GameFactoryResponse::GameCreated { game_id };
                response_channel.send(game_created).unwrap();
            }
            GameFactoryCommand::GetGameActor {
                game_id,
                response_channel,
            } => {
                let response = game_factory.get_game(&game_id).map_or_else(
                    || GameFactoryResponse::GameNotFound,
                    |channel| GameFactoryResponse::GameActor {
                        game_channel: channel.clone(),
                    },
                );

                response_channel.send(response).unwrap();
            }
        }
    }
}
