use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::{self, Receiver as OneshotReceiver, Sender as OneshotSender};

use crate::error::Error;
use crate::game::actor_client::GameClient;
use crate::game_factory::actor::{GameFactoryCommand, GameFactoryResponse};

pub struct GameFactoryClient {
    pub(super) game_factory_tx: Sender<GameFactoryCommand>,
}

impl GameFactoryClient {
    pub async fn create_game(&self) -> Result<String, Error> {
        let (tx, rx): (
            OneshotSender<GameFactoryResponse>,
            OneshotReceiver<GameFactoryResponse>,
        ) = oneshot::channel();

        self.game_factory_tx
            .send(GameFactoryCommand::CreateGame {
                response_channel: tx,
            })
            .await
            .map_err(|error| {
                Error::log_and_create_internal(&format!(
                    "The GameFactory is not alive. Can't create Game. Error: '{error}'"
                ))
            })?;

        match rx.await {
            Ok(GameFactoryResponse::GameCreated { game_id }) => Ok(game_id),
            Ok(unexpected_response) => Err(Error::log_and_create_internal(&format!(
                "Received an unexpected GameFactoryResponse. Error {unexpected_response}."
            ))),
            Err(error) => Err(Error::log_and_create_internal(&format!(
                "The GameFactory channel is closed. Error: {error}."
            ))),
        }
    }

    pub async fn remove_game(&self, game_id: &str) -> Result<(), Error> {
        self.game_factory_tx
            .send(GameFactoryCommand::RemoveGame {
                game_id: game_id.to_string(),
            })
            .await
            .map_err(|error| {
                Error::log_and_create_internal(&format!(
                    "The GameFactory channel is closed. Error: '{error}'."
                ))
            })
    }

    pub async fn get_game(&self, game_id: &str) -> Result<GameClient, Error> {
        let (tx, rx): (
            OneshotSender<GameFactoryResponse>,
            OneshotReceiver<GameFactoryResponse>,
        ) = oneshot::channel();

        self.game_factory_tx
            .send(GameFactoryCommand::GetGameActor {
                game_id: game_id.to_string(),
                response_channel: tx,
            })
            .await
            .map_err(|error| {
                Error::log_and_create_internal(&format!(
                    "The GameFactory channel is closed. Error: '{error}'."
                ))
            })?;

        match rx.await {
            Ok(GameFactoryResponse::GameActor { game }) => Ok(game),
            Ok(GameFactoryResponse::Error { error }) => Err(error),
            Ok(unexpected_response) => Err(Error::log_and_create_internal(&format!(
                "Received an unexpected GameFactoryResponse. Error {unexpected_response}."
            ))),
            Err(error) => Err(Error::log_and_create_internal(&format!(
                "The GameFactory channel is closed. Error: {error}."
            ))),
        }
    }
}
