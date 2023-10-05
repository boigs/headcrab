use std::fmt::{Display, Formatter};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::oneshot::{self, Receiver as OneshotReceiver, Sender as OneshotSender};

use crate::actor::game::GameClient;
use crate::domain::game_factory::GameFactory;

pub struct GameFactoryClient {
    game_factory_tx: Sender<GameFactoryCommand>,
}

impl GameFactoryClient {
    pub async fn create_game(&self) -> Result<String, String> {
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

        match rx.await {
            Ok(GameFactoryResponse::GameCreated { game_id }) => Ok(game_id),
            Ok(unexpected_response) => {
                println!(
                    "ERROR: Received an unexpected GameFactoryResponse. Response: {unexpected_response}.",
                );
                Err(format!(
                    "ERROR: Received an unexpected GameFactoryResponse. Error {unexpected_response}."
                ))
            }
            Err(error) => {
                println!("ERROR: The Game channel is closed. Error: {error}.");
                Err(format!(
                    "ERROR: The Game channel is closed. Error: {error}."
                ))
            }
        }
    }

    pub async fn get_game(&self, game_id: &str) -> Result<GameClient, String> {
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
            Ok(GameFactoryResponse::GameActor { game }) => Ok(game),
            Ok(GameFactoryResponse::GameNotFound) => Err("Game not found.".to_string()),
            Ok(unexpected_response) => {
                println!(
                    "ERROR: Received an unexpected GameFactoryResponse. Response: {unexpected_response}.",
                );
                Err(format!(
                    "ERROR: Received an unexpected GameFactoryResponse. Error {unexpected_response}."
                ))
            }
            Err(error) => {
                println!("ERROR: The Game channel is closed. Error: {error}.");
                Err(format!(
                    "ERROR: The Game channel is closed. Error: {error}."
                ))
            }
        }
    }
}

pub struct GameFactoryActor {
    game_factory: GameFactory,
    game_factory_rx: Receiver<GameFactoryCommand>,
}

impl GameFactoryActor {
    /// Runs the GameFactory Actor in background and returns a Client to communicate with it
    pub fn spawn() -> GameFactoryClient {
        let game_factory = GameFactory::new();
        let (game_factory_tx, game_factory_rx): (
            Sender<GameFactoryCommand>,
            Receiver<GameFactoryCommand>,
        ) = mpsc::channel(512);

        tokio::spawn(
            GameFactoryActor {
                game_factory,
                game_factory_rx,
            }
            .start(),
        );

        GameFactoryClient { game_factory_tx }
    }

    async fn start(mut self) {
        while let Some(message) = self.game_factory_rx.recv().await {
            match message {
                GameFactoryCommand::CreateGame { response_channel } => {
                    let game_id = self.game_factory.create_new_game();
                    let game_created = GameFactoryResponse::GameCreated { game_id };
                    response_channel.send(game_created).unwrap();
                }
                GameFactoryCommand::GetGameActor {
                    game_id,
                    response_channel,
                } => {
                    let response = self.game_factory.get_game(&game_id).map_or_else(
                        || GameFactoryResponse::GameNotFound,
                        |game| GameFactoryResponse::GameActor { game: game.clone() },
                    );
                    response_channel.send(response).unwrap();
                }
            }
        }
    }
}

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
    GameActor { game: GameClient },
    GameNotFound,
}

impl Display for GameFactoryResponse {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}",
            match self {
                GameFactoryResponse::GameCreated { game_id } =>
                    format!("GameCreated(game_id: {game_id})"),
                GameFactoryResponse::GameActor { game: _ } => "GameActor".to_string(),
                GameFactoryResponse::GameNotFound => "GameNotFound".to_string(),
            }
        )
    }
}
