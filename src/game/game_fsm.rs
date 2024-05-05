use std::fmt;

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
        StartGame => CreatingNewRound
    },
    CreatingNewRound => {
        StartRound => PlayersSubmittingWords,
        NoMoreRounds => EndOfGame,
    },
    PlayersSubmittingWords => {
        // TODO: TimesUp => ScoreCounting,
        AllPlayersSubmittedWords => ChooseNextVotingItem
    },
    ChooseNextVotingItem => {
        NextVotingItem => PlayersSubmittingVotingWord,
        NoMoreVotingItems => EndOfRound,
    },
    PlayersSubmittingVotingWord => {
        AcceptPlayersVotingWords => ChooseNextVotingItem,
    },
    EndOfRound => {
        ContinueToNextRound => CreatingNewRound,
    }
}

impl fmt::Display for GameFsmState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
