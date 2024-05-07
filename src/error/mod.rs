pub mod domain_error;
pub mod external_error;

use thiserror::Error;

use self::{domain_error::DomainError, external_error::ExternalError};

#[derive(Clone, Debug, Error, PartialEq)]
pub enum Error {
    #[error("Domain Error.")]
    Domain(DomainError),
    #[error("External Error.")]
    External(ExternalError),
    #[error("Internal Error. Error: '{0}'.")]
    Internal(String),
}

impl Error {
    pub fn log_and_create_internal(message: &str) -> Error {
        log::error!("{message}");
        Error::Internal(message.to_string())
    }
}
