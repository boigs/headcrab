use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::{self, Receiver as OneshotReceiver, Sender as OneshotSender};

use crate::actor::game::client::GameClient;

use super::{GameFactoryCommand, GameFactoryResponse};

pub struct GameFactoryClient {
    pub(super) game_factory_tx: Sender<GameFactoryCommand>,
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
                log::info!(
                    "ERROR: Received an unexpected GameFactoryResponse. Response: {unexpected_response}.",
                );
                Err(format!(
                    "ERROR: Received an unexpected GameFactoryResponse. Error {unexpected_response}."
                ))
            }
            Err(error) => {
                log::error!("The Game channel is closed. Error: {error}.");
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
                log::info!(
                    "ERROR: Received an unexpected GameFactoryResponse. Response: {unexpected_response}.",
                );
                Err(format!(
                    "ERROR: Received an unexpected GameFactoryResponse. Error {unexpected_response}."
                ))
            }
            Err(error) => {
                log::error!("The Game channel is closed. Error: {error}.");
                Err(format!(
                    "ERROR: The Game channel is closed. Error: {error}."
                ))
            }
        }
    }
}
