pub mod actor;
pub mod actor_client;
pub mod game_fsm;

use rust_fsm::StateMachine;

use crate::error::Error;
use crate::game::game_fsm::{GameFsm, GameFsmInput, GameFsmState};
use crate::player::Player;
use crate::round::Round;

pub struct Game {
    id: String,
    fsm: StateMachine<GameFsm>,
    players: Vec<Player>,
    rounds: Vec<Round>,
}

impl Game {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            fsm: StateMachine::default(),
            players: Vec::default(),
            rounds: Vec::default(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn state(&self) -> &GameFsmState {
        self.fsm.state()
    }

    pub fn players(&self) -> &[Player] {
        &self.players
    }

    pub fn rounds(&self) -> &[Round] {
        &self.rounds
    }

    pub fn all_players_are_disconnected(&self) -> bool {
        self.get_connected_players().is_empty()
    }

    fn get_connected_players(&self) -> Vec<&Player> {
        self.players
            .iter()
            .filter(|player| player.is_connected)
            .collect()
    }

    pub fn add_player(&mut self, nickname: &str) -> Result<(), Error> {
        let state = self.state().clone();

        if let Some(player) = self.get_player_mut(nickname) {
            if player.is_connected {
                return Err(Error::PlayerAlreadyExists(nickname.to_string()));
            } else {
                player.is_connected = true;
            }
        } else if state == GameFsmState::Lobby {
            let new_player = Player::new(nickname);
            self.players.push(new_player);
        } else {
            return Err(Error::GameAlreadyInProgress);
        }

        self.assign_host();
        Ok(())
    }

    pub fn disconnect_player(&mut self, nickname: &str) -> Result<(), Error> {
        if let Some(player) = self.get_player_mut(nickname) {
            player.is_connected = false;
            player.is_host = false;
            self.assign_host();
            Ok(())
        } else {
            Err(Error::log_and_create_internal(&format!(
                "Tried to disconnect player '{nickname}' but it does not exist."
            )))
        }
    }

    pub fn start_game(&mut self, nickname: &str) -> Result<(), Error> {
        if self.is_host(nickname) {
            if self.get_connected_players().len() >= 3 {
                self.process_event(&GameFsmInput::StartGame)
            } else {
                Err(Error::NotEnoughPlayers)
            }
        } else {
            Err(Error::CommandNotAllowed(
                nickname.to_string(),
                "start_game".to_string(),
            ))
        }
    }

    fn get_player(&self, nickname: &str) -> Option<&Player> {
        self.players
            .iter()
            .find(|player| player.nickname == nickname)
    }

    fn get_player_mut(&mut self, nickname: &str) -> Option<&mut Player> {
        self.players
            .iter_mut()
            .find(|player| player.nickname == nickname)
    }

    fn assign_host(&mut self) {
        if self.players.iter().all(|player| !player.is_host) {
            if let Some(player) = self.players.iter_mut().find(|player| player.is_connected) {
                player.is_host = true;
            }
        }
    }

    fn is_host(&self, nickname: &str) -> bool {
        self.get_player(nickname)
            .map(|player| player.is_host)
            .unwrap_or(false)
    }

    fn get_current_round_mut(&mut self) -> &mut Round {
        self.rounds.last_mut().unwrap()
    }

    fn process_event(&mut self, event: &GameFsmInput) -> Result<(), Error> {
        match self.fsm.consume(event) {
            Ok(_) => match self.fsm.state() {
                GameFsmState::CreatingNewRound => {
                    if self.rounds.len() >= 3 {
                        self.process_event(&GameFsmInput::NoMoreRounds)
                    } else {
                        self.start_new_round()?;
                        self.process_event(&GameFsmInput::StartRound)
                    }
                }
                GameFsmState::PlayersWritingWords => Ok(()),
                GameFsmState::Lobby => Ok(()),
                GameFsmState::ScoreCounting => {
                    self.process_event(&GameFsmInput::BeginScoreCounting)
                }
                GameFsmState::PlayersSendingWordSubmission => Ok(()),
                GameFsmState::ChooseNextPlayer => {
                    if self
                        .get_current_round_mut()
                        .next_player_to_score()
                        .is_some()
                    {
                        self.process_event(&GameFsmInput::NextPlayer)
                    } else {
                        self.process_event(&GameFsmInput::NoMorePlayers)
                    }
                }
                GameFsmState::ChooseNextWord => {
                    if self.get_current_round_mut().next_word_to_score().is_some() {
                        self.process_event(&GameFsmInput::NextWord)
                    } else {
                        self.process_event(&GameFsmInput::NoMoreWords)
                    }
                }
                GameFsmState::EndOfRound => Ok(()),
                GameFsmState::EndOfGame => Ok(()),
            },
            Err(error) => Err(Error::log_and_create_internal(&format!(
                "The fsm in state {:?} can't transition with an event {:?}. Error: '{error}'.",
                self.fsm.state(),
                event
            ))),
        }
    }

    fn start_new_round(&mut self) -> Result<(), Error> {
        let word = Game::choose_random_word();
        let round = Round::new(
            &word,
            self.players()
                .iter()
                .map(|player| player.nickname.clone())
                .collect(),
        )?;
        self.rounds.push(round);
        Ok(())
    }

    fn choose_random_word() -> String {
        "alien".to_string()
    }

    pub fn add_word_to_score(&mut self, nickname: &str, word: Option<String>) -> Result<(), Error> {
        // None if the player says they don't have that word on their list
        // Verify the player has this word
        // Verify the player hasn't already added this word as validated
        // If all players have sent something then compute the score and go to validate the next word
        if self.state() != &GameFsmState::PlayersSendingWordSubmission {
            return Err(Error::CommandNotAllowed(
                nickname.to_string(),
                "add_word_to_score_invalid_state".to_string(),
            ));
        }

        let number_of_connected_players = self.get_connected_players().len();

        let current_round = self.get_current_round_mut();
        if let Some(word) = word.clone() {
            if !current_round.player_has_word(nickname, &word) {
                return Err(Error::CommandNotAllowed(
                    nickname.to_string(),
                    "add_word_to_score_non_existing_word".to_string(),
                ));
            }
        }

        current_round.add_player_word_submission(nickname, word)?;

        if current_round.players_submitted_words_count() >= number_of_connected_players {
            current_round.compute_score();
            return self.process_event(&GameFsmInput::AllPlayersSentWordSubmission);
        }

        Ok(())
    }

    pub fn add_words(&mut self, nickname: &str, words: Vec<String>) -> Result<(), Error> {
        if self.fsm.state() != &GameFsmState::PlayersWritingWords {
            return Err(Error::CommandNotAllowed(
                nickname.to_string(),
                "AddWords".to_string(),
            ));
        }

        if let Some(round) = self.rounds.last_mut() {
            round.add_words(nickname, words)?;
            let connected_players: Vec<String> = self
                .players
                .iter()
                .filter(|player| player.is_connected)
                .map(|player| player.nickname.clone())
                .collect();
            if round.have_all_players_submitted_words(&connected_players) {
                return self.process_event(&GameFsmInput::PlayersFinished);
            }
        }

        Ok(())
    }

    pub fn continue_to_next_round(&mut self, nickname: &str) -> Result<(), Error> {
        if self.is_host(nickname) {
            self.process_event(&GameFsmInput::ContinueToNextRound)
        } else {
            Err(Error::CommandNotAllowed(
                nickname.to_string(),
                "continue_to_next_round".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Game;
    use crate::{error::Error, game::game_fsm::GameFsmState};

    static PLAYER_1: &str = "p1";
    static PLAYER_2: &str = "p2";
    static PLAYER_3: &str = "p3";
    fn players() -> Vec<String> {
        vec![PLAYER_1, PLAYER_2, PLAYER_3]
            .iter()
            .map(|player| player.to_string())
            .collect()
    }

    static WORD_1: &str = "w1";
    static WORD_2: &str = "w2";
    fn words() -> Vec<String> {
        vec![WORD_1, WORD_2]
            .iter()
            .map(|word| word.to_string())
            .collect()
    }

    #[test]
    fn add_player_works() {
        let mut game = Game::new("id");

        game.add_player(PLAYER_1).unwrap();

        assert_eq!(game.players().len(), 1);
        assert_eq!(game.players()[0].nickname, PLAYER_1);
    }

    #[test]
    fn disconnect_player_works() {
        let mut game = get_game(&GameFsmState::Lobby);

        assert_eq!(game.players().len(), 3);
        assert!(game.players()[0].is_connected);
        assert!(game.players()[1].is_connected);
        assert!(game.players()[2].is_connected);

        game.disconnect_player(PLAYER_1).unwrap();

        assert_eq!(game.players().len(), 3);
        assert!(!game.players()[0].is_connected);
        assert!(game.players()[1].is_connected);
        assert!(game.players()[2].is_connected);
    }

    #[test]
    fn disconnect_non_existing() {
        let mut game = get_game(&GameFsmState::Lobby);

        let removed = game.disconnect_player("non_existent_player");

        assert!(removed.is_err());
    }

    #[test]
    fn only_first_player_added_is_host() {
        let game = get_game(&GameFsmState::Lobby);

        assert!(game.players()[0].is_host);
        assert!(!game.players()[1].is_host);
    }

    #[test]
    fn host_player_is_reelected_when_disconnected() {
        let mut game = get_game(&GameFsmState::Lobby);

        game.disconnect_player(PLAYER_1).unwrap();

        assert!(!game.players()[0].is_host);
        assert!(game.players()[1].is_host);
    }

    #[test]
    fn game_cannot_be_started_with_less_than_three_players() {
        let mut game = Game::new("id");
        game.add_player(PLAYER_1).unwrap();

        let result = game.start_game(PLAYER_1);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), Error::NotEnoughPlayers);
    }

    #[test]
    fn host_player_can_start_game() {
        let mut game = get_game(&GameFsmState::Lobby);

        assert!(game.start_game(PLAYER_1).is_ok());
    }

    #[test]
    fn non_host_player_cannot_start_game() {
        let mut game = get_game(&GameFsmState::Lobby);

        let result = game.start_game(PLAYER_2);
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            Error::CommandNotAllowed("p2".to_string(), "start_game".to_string())
        );
    }

    #[test]
    fn game_starts_in_lobby() {
        let game = Game::new("id");

        assert_eq!(game.state(), &GameFsmState::Lobby);
    }

    #[test]
    fn game_initializes_first_round() {
        let mut game = get_game(&GameFsmState::Lobby);

        game.start_game(PLAYER_1).unwrap();

        assert_eq!(game.state(), &GameFsmState::PlayersWritingWords);
        assert_eq!(game.rounds().len(), 1);
        assert!(!game.rounds().first().unwrap().word.is_empty());
    }

    #[test]
    fn all_players_are_disconnected_is_false() {
        let game = get_game(&GameFsmState::Lobby);

        assert!(!game.all_players_are_disconnected());
    }

    #[test]
    fn all_players_are_disconnected_is_true() {
        let mut game = get_game(&GameFsmState::Lobby);
        let _ = game.disconnect_player(PLAYER_1);
        let _ = game.disconnect_player(PLAYER_2);
        let _ = game.disconnect_player(PLAYER_3);

        assert!(game.all_players_are_disconnected());
    }

    #[test]
    fn all_players_are_disconnected_is_true_when_empty_players() {
        let game = Game::new("id");

        assert!(game.all_players_are_disconnected());
    }

    #[test]
    fn add_words_works() {
        let mut game = get_game(&GameFsmState::PlayersWritingWords);

        let result = game.add_words(PLAYER_1, words());

        assert!(result.is_ok());
        assert_eq!(game.state(), &GameFsmState::PlayersWritingWords);
    }

    #[test]
    fn add_words_transitions_to_players_sending_word_submission_on_last_player_words() {
        let mut game = get_game(&GameFsmState::PlayersWritingWords);

        game.add_words(PLAYER_1, words()).unwrap();
        game.add_words(PLAYER_2, words()).unwrap();
        game.add_words(PLAYER_3, words()).unwrap();

        assert_eq!(game.state(), &GameFsmState::PlayersSendingWordSubmission);
    }

    #[test]
    fn add_words_fails_when_state_is_not_players_writing_words() {
        let mut game = get_game(&GameFsmState::Lobby);

        let result = game.add_words(PLAYER_1, words());

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            Error::CommandNotAllowed(PLAYER_1.to_string(), "AddWords".to_string())
        );
    }

    #[test]
    fn add_word_to_score_works_with_word() {
        let mut game = get_game(&GameFsmState::PlayersSendingWordSubmission);

        let result = game.add_word_to_score(PLAYER_2, Some(WORD_1.to_string()));

        assert!(result.is_ok());
        assert_eq!(game.state(), &GameFsmState::PlayersSendingWordSubmission);
    }

    #[test]
    fn add_word_to_score_works_with_empty_word() {
        let mut game = get_game(&GameFsmState::PlayersSendingWordSubmission);

        let result = game.add_word_to_score(PLAYER_2, None);

        assert!(result.is_ok());
        assert_eq!(game.state(), &GameFsmState::PlayersSendingWordSubmission);
    }

    #[test]
    fn add_word_to_score_fails_with_unsubmitted_word() {
        let mut game = get_game(&GameFsmState::PlayersSendingWordSubmission);

        let result = game.add_word_to_score(PLAYER_2, Some("NOT_SUBMITTED_WORD".to_string()));

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            Error::CommandNotAllowed(
                PLAYER_2.to_string(),
                "add_word_to_score_non_existing_word".to_string()
            )
        );
    }

