use thiserror::Error;

use crate::game::game_fsm::GameFsmState;

#[derive(Clone, Debug, Error, PartialEq)]
pub enum DomainError {
    #[error("The game is already in progress. GameId: '{0}'.")]
    GameAlreadyInProgress(String),
    #[error("The game does not exist. GameId: '{0}'.")]
    GameDoesNotExist(String),
    #[error("Invalid state for submitting Words. ActualState: '{0:?}', ExpectedState: '{1:?}'.")]
    InvalidStateForWordsSubmission(GameFsmState, GameFsmState),
    #[error(
        "Invalid state for submitting a Voting Word. ActualState: '{0:?}', ExpectedState: '{1:?}'."
    )]
    InvalidStateForVotingWordSubmission(GameFsmState, GameFsmState),
    #[error("Not enough players to start the game. ActualPlayers: '{0}', MinimumPlayers: '{1}'.")]
    NotEnoughPlayers(usize, usize),
    #[error("Not enough rounds to start the game. ActualRounds: '{0}', MinimumRounds: '{1}'.")]
    NotEnoughRounds(usize, usize),
    #[error("A non host player cannot continue the game to the next round. Nickname: '{0}'.")]
    NonHostPlayerCannotContinueToNextRound(String),
    #[error(
        "A non host player cannot continue the game to the next voting item. Nickname: '{0}'."
    )]
    NonHostPlayerCannotSendPlayAgain(String),
    #[error("A non host player cannot send play again. Nickname: '{0}'.")]
    NonHostPlayerCannotContinueToNextVotingItem(String),
    #[error(
        "A non host player cannot continue the game to the next voting item. Nickname: '{0}'."
    )]
    NonHostPlayerCannotStartGame(String),
    #[error("A player with the same nickname already exists. Nickname: '{0}'.")]
    PlayerAlreadyExists(String),
    #[error(
        "A player cannot submit a non-existing or used word as a Voting Word. Nickname: '{0}'."
    )]
    PlayerCannotSubmitNonExistingOrUsedVotingWord(String),
    #[error("A player cannot submit a Voting Word when the current Voting Item is none. Nickname: '{0}'.")]
    PlayerCannotSubmitVotingWordWhenVotingItemIsNone(String),
    #[error(
        "A player cannot submit Words with repeated words. Nickname: '{nickname}', RepeatedWords: '{}'.", .repeated_words.join(",")
    )]
    RepeatedWords {
        nickname: String,
        repeated_words: Vec<String>,
    },
    #[error("The player of the current Voting Item cannot submit a Voting Word. Nickname: '{0}'.")]
    VotingItemPlayerCannotSubmitVotingWord(String),
    #[error("Cannot reject words in the current state.")]
    InvalidStateForRejectingMatchedWords,
    #[error("The rejected matched word does not exist.")]
    RejectedMatchedWordDoesNotExist,
    #[error("The rejected player does not exist.")]
    RejectedMatchedPlayerDoesNotExist,
    #[error("Non host cannot reject matched words")]
    NonHostCannotRejectMatchedWords,
    #[error("Cannot reject matching words when voting item is none")]
    CannotRejectMatchedWordsWhenVotingItemIsNone,
    #[error("Cannot reject a word that was not previously picked by the player during matching")]
    RejectedMatchedWordWasNotPickedByPlayer,
}
