use std::fmt::{Display, Formatter};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::oneshot::Sender as OneshotSender;

use crate::config::GameSettings;
use crate::error::Error;
use crate::game::actor_client::GameClient;
use crate::game_factory::actor_client::GameFactoryClient;
use crate::game_factory::GameFactory;

pub struct GameFactoryActor {
    game_factory: GameFactory,
    game_factory_rx: Receiver<GameFactoryCommand>,
    game_factory_tx: Sender<GameFactoryCommand>,
}

impl GameFactoryActor {
    /// Runs the GameFactory Actor in background and returns a Client to communicate with it
    pub fn spawn(game_settings: GameSettings) -> GameFactoryClient {
        let game_factory = GameFactory::new(game_settings);
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
            let response = match message {
                GameFactoryCommand::CreateGame { response_channel } => {
                    let game_id = self.game_factory.create_new_game(GameFactoryClient {
                        game_factory_tx: self.game_factory_tx.clone(),
                    });
                    Some((
                        Ok(GameFactoryResponse::GameCreated { game_id }),
                        response_channel,
                    ))
                }
                GameFactoryCommand::RemoveGame { game_id } => {
                    let _ = self.game_factory.remove_game(&game_id);
                    None
                }
                GameFactoryCommand::GetGameActor {
                    game_id,
                    response_channel,
                } => {
                    let result = self
                        .game_factory
                        .get_game(&game_id)
                        .map(|game| GameFactoryResponse::GameActor { game: game.clone() });
                    Some((result, response_channel))
                }
            };
            if let Some((result, response_tx)) = response {
                let event = match result {
                    Ok(event) => event,
                    Err(error) => GameFactoryResponse::Error { error },
                };
                if let Err(error) = response_tx.send(event) {
                    log::error!("Sent GameFactoryResponse but the response channel is closed. Error: '{error}'.");
                }
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum GameFactoryCommand {
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
pub(crate) enum GameFactoryResponse {
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
