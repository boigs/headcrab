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
    #[error("Received a bad formatted message. Message: '{1}', Error: '{0}'.")]
    UnprocessableMessage(String, String),
    #[error("Internal Error '{0}'")]
    Internal(String),
    #[error("The websocket with the player is closed '{0}'")]
    WebsocketClosed(String),
}

impl Error {
    pub fn is_domain_error(&self) -> bool {
        match self {
            Error::NotEnoughPlayers => true,
            Error::CommandNotAllowed(_, _) => true,
            Error::GameDoesNotExist(_) => true,
            Error::PlayerAlreadyExists(_) => true,
            Error::UnprocessableMessage(_, _) => true,
            Error::Internal(_) => false,
            Error::WebsocketClosed(_) => false,
        }
    }

    pub fn log_and_create_internal(message: &str) -> Error {
        log::error!("{message}");
        Error::Internal(message.to_string())
    }
}

#[cfg(test)]
mod tests {
    use crate::error::Error;

    #[test]
    fn is_domain_error_is_true() {
        assert!(Error::GameDoesNotExist("".to_owned()).is_domain_error());
        assert!(Error::PlayerAlreadyExists("".to_owned()).is_domain_error());
        assert!(Error::NotEnoughPlayers.is_domain_error());
        assert!(Error::CommandNotAllowed("".to_owned(), "".to_owned()).is_domain_error());
        assert!(Error::UnprocessableMessage("".to_string(), "".to_string()).is_domain_error());
    }

    #[test]
    fn is_domain_error_is_false() {
        assert!(!Error::Internal("".to_owned()).is_domain_error());
        assert!(!Error::WebsocketClosed("".to_owned()).is_domain_error());
    }
}
