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
    derive(Debug, Clone)
    pub GameFsm(Lobby)

    Lobby(StartGame) => ChooseWord,
    ChooseWord(WordChosen) => PlayersWritingWords,
    PlayersWritingWords => {
        TimesUp => WordCounting,
        PlayersFinished => WordCounting
    },
    WordCounting => {
        LastRound => EndOfGame,
        NotLastRound => ChooseWord
    },
}
