use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::error::RecvError;
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

        self.send_command(
            GameFactoryCommand::CreateGame {
                response_channel: tx,
            },
            "The GameFactory is not alive. Can't create Game",
        )
        .await?;

        match rx.await {
            Ok(GameFactoryResponse::GameCreated { game_id }) => Ok(game_id),
            error => Err(GameFactoryClient::handle_event_error(error)),
        }
    }

    pub async fn remove_game(&self, game_id: &str) -> Result<(), Error> {
        self.send_command(
            GameFactoryCommand::RemoveGame {
                game_id: game_id.to_string(),
            },
            "The GameFactory channel is closed",
        )
        .await
    }

    pub async fn get_game(&self, game_id: &str) -> Result<GameClient, Error> {
        let (tx, rx): (
            OneshotSender<GameFactoryResponse>,
            OneshotReceiver<GameFactoryResponse>,
        ) = oneshot::channel();

        self.send_command(
            GameFactoryCommand::GetGameActor {
                game_id: game_id.to_string(),
                response_channel: tx,
            },
            "The GameFactory channel is closed",
        )
        .await?;

        match rx.await {
            Ok(GameFactoryResponse::GameActor { game }) => Ok(game),
            error => Err(GameFactoryClient::handle_event_error(error)),
        }
    }

    async fn send_command(
        &self,
        command: GameFactoryCommand,
        error_message: &str,
    ) -> Result<(), Error> {
        self.game_factory_tx.send(command).await.map_err(|error| {
            Error::log_and_create_internal(&format!("{error_message}. Error: '{error}'"))
        })
    }

    fn handle_event_error(error: Result<GameFactoryResponse, RecvError>) -> Error {
        match error {
            Ok(GameFactoryResponse::Error { error }) => error,
            Ok(unexpected_response) => Error::log_and_create_internal(&format!(
                "Received an unexpected GameFactoryResponse. GameFactoryResponse: '{unexpected_response}'."
            )),
            _ => Error::log_and_create_internal(
                "Sent a command to the GameFactory actor, but the actor channel died.",
            ),
        }
    }
}
