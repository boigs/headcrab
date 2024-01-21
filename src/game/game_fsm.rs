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

    Lobby(StartGame) => CreatingNewRound,
    CreatingNewRound => {
        StartRound => PlayersWritingWords,
        NoMoreRounds => EndOfGame,
    },
    PlayersWritingWords => {
        //TimesUp => ScoreCounting,
        PlayersFinished => ScoreCounting
    },
    ScoreCounting(BeginScoreCounting) => ChooseNextPlayer,
    ChooseNextPlayer => {
        NoMorePlayers => CreatingNewRound,
        NextPlayer => ChooseNextWord,
    },
    ChooseNextWord => {
        NoMoreWords => ChooseNextPlayer,
        NextWord => PlayersSendingWordSubmission,
    },
    PlayersSendingWordSubmission => {
        AllPlayersSentWordSubmission => ChooseNextWord,
    }
}
