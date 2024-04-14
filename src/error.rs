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
    pub fn is_fatal_error_and_should_finalize_flow(&self) -> bool {
        // TODO change this to a match statement in order to get a compilation error when new errors are added
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

#[cfg(test)]
mod tests {
    use crate::error::Error;

    #[test]
    fn when_error_is_not_fatal_error_then_finalize_flow_is_false() {
        assert!(!Error::NotEnoughPlayers.is_fatal_error_and_should_finalize_flow());
        assert!(!Error::CommandNotAllowed("".to_owned(), "".to_owned())
            .is_fatal_error_and_should_finalize_flow());
        assert!(!Error::UnprocessableMessage("".to_string(), "".to_string())
            .is_fatal_error_and_should_finalize_flow());
    }

    #[test]
    fn when_error_is_fatal_error_then_finalize_flow_is_true() {
        assert!(Error::GameDoesNotExist("".to_owned()).is_fatal_error_and_should_finalize_flow());
        assert!(Error::PlayerAlreadyExists("".to_owned()).is_fatal_error_and_should_finalize_flow());
        assert!(Error::Internal("".to_owned()).is_fatal_error_and_should_finalize_flow());
        assert!(Error::WebsocketClosed("".to_owned()).is_fatal_error_and_should_finalize_flow());
    }
}
