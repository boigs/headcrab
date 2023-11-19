use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum Error {
    #[error("The game with id '{0}' does not exist")]
    GameDoesNotExist(String),
    #[error("The player with nickname '{0}' already exists")]
    PlayerAlreadyExists(String),
    #[error("Internal Error '{0}'")]
    Internal(String),
}
