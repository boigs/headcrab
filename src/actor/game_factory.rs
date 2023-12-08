pub mod client;

use std::fmt::{Display, Formatter};
use std::time::Duration;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::oneshot::Sender as OneshotSender;

use crate::actor::game::client::GameClient;
use crate::domain::error::Error;
use crate::domain::game_factory::GameFactory;

use self::client::GameFactoryClient;

pub struct GameFactoryActor {
    game_factory: GameFactory,
    game_factory_rx: Receiver<GameFactoryCommand>,
    game_factory_tx: Sender<GameFactoryCommand>,
}

impl GameFactoryActor {
    /// Runs the GameFactory Actor in background and returns a Client to communicate with it
    pub fn spawn(game_inactivity_timeout: Duration) -> GameFactoryClient {
        let game_factory = GameFactory::new(game_inactivity_timeout);
        let (game_factory_tx, game_factory_rx): (
            Sender<GameFactoryCommand>,
            Receiver<GameFactoryCommand>,
        ) = mpsc::channel(512);

        tokio::spawn(
            GameFactoryActor {
                game_factory,
                game_factory_rx,
                game_factory_tx: game_factory_tx.clone(),
            }
            .start(),
        );

        GameFactoryClient { game_factory_tx }
    }

    async fn start(mut self) {
        while let Some(message) = self.game_factory_rx.recv().await {
            match message {
                GameFactoryCommand::CreateGame { response_channel } => {
                    let game_id = self.game_factory.create_new_game(GameFactoryClient {
                        game_factory_tx: self.game_factory_tx.clone(),
                    });
                    let game_created = GameFactoryResponse::GameCreated { game_id };
                    if let Err(error) = response_channel.send(game_created) {
                        log::error!(
                            "The GameFactory response channel is closed. Error: '{error}'."
                        );
                    }
                }
                GameFactoryCommand::RemoveGame { game_id } => {
                    let _ = self.game_factory.remove_game(&game_id);
                }
                GameFactoryCommand::GetGameActor {
                    game_id,
                    response_channel,
                } => {
                    let response = self.game_factory.get_game(&game_id).map_or_else(
                        |error| GameFactoryResponse::Error { error },
                        |game| GameFactoryResponse::GameActor { game: game.clone() },
                    );
                    if let Err(error) = response_channel.send(response) {
                        log::error!(
                            "The GameFactory response channel is closed. Error: '{error}'."
                        );
                    }
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
    RemoveGame {
        game_id: String,
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
    Error { error: Error },
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
                GameFactoryResponse::Error { error } => format!("Error '{error}'").to_string(),
            }
        )
    }
}
