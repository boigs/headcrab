use crate::domain::game_factory::GameFactory;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::actor::message::game_factory::GameFactoryCommand::{self, *};
use crate::actor::message::game_factory::GameFactoryResponse::*;

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
            CreateGame { response_channel } => {
                let game_id = game_factory.create_new_game();
                let game_created = GameCreated { game_id };
                response_channel.send(game_created).unwrap();
            }
            GetGameActor {
                game_id,
                response_channel,
            } => {
                let response = game_factory.get_game(&game_id).map_or_else(
                    || GameNotFound,
                    |channel| GameActor {
                        game_channel: channel.clone(),
                    },
                );

                response_channel.send(response).unwrap();
            }
        }
    }
}
