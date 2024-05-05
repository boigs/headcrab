#[derive(Clone, Debug, PartialEq)]
pub enum DomainErrorType {
    GameAlreadyInProgress,
    GameDoesNotExist,
    InvalidStateForWordsSubmission,
    InvalidStateForVotingWordSubmission,
    NotEnoughPlayers,
    NotEnoughRounds,
    NonHostPlayerCannotContinueToNextRound,
    NonHostPlayerCannotContinueToNextVotingItem,
    NonHostPlayerCannotStartGame,
    PlayerAlreadyExists,
    PlayerCannotSubmitVotingWordWhenVotingItemIsNone,
    PlayerCannotSubmitNonExistingOrUsedVotingWord,
    RepeatedWords,
    VotingItemPlayerCannotSubmitVotingWord,
}
