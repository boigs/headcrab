pub mod domain_error;

use thiserror::Error;

use self::domain_error::DomainError;

#[derive(Clone, Debug, Error, PartialEq)]
pub enum Error {
    #[error("Domain Error.")]
    Domain(DomainError),
    #[error("Internal Error. Error: '{0}'.")]
    Internal(String),
    #[error("Received a bad formatted message. Message: '{1}', Error: '{0}'.")]
    UnprocessableMessage(String, String),
    #[error("The websocket with the player is closed. Reason: '{0}'.")]
    WebsocketClosed(String),
}

impl Error {
    pub fn log_and_create_internal(message: &str) -> Error {
        log::error!("{message}");
        Error::Internal(message.to_string())
    }
}