    #[test]
    fn add_word_to_score_fails_when_state_is_not_players_sending_word_submission() {
        let mut game = get_game(&GameFsmState::PlayersWritingWords);

        let result = game.add_word_to_score(PLAYER_2, Some(WORD_1.to_string()));

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            Error::CommandNotAllowed(
                PLAYER_2.to_string(),
                "add_word_to_score_invalid_state".to_string()
            )
        );
    }

    #[test]
    fn when_all_players_send_word_to_score_game_goes_to_end_of_round() {
        let mut game = get_game(&GameFsmState::PlayersWritingWords);

        complete_round(&mut game);

        assert_eq!(game.state(), &GameFsmState::EndOfRound);
    }

    #[test]
    fn continue_to_next_round_fails_when_state_is_not_end_of_round() {
        let mut game = get_game(&GameFsmState::Lobby);

        let result = game.continue_to_next_round(PLAYER_1);

        assert!(result.is_err());
    }

    #[test]
    fn continue_to_next_round_fails_when_player_is_not_host() {
        let mut game = get_game(&GameFsmState::EndOfRound);

        let result = game.continue_to_next_round(PLAYER_2);

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            Error::CommandNotAllowed(PLAYER_2.to_string(), "continue_to_next_round".to_string())
        );
    }

    #[test]
    fn continue_to_next_round_proceeds_to_next_round() {
        let mut game = get_game(&GameFsmState::EndOfRound);

        let result = game.continue_to_next_round(PLAYER_1);

        assert!(result.is_ok());
        assert_eq!(game.state(), &GameFsmState::PlayersWritingWords);
    }

    #[test]
    fn continue_to_next_round_proceeds_to_end_of_game() {
        let mut game = get_game(&GameFsmState::PlayersWritingWords);

        complete_round(&mut game);
        game.continue_to_next_round(PLAYER_1).unwrap();
        complete_round(&mut game);
        game.continue_to_next_round(PLAYER_1).unwrap();
        complete_round(&mut game);
        game.continue_to_next_round(PLAYER_1).unwrap();

        assert_eq!(game.state(), &GameFsmState::EndOfGame);
    }

    #[test]
    fn new_players_cannot_be_added_after_game_is_started() {
        let mut game = get_game(&GameFsmState::PlayersWritingWords);

        let result = game.add_player("new_player");

        assert_eq!(result, Err(Error::GameAlreadyInProgress));
    }

    #[test]
    fn existing_player_can_rejoin_after_game_is_started() {
        let mut game = get_game(&GameFsmState::PlayersWritingWords);

        let _ = game.disconnect_player(PLAYER_2);

        let result = game.add_player(PLAYER_2);

        assert_eq!(result, Ok(()));
    }

    fn get_game(state: &GameFsmState) -> Game {
        let mut game = Game::new("id");
        game.add_player(PLAYER_1).unwrap();
        game.add_player(PLAYER_2).unwrap();
        game.add_player(PLAYER_3).unwrap();

        match state {
            GameFsmState::Lobby => {}
            GameFsmState::PlayersWritingWords => game.start_game(PLAYER_1).unwrap(),
            GameFsmState::PlayersSendingWordSubmission => {
                game.start_game(PLAYER_1).unwrap();
                send_players_words_submision(&mut game);
            }
            GameFsmState::EndOfRound => {
                game.start_game(PLAYER_1).unwrap();
                complete_round(&mut game);
            }
            _ => panic!("Unsupported desired state for unit tests"),
        }
        assert_eq!(game.state(), state);

        game
    }

    fn send_players_words_submision(game: &mut Game) {
        for player in players() {
            game.add_words(&player, words()).unwrap();
        }
    }

    fn complete_round(game: &mut Game) {
        send_players_words_submision(game);
        for word in words() {
            for player in players() {
                // For simplicity in the test setup, we'll iterate over all the words, even if they are already used, ignore such error
                let _ = game.add_word_to_score(&player, Some(word.to_string()));
            }
        }
    }
}
