use thiserror::Error;

#[derive(Clone, Debug, Error, PartialEq)]
pub enum ExternalError {
    #[error("Received a bad formatted message. Message: '{1}', Error: '{0}'.")]
    UnprocessableWebsocketMessage(String, String),
    #[error("The websocket with the player is closed. Reason: '{0}'.")]
    WebsocketClosed(String),
}
