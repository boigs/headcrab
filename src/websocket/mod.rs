pub mod message;

use axum::extract::ws::{Message, WebSocket};
use serde::Serialize;

use crate::error::domain_error::DomainError;
use crate::error::external_error::ExternalError;
use crate::error::Error;
use crate::websocket::message::WsMessageOut;

use self::message::WsMessageIn;

pub async fn send_error(websocket: &mut WebSocket, error: &Error) {
    match error {
        // Do not return internal errors to the user
        Error::Internal(_) => {}
        _ => {
            // We are closing the websocket, ignore if there's any error sending the last message
            let _ = send_message(websocket, &error_to_ws_error(error.clone())).await;
        }
    }
}

pub async fn close(websocket: WebSocket) {
    // The websocket might already be closed, if so, ignore the error
    let _ = websocket.close().await;
}

pub fn parse_message(message: &str) -> Result<WsMessageIn, Error> {
    serde_json::from_str(message).map_err(|error| {
        Error::External(ExternalError::UnprocessableWebsocketMessage(
            message.to_string(),
            error.to_string(),
        ))
    })
}

pub async fn send_message<T>(websocket: &mut WebSocket, value: &T) -> Result<(), Error>
where
    T: ?Sized + Serialize,
{
    let message = serde_json::to_string(value).map_err(|error| {
        Error::log_and_create_internal(&format!(
            "Could not serialize the message. Error: '{error}'."
        ))
    })?;
    send_message_string(websocket, &message).await
}

pub async fn send_message_string(websocket: &mut WebSocket, value: &str) -> Result<(), Error> {
    websocket
        .send(Message::Text(value.to_string()))
        .await
        .map_err(|error| Error::External(ExternalError::WebsocketClosed(error.to_string())))
}

fn error_to_ws_error(error: Error) -> WsMessageOut {
    WsMessageOut::Error {
        r#type: match error {
            Error::Domain(ref domain_error) => match domain_error {
                DomainError::GameAlreadyInProgress(_) => "GAME_ALREADY_IN_PROGRESS",
                DomainError::GameDoesNotExist(_) => "GAME_DOES_NOT_EXIST",
                DomainError::InvalidStateForWordsSubmission(_, _) => {
                    "INVALID_STATE_FOR_WORDS_SUBMISSION"
                }
                DomainError::InvalidStateForVotingWordSubmission(_, _) => {
                    "INVALID_STATE_FOR_VOTING_WORD_SUBMISSION"
                }
                DomainError::NotEnoughPlayers(_, _) => "NOT_ENOUGH_PLAYERS",
                DomainError::NotEnoughRounds(_, _) => "NOT_ENOUGH_ROUNDS",
                DomainError::NonHostPlayerCannotContinueToNewGame(_) => {
                    "NON_HOST_PLAYER_CANNOT_CONTINUE_TO_NEW_GAME"
                }
                DomainError::NonHostPlayerCannotContinueToNextRound(_) => {
                    "NON_HOST_PLAYER_CANNOT_CONTINUE_TO_NEXT_ROUND"
                }
                DomainError::NonHostPlayerCannotContinueToNextVotingItem(_) => {
                    "NON_HOST_PLAYER_CANNOT_CONTINUE_TO_NEXT_VOTING_ITEM"
                }
                DomainError::NonHostPlayerCannotStartGame(_) => "NON_HOST_PLAYER_CANNOT_START_GAME",
                DomainError::PlayerAlreadyExists(_) => "PLAYER_ALREADY_EXISTS",
                DomainError::PlayerCannotSubmitNonExistingOrUsedVotingWord(_) => {
                    "PLAYER_CANNOT_SUBMIT_NON_EXISTING_OR_USED_WORD"
                }
                DomainError::PlayerCannotSubmitVotingWordWhenVotingItemIsNone(_) => {
                    "PLAYER_CANNOT_SUBMIT_VOTING_WORD_WHEN_VOTING_ITEM_IS_NONE"
                }
                DomainError::RepeatedWords { .. } => "REPEATED_WORDS",
                DomainError::VotingItemPlayerCannotSubmitVotingWord(_) => {
                    "VOTING_ITEM_PLAYER_CANNOT_SUBMIT_VOTING_WORD"
                }
            },
            Error::External(ref external_error) => match external_error {
                ExternalError::UnprocessableWebsocketMessage(_, _) => {
                    "UNPROCESSABLE_WEBSOCKET_MESSAGE"
                }
                ExternalError::WebsocketClosed(_) => "WEBSOCKET_CLOSED",
            },
            Error::Internal(_) => "INTERNAL",
        }
        .to_string(),
        title: error.to_string(),
        detail: match error {
            Error::Domain(domain_error) => match domain_error {
                DomainError::RepeatedWords {
                    nickname: _,
                    repeated_words,
                } => repeated_words.join(","),
                _ => domain_error.to_string(),
            },
            Error::External(external_error) => external_error.to_string(),
            Error::Internal(internal_error) => internal_error.to_string(),
        },
    }
}
