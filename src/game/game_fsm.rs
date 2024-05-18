use rust_fsm::state_machine;

/*
 * Lobby
 * Round
 *    Seleccionar palabra
 *    Jugadores escriben sus palabras
 *    Recuento
 *    Si ultima ronda ir a fin partida, sino a ronda
 * Fin Partida
 */
state_machine! {
    derive(Debug, Clone, PartialEq)
    pub GameFsm(Lobby)

    Lobby =>  {
        StartGame => PlayersSubmittingWords
    },
    PlayersSubmittingWords => {
        // TODO: TimesUp => ScoreCounting,
        AllPlayersSubmittedWords => PlayersSubmittingVotingWord
    },
    PlayersSubmittingVotingWord => {
        NextVotingItem => PlayersSubmittingVotingWord,
        NoMoreVotingItems => EndOfRound,
    },
    EndOfRound => {
        NextRound => PlayersSubmittingWords,
        NoMoreRounds => EndOfGame,
    },
    EndOfGame => {
        PlayAgain => Lobby
    }
}
