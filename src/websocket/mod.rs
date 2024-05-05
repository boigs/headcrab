pub mod message;

use axum::extract::ws::{Message, WebSocket};
use serde::Serialize;

use crate::error::domain_error_type::DomainErrorType;
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
    serde_json::from_str(message)
        .map_err(|error| Error::UnprocessableMessage(message.to_string(), error.to_string()))
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
        .map_err(|error| Error::WebsocketClosed(error.to_string()))
}

fn error_to_ws_error(error: Error) -> WsMessageOut {
    match error {
        Error::Domain(error_type, detail) => WsMessageOut::Error {
            r#type: match error_type {
                DomainErrorType::GameAlreadyInProgress => "GAME_ALREADY_IN_PROGRESS",
                DomainErrorType::GameDoesNotExist => "GAME_DOES_NOT_EXIST",
                DomainErrorType::InvalidStateForWordsSubmission => {
                    "INVALID_STATE_FOR_WORDS_SUBMISSION"
                }
                DomainErrorType::InvalidStateForVotingWordSubmission => {
                    "INVALID_STATE_FOR_VOTING_WORD_SUBMISSION"
                }
                DomainErrorType::NotEnoughPlayers => "NOT_ENOUGH_PLAYERS",
                DomainErrorType::NotEnoughRounds => "NOT_ENOUGH_ROUNDS",
                DomainErrorType::NonHostPlayerCannotContinueToNextRound => {
                    "NON_HOST_PLAYER_CANNOT_CONTINUE_TO_NEXT_ROUND"
                }
                DomainErrorType::NonHostPlayerCannotContinueToNextVotingItem => {
                    "NON_HOST_PLAYER_CANNOT_CONTINUE_TO_NEXT_VOTING_ITEM"
                }
                DomainErrorType::NonHostPlayerCannotStartGame => {
                    "NON_HOST_PLAYER_CANNOT_START_GAME"
                }
                DomainErrorType::PlayerAlreadyExists => "PLAYER_ALREADY_EXISTS",
                DomainErrorType::PlayerCannotSubmitVotingWordWhenVotingItemIsNone => {
                    "PLAYER_CANNOT_SUBMIT_VOTING_WORD_WHEN_VOTING_ITEM_IS_NONE"
                }
                DomainErrorType::PlayerCannotSubmitNonExistingOrUsedVotingWord => {
                    "PLAYER_CANNOT_SUBMIT_NON_EXISTING_OR_USED_WORD"
                }
                DomainErrorType::RepeatedWords => "REPEATED_WORDS",
                DomainErrorType::VotingItemPlayerCannotSubmitVotingWord => {
                    "VOTING_ITEM_PLAYER_CANNOT_SUBMIT_VOTING_WORD"
                }
            }
            .to_string(),
            title: "Domain Error".to_string(),
            detail,
        },
        Error::Internal(_) => WsMessageOut::Error {
            r#type: "INTERNAL_SERVER".to_string(),
            title: "Internal Server Error".to_string(),
            detail: error.to_string(),
        },
        Error::UnprocessableMessage(_, _) => WsMessageOut::Error {
            r#type: "UNPROCESSABLE_WEBSOCKET_MESSAGE".to_string(),
            title: "Received an invalid message".to_string(),
            detail: error.to_string(),
        },
        Error::WebsocketClosed(_) => WsMessageOut::Error {
            r#type: "WEBSOCKET_CLOSED".to_string(),
            title: "The player websocket is closed".to_string(),
            detail: error.to_string(),
        },
    }
}
