use std::fmt::{Display, Formatter};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::oneshot::{self, Receiver as OneshotReceiver, Sender as OneshotSender};

use crate::actor::game::GameCommand;
use crate::domain::game_factory::GameFactory;

#[derive(Debug)]
enum GameFactoryCommand {
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
enum GameFactoryResponse {
    GameCreated { game_id: String },
    GameActor { game_channel: Sender<GameCommand> },
    GameNotFound,
}

pub struct GameFactoryActor {
    game_factory_tx: Sender<GameFactoryCommand>,
}

impl GameFactoryActor {
    /// Runs the GameFactory actor in background and returns an object to communicate with it
    pub fn new() -> Self {
        let (sender, receiver): (Sender<GameFactoryCommand>, Receiver<GameFactoryCommand>) =
            mpsc::channel(512);

        tokio::spawn(handler(receiver));

        GameFactoryActor {
            game_factory_tx: sender,
        }
    }

    pub async fn create_game(&self) -> Result<String, &'static str> {
        let (tx, rx): (
            OneshotSender<GameFactoryResponse>,
            OneshotReceiver<GameFactoryResponse>,
        ) = oneshot::channel();

        self.game_factory_tx
            .send(GameFactoryCommand::CreateGame {
                response_channel: tx,
            })
            .await
            .unwrap();

        match rx.await.unwrap() {
            GameFactoryResponse::GameCreated { game_id } => Ok(game_id),
            _ => Err("Could not create game, unexpected response from GameFactory"),
        }
    }

    pub async fn get_game(&self, game_id: &str) -> Result<Option<Sender<GameCommand>>, String> {
        let (tx, rx): (
            OneshotSender<GameFactoryResponse>,
            OneshotReceiver<GameFactoryResponse>,
        ) = oneshot::channel();

        if self
            .game_factory_tx
            .send(GameFactoryCommand::GetGameActor {
                game_id: game_id.to_string(),
                response_channel: tx,
            })
            .await
            .is_err()
        {
            return Err("ERROR: The GameFactory channel is closed.".to_string());
        }

        match rx.await {
            Ok(GameFactoryResponse::GameActor { game_channel }) => Ok(Some(game_channel)),
            Ok(GameFactoryResponse::GameNotFound) => Err("Game not found.".to_string()),
            Err(error) => {
                println!("ERROR: The Game channel is closed. Error: {error}.");
                Err(format!(
                    "ERROR: The Game channel is closed. Error: {error}."
                ))
            }
            Ok(unexpected_response) => {
                println!(
                    "ERROR: Received an unexpected GameFactoryResponse. Response: {unexpected_response}.",
                );
                Err(format!(
                    "ERROR: Received an unexpected GameFactoryResponse. Error {unexpected_response}."
                ))
            }
        }
    }
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

impl Display for GameFactoryResponse {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}",
            match self {
                GameFactoryResponse::GameCreated { game_id } =>
                    format!("GameCreated(game_id: {game_id})"),
                GameFactoryResponse::GameActor { game_channel: _ } => "GameActor".to_string(),
                GameFactoryResponse::GameNotFound => "GameNotFound".to_string(),
            }
        )
    }
}
