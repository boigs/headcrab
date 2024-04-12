use thiserror::Error;

#[derive(Clone, Debug, Error, PartialEq)]
pub enum Error {
    #[error("Not enough players to start the game.")]
    NotEnoughPlayers,
    #[error("The player '{0}' cannot execute command '{1}'")]
    CommandNotAllowed(String, String),
    #[error("The game with id '{0}' does not exist")]
    GameDoesNotExist(String),
    #[error("The player with nickname '{0}' already exists")]
    PlayerAlreadyExists(String),
    #[error("Internal Error '{0}'")]
    Internal(String),
    #[error("The websocket with the player is closed '{0}'")]
    WebsocketClosed(String),
    #[error("Received a bad formatted message. Message: '{1}', Error: '{0}'.")]
    UnprocessableMessage(String, String),
}

impl Error {
    pub fn is_error_to_finalize_flow(&self) -> bool {
        !matches!(
            self,
            Error::NotEnoughPlayers
                | Error::CommandNotAllowed(_, _)
                | Error::UnprocessableMessage(_, _)
        )
    }

    pub fn log_and_create_internal(message: &str) -> Error {
        log::error!("{message}");
        Error::Internal(message.to_string())
    }
}
