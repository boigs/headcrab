use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum Error {
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
    #[error("The websocket received a bad formatted message. Error: '{0}'.")]
    UnprocessableWebsocketMessage(String),
    #[error("The websocket received an unknown text message. Message: '{0}', Error: '{1}'.")]
    UnknownTextMessage(String, String),
}

impl Error {
    pub fn log_and_create_internal(message: &str) -> Error {
        log::error!("{message}");
        Error::Internal(message.to_string())
    }
}
