pub mod actor;
pub mod actor_client;
pub mod game_fsm;
mod game_word;

use rand::{seq::SliceRandom, thread_rng};
use rust_fsm::StateMachine;

use crate::error::domain_error::DomainError;
use crate::error::Error;
use crate::game::game_fsm::{GameFsm, GameFsmInput, GameFsmState};
use crate::player::Player;
use crate::round::Round;

use self::game_word::GameWord;

pub struct Game {
    id: String,
    words: Vec<GameWord>,
    fsm: StateMachine<GameFsm>,
    players: Vec<Player>,
    rounds: Vec<Round>,
    pub amount_of_rounds: Option<u8>,
}

impl Game {
    const MINIMUM_PLAYERS: u8 = 3;
    const MINIMUM_ROUNDS: u8 = 1;
    const DEFAULT_ROUNDS: u8 = 3;

    pub fn new(id: &str, words: Vec<String>) -> Self {
        let words = if words.len() >= Game::MINIMUM_ROUNDS.into() {
            words
        } else {
            log::error!("Game created without enough words, defaulting to the built-in list of words. GameId: '{}', ActualWords: '{}', MinimumWords: '{}'", id, words.len(), Game::MINIMUM_ROUNDS);
            Game::default_words()
        };

        Self {
            id: id.to_string(),
            // Create a pre-shuffled list of words, so that we don't need to do random picks every round
            words: Game::shuffle_words(words),
            fsm: StateMachine::default(),
            players: Vec::default(),
            rounds: Vec::default(),
            amount_of_rounds: None,
        }
    }

    fn default_words() -> Vec<String> {
        ["summer", "space", "dog", "pizza", "rock", "picnic", "surf"]
            .iter()
            .map(|word| word.to_string())
            .collect()
    }

    fn shuffle_words(words: Vec<String>) -> Vec<GameWord> {
        let mut words: Vec<GameWord> = words
            .into_iter()
            .map(|word| GameWord {
                value: word,
                is_used: false,
            })
            .collect();
        let mut rng = thread_rng();
        words.shuffle(&mut rng);
        words
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
                return Err(Error::Domain(DomainError::PlayerAlreadyExists(
                    nickname.to_string(),
                )));
            } else {
                player.is_connected = true;
            }
        } else if state == GameFsmState::Lobby {
            let new_player = Player::new(nickname);
            self.players.push(new_player);
        } else {
            return Err(Error::Domain(DomainError::GameAlreadyInProgress(
                self.id.to_string(),
            )));
        }

        self.assign_host();
        Ok(())
    }

    pub fn disconnect_player(&mut self, nickname: &str) -> Result<(), Error> {
        if let Some(player) = self.get_player_mut(nickname) {
            player.is_connected = false;
            player.is_host = false;
            self.assign_host();
            self.check_transition_to_voting()
        } else {
            Err(Error::log_and_create_internal(&format!(
                "Tried to disconnect player '{nickname}' but it does not exist."
            )))
        }
    }

    pub fn start_game(&mut self, nickname: &str, amount_of_rounds: u8) -> Result<(), Error> {
        if self.is_host(nickname) {
            if amount_of_rounds < Game::MINIMUM_ROUNDS {
                Err(Error::Domain(DomainError::NotEnoughRounds(
                    amount_of_rounds.into(),
                    Game::MINIMUM_ROUNDS.into(),
                )))
            } else if self.get_connected_players().len() < Game::MINIMUM_PLAYERS.into() {
                Err(Error::Domain(DomainError::NotEnoughPlayers(
                    self.get_connected_players().len(),
                    Game::MINIMUM_PLAYERS.into(),
                )))
            } else {
                self.amount_of_rounds = Some(amount_of_rounds);
                self.process_event(&GameFsmInput::StartGame)
            }
        } else {
            Err(Error::Domain(DomainError::NonHostPlayerCannotStartGame(
                nickname.to_string(),
            )))
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
                    if self.rounds.len()
                        >= self.amount_of_rounds.unwrap_or(Game::DEFAULT_ROUNDS).into()
                    {
                        self.process_event(&GameFsmInput::NoMoreRounds)
                    } else {
                        self.start_new_round();
                        self.process_event(&GameFsmInput::StartRound)
                    }
                }
                GameFsmState::PlayersSubmittingWords => Ok(()),
                GameFsmState::Lobby => Ok(()),
                GameFsmState::PlayersSubmittingVotingWord => Ok(()),
                GameFsmState::ChooseNextVotingItem => {
                    if self.get_current_round_mut().next_voting_item().is_some() {
                        self.process_event(&GameFsmInput::NextVotingItem)
                    } else {
                        self.process_event(&GameFsmInput::NoMoreVotingItems)
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

    fn start_new_round(&mut self) {
        let word = self.choose_random_word();
        let round = Round::new(
            &word,
            self.players()
                .iter()
                .map(|player| player.nickname.clone())
                .collect(),
        );
        self.rounds.push(round);
    }

    fn choose_random_word(&mut self) -> String {
        match self.words.iter_mut().find(|word| !word.is_used) {
            Some(word) => {
                word.is_used = true;
                word.value.to_string()
            }
            None => {
                log::error!("Ran out of unused random words, resetting the used words. GameId: '{}', AmountOfWords: '{}', AmountOfRounds: '{}'", self.id, self.rounds.len(), self.words.len());
                self.words = Game::shuffle_words(
                    self.words
                        .iter()
                        .map(|word| word.value.to_string())
                        .collect(),
                );
                // This is a recursive call, it will always do just 1 recursive call as we ensure the game is constructed with at least 1 word
                self.choose_random_word()
            }
        }
    }

    pub fn set_player_voting_word(
        &mut self,
        nickname: &str,
        word: Option<String>,
    ) -> Result<(), Error> {
        // None if the player says they don't have that word on their list
        // Verify the player has this word
        // Verify the player hasn't already added this word as validated
        // If all players have sent something then compute the score and go to validate the next word
        if self.state() != &GameFsmState::PlayersSubmittingVotingWord {
            return Err(Error::Domain(
                DomainError::InvalidStateForVotingWordSubmission(
                    self.state().to_owned(),
                    GameFsmState::PlayersSubmittingVotingWord,
                ),
            ));
        }
        self.get_current_round_mut()
            .set_player_voting_word(nickname, word)
    }

    pub fn add_player_words(&mut self, nickname: &str, words: Vec<String>) -> Result<(), Error> {
        if self.fsm.state() != &GameFsmState::PlayersSubmittingWords {
            return Err(Error::Domain(DomainError::InvalidStateForWordsSubmission(
                self.fsm.state().to_owned(),
                GameFsmState::PlayersSubmittingWords,
            )));
        }

        let round = self
            .rounds
            .last_mut()
            .expect("Missing round, there is a bug in the code.");
        round.add_player_words(nickname, words)?;

        self.check_transition_to_voting()
    }

    fn check_transition_to_voting(&mut self) -> Result<(), Error> {
        if self.fsm.state() == &GameFsmState::PlayersSubmittingWords {
            let round = self
                .rounds
                .last_mut()
                .expect("Missing round, there is a bug in the code.");
            let connected_players: Vec<String> = self
                .players
                .iter()
                .filter(|player| player.is_connected)
                .map(|player| player.nickname.clone())
                .collect();
            if round.have_all_players_submitted_words(&connected_players) {
                for disconnected_player in self.players.iter().filter(|player| !player.is_connected)
                {
                    round.add_player_words(&disconnected_player.nickname, Vec::default())?;
                }
                return self.process_event(&GameFsmInput::AllPlayersSubmittedWords);
            }
        }
        Ok(())
    }

    pub fn accept_players_voting_words(&mut self, nickname: &str) -> Result<(), Error> {
        if self.is_host(nickname) {
            self.get_current_round_mut().compute_score();
            self.process_event(&GameFsmInput::AcceptPlayersVotingWords)
        } else {
            Err(Error::Domain(
                DomainError::NonHostPlayerCannotContinueToNextVotingItem(nickname.to_string()),
            ))
        }
    }

    pub fn continue_to_next_round(&mut self, nickname: &str) -> Result<(), Error> {
        if self.is_host(nickname) {
            self.process_event(&GameFsmInput::ContinueToNextRound)
        } else {
            Err(Error::Domain(
                DomainError::NonHostPlayerCannotContinueToNextRound(nickname.to_string()),
            ))
        }
    }

    pub fn play_again(&mut self, nickname: &str) -> Result<(), Error> {
        if self.is_host(nickname) {
            self.process_event(&GameFsmInput::PlayAgain)?;
            self.amount_of_rounds = None;
            self.rounds = Vec::default();
            Ok(())
        } else {
            Err(Error::Domain(
                DomainError::NonHostPlayerCannotSendPlayAgain(nickname.to_string()),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::Game;
    use crate::{
        error::{domain_error::DomainError, Error},
        game::game_fsm::GameFsmState,
    };

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
        let mut game = get_empty_game();

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

        let result = game.disconnect_player("non_existent_player");

        assert_eq!(
            result,
            Err(Error::Internal(
                "Tried to disconnect player 'non_existent_player' but it does not exist."
                    .to_string()
            ))
        );
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
        let mut game = get_empty_game();
        game.add_player(PLAYER_1).unwrap();

        let result = game.start_game(PLAYER_1, 3);

        assert_eq!(
            result,
            Err(Error::Domain(DomainError::NotEnoughPlayers(1, 3)))
        );
    }

    #[test]
    fn game_cannot_be_started_with_less_than_one_round() {
        let mut game = get_empty_game();
        game.add_player(PLAYER_1).unwrap();

        let result = game.start_game(PLAYER_1, 0);

        assert_eq!(
            result,
            Err(Error::Domain(DomainError::NotEnoughRounds(0, 1)))
        );
    }

    #[test]
    fn host_player_can_start_game() {
        let mut game = get_game(&GameFsmState::Lobby);

        let result = game.start_game(PLAYER_1, 3);

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn non_host_player_cannot_start_game() {
        let mut game = get_game(&GameFsmState::Lobby);

        let result = game.start_game(PLAYER_2, 3);

        assert_eq!(
            result,
            Err(Error::Domain(DomainError::NonHostPlayerCannotStartGame(
                PLAYER_2.to_string()
            )))
        );
    }

    #[test]
    fn game_starts_in_lobby() {
        let game = get_empty_game();

        assert_eq!(game.state(), &GameFsmState::Lobby);
    }

    #[test]
    fn game_initializes_first_round() {
        let mut game = get_game(&GameFsmState::Lobby);

        game.start_game(PLAYER_1, 3).unwrap();

        assert_eq!(game.state(), &GameFsmState::PlayersSubmittingWords);
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
        let game = get_empty_game();

        assert!(game.all_players_are_disconnected());
    }

    #[test]
    fn add_player_words_works() {
        let mut game = get_game(&GameFsmState::PlayersSubmittingWords);

        let result = game.add_player_words(PLAYER_1, words());

        assert_eq!(result, Ok(()));
        assert_eq!(game.state(), &GameFsmState::PlayersSubmittingWords);
    }

    #[test]
    fn add_player_words_transitions_to_players_submitting_voting_word_on_last_player_words() {
        let mut game = get_game(&GameFsmState::PlayersSubmittingWords);

        game.add_player_words(PLAYER_1, words()).unwrap();
        game.add_player_words(PLAYER_2, words()).unwrap();
        game.add_player_words(PLAYER_3, words()).unwrap();

        assert_eq!(game.state(), &GameFsmState::PlayersSubmittingVotingWord);
    }

    #[test]
    fn add_player_words_transitions_and_sets_default_words_for_disconnected_players() {
        let mut game = get_game(&GameFsmState::PlayersSubmittingWords);
        game.disconnect_player(PLAYER_2).unwrap();
        game.disconnect_player(PLAYER_3).unwrap();

        game.add_player_words(PLAYER_1, words()).unwrap();

        assert_eq!(game.state(), &GameFsmState::PlayersSubmittingVotingWord);
        let round = game.rounds().last().unwrap();
        assert!(round.player_words.contains_key(PLAYER_2));
        assert!(round.player_words[PLAYER_2].is_empty());
        assert!(round.player_words.contains_key(PLAYER_3));
        assert!(round.player_words[PLAYER_3].is_empty());
    }

    #[test]
    fn add_player_words_fails_when_state_is_not_players_submitting_words() {
        let mut game = get_game(&GameFsmState::Lobby);

        let result = game.add_player_words(PLAYER_1, words());

        assert_eq!(
            result,
            Err(Error::Domain(DomainError::InvalidStateForWordsSubmission(
                GameFsmState::Lobby,
                GameFsmState::PlayersSubmittingWords
            )))
        );
    }

    #[test]
    fn add_player_voting_word_works_with_valid_word() {
        let mut game = get_game(&GameFsmState::PlayersSubmittingVotingWord);

        let result = game.set_player_voting_word(PLAYER_2, Some(WORD_1.to_string()));

        assert_eq!(result, Ok(()));
        assert_eq!(game.state(), &GameFsmState::PlayersSubmittingVotingWord);
    }

    #[test]
    fn add_player_voting_word_works_with_empty_word() {
        let mut game = get_game(&GameFsmState::PlayersSubmittingVotingWord);

        let result = game.set_player_voting_word(PLAYER_2, None);

        assert_eq!(result, Ok(()));
        assert_eq!(game.state(), &GameFsmState::PlayersSubmittingVotingWord);
    }

    #[test]
    fn add_player_voting_word_fails_when_state_is_not_players_submitting_voting_word() {
        let mut game = get_game(&GameFsmState::PlayersSubmittingWords);

        let result = game.set_player_voting_word(PLAYER_2, Some(WORD_1.to_string()));

        assert_eq!(
            result,
            Err(Error::Domain(
                DomainError::InvalidStateForVotingWordSubmission(
                    GameFsmState::PlayersSubmittingWords,
                    GameFsmState::PlayersSubmittingVotingWord
                )
            ))
        );
    }

    #[test]
    fn when_last_player_without_words_submission_is_disonnected_games_proceeds_to_voting() {
        let mut game = get_game(&GameFsmState::PlayersSubmittingWords);
        game.add_player_words(PLAYER_1, words()).unwrap();
        game.add_player_words(PLAYER_2, words()).unwrap();
        assert_eq!(game.state(), &GameFsmState::PlayersSubmittingWords);

        game.disconnect_player(PLAYER_3).unwrap();

        assert_eq!(game.state(), &GameFsmState::PlayersSubmittingVotingWord);
    }

    #[test]
    fn continue_to_next_round_fails_when_state_is_not_end_of_round() {
        let mut game = get_game(&GameFsmState::Lobby);

        let result = game.continue_to_next_round(PLAYER_1);

        assert_eq!(result, Err(Error::Internal("The fsm in state Lobby can't transition with an event ContinueToNextRound. Error: 'cannot perform a state transition from the current state with the provided input'.".to_string())));
    }

    #[test]
    fn continue_to_next_round_fails_when_player_is_not_host() {
        let mut game = get_game(&GameFsmState::EndOfRound);

        let result = game.continue_to_next_round(PLAYER_2);

        assert_eq!(
            result,
            Err(Error::Domain(
                DomainError::NonHostPlayerCannotContinueToNextRound(PLAYER_2.to_string())
            ))
        );
    }

    #[test]
    fn continue_to_next_round_proceeds_to_next_round() {
        let mut game = get_game(&GameFsmState::EndOfRound);

        let result = game.continue_to_next_round(PLAYER_1);

        assert_eq!(result, Ok(()));
        assert_eq!(game.state(), &GameFsmState::PlayersSubmittingWords);
    }

    #[test]
    fn continue_to_next_round_proceeds_to_end_of_game() {
        let mut game = get_game_with_rounds(&GameFsmState::PlayersSubmittingWords, 3);
        for _ in 0..game.amount_of_rounds.unwrap() {
            complete_round(&mut game);
            game.continue_to_next_round(PLAYER_1).unwrap();
        }
        assert_eq!(game.state(), &GameFsmState::EndOfGame);

        let mut game = get_game_with_rounds(&GameFsmState::PlayersSubmittingWords, 6);
        for _ in 0..game.amount_of_rounds.unwrap() {
            complete_round(&mut game);
            game.continue_to_next_round(PLAYER_1).unwrap();
        }
        assert_eq!(game.state(), &GameFsmState::EndOfGame);
    }

    #[test]
    fn continue_to_next_voting_item_fails_when_state_is_not_players_submitting_voting_word() {
        let mut game = get_game(&GameFsmState::PlayersSubmittingWords);

        let result = game.accept_players_voting_words(PLAYER_1);

        assert_eq!(result, Err(Error::Internal("The fsm in state PlayersSubmittingWords can't transition with an event AcceptPlayersVotingWords. Error: 'cannot perform a state transition from the current state with the provided input'.".to_string())));
    }

    #[test]
    fn continue_to_next_voting_item_fails_when_player_is_not_host() {
        let mut game = get_game(&GameFsmState::PlayersSubmittingVotingWord);

        let result = game.accept_players_voting_words(PLAYER_2);

        assert_eq!(
            result,
            Err(Error::Domain(
                DomainError::NonHostPlayerCannotContinueToNextVotingItem(PLAYER_2.to_string())
            ))
        );
    }

    #[test]
    fn continue_to_next_round_proceeds_to_next_voting_item() {
        let mut game = get_game(&GameFsmState::PlayersSubmittingVotingWord);

        let result = game.accept_players_voting_words(PLAYER_1);

        assert_eq!(result, Ok(()));
        assert_eq!(game.state(), &GameFsmState::PlayersSubmittingVotingWord);
    }

    #[test]
    fn continue_to_next_round_proceeds_to_end_of_round_when_last_voting_item() {
        let mut game = get_game(&GameFsmState::PlayersSubmittingWords);

        complete_round(&mut game);

        assert_eq!(game.state(), &GameFsmState::EndOfRound);
    }

    #[test]
    fn new_players_cannot_be_added_after_game_is_started() {
        let mut game = get_game(&GameFsmState::PlayersSubmittingWords);

        let result = game.add_player("new_player");

        assert_eq!(
            result,
            Err(Error::Domain(DomainError::GameAlreadyInProgress(game.id)))
        );
    }

    #[test]
    fn existing_player_can_rejoin_after_game_is_started() {
        let mut game = get_game(&GameFsmState::PlayersSubmittingWords);

        let _ = game.disconnect_player(PLAYER_2);

        let result = game.add_player(PLAYER_2);

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn next_round_chooses_different_word() {
        let amount_of_rounds: u8 = Game::default_words().len().try_into().unwrap();
        let mut used_words: HashSet<String> = HashSet::new();
        let mut game =
            get_game_with_rounds(&GameFsmState::PlayersSubmittingWords, amount_of_rounds);
        for _ in 0..amount_of_rounds {
            let round = game.rounds().last().unwrap();
            assert!(!used_words.contains(&round.word));
            used_words.insert(round.word.to_string());
            complete_round(&mut game);
            game.continue_to_next_round(PLAYER_1).unwrap();
        }
        assert_eq!(game.state(), &GameFsmState::EndOfGame);
    }

    #[test]
    fn word_is_repeated_when_more_rounds_than_words() {
        let amount_of_rounds: u8 = (Game::default_words().len() + 1).try_into().unwrap();
        let mut used_words: HashSet<String> = HashSet::new();
        let mut game =
            get_game_with_rounds(&GameFsmState::PlayersSubmittingWords, amount_of_rounds);
        for round_index in 0..amount_of_rounds {
            let round = game.rounds().last().unwrap();
            if round_index == amount_of_rounds - 1 {
                assert!(used_words.contains(&round.word));
            }
            used_words.insert(round.word.to_string());
            complete_round(&mut game);
            game.continue_to_next_round(PLAYER_1).unwrap();
        }
        assert_eq!(game.state(), &GameFsmState::EndOfGame);
    }

    #[test]
    fn different_games_choose_words_in_different_order() {
        let amount_of_rounds: u8 = Game::default_words().len().try_into().unwrap();

        let mut game_1_words: Vec<String> = vec![];
        let mut game_1 =
            get_game_with_rounds(&GameFsmState::PlayersSubmittingWords, amount_of_rounds);
        for _ in 0..amount_of_rounds {
            let round = game_1.rounds().last().unwrap();
            game_1_words.push(round.word.to_string());
            complete_round(&mut game_1);
            game_1.continue_to_next_round(PLAYER_1).unwrap();
        }

        let mut game_2_words: Vec<String> = vec![];
        let mut game_2 =
            get_game_with_rounds(&GameFsmState::PlayersSubmittingWords, amount_of_rounds);
        for _ in 0..amount_of_rounds {
            let round = game_2.rounds().last().unwrap();
            game_2_words.push(round.word.to_string());
            complete_round(&mut game_2);
            game_2.continue_to_next_round(PLAYER_1).unwrap();
        }

        assert_eq!(game_1_words.len(), game_2_words.len());
        // This unit test is not deterministic, as we suffle words in a random way every time we construct a new game,
        // and we do not provide any seed. The chance of two games ending up with the same word order is pretty small,
        // so I think we can assume this won't happen often, we can always increase the amount of words for the tests
        // to further decrease the chances
        assert!((0..game_1_words.len())
            .any(|word_index| game_1_words[word_index] != game_2_words[word_index]));
    }

    #[test]
    fn play_again_fails_when_player_is_not_host() {
        let mut game = get_game(&GameFsmState::EndOfGame);

        let result = game.play_again(PLAYER_2);

        assert_eq!(
            result,
            Err(Error::Domain(
                DomainError::NonHostPlayerCannotSendPlayAgain(PLAYER_2.to_string())
            ))
        );
    }

    #[test]
    fn play_again_proceeds_to_lobby() {
        let mut game = get_game(&GameFsmState::EndOfGame);
        let expected_used_words = game.words.iter().filter(|word| word.is_used).count();

        let result = game.play_again(PLAYER_1);
        let actual_used_words = game.words.iter().filter(|word| word.is_used).count();

        assert_eq!(result, Ok(()));
        assert_eq!(game.state(), &GameFsmState::Lobby);
        assert_eq!(actual_used_words, expected_used_words);
        assert!(game.rounds().is_empty());
        assert!(game.amount_of_rounds.is_none());
    }

    fn get_empty_game() -> Game {
        Game::new("id", Game::default_words())
    }

    fn get_game(state: &GameFsmState) -> Game {
        get_game_with_rounds(state, 3)
    }

    fn get_game_with_rounds(state: &GameFsmState, amount_of_rounds: u8) -> Game {
        let mut game = get_empty_game();
        game.add_player(PLAYER_1).unwrap();
        game.add_player(PLAYER_2).unwrap();
        game.add_player(PLAYER_3).unwrap();

        match state {
            GameFsmState::Lobby => {}
            GameFsmState::PlayersSubmittingWords => {
                game.start_game(PLAYER_1, amount_of_rounds).unwrap()
            }
            GameFsmState::PlayersSubmittingVotingWord => {
                game.start_game(PLAYER_1, 3).unwrap();
                send_players_words(&mut game);
            }
            GameFsmState::EndOfRound => {
                game.start_game(PLAYER_1, amount_of_rounds).unwrap();
                complete_round(&mut game);
            }
            GameFsmState::EndOfGame => {
                game.start_game(PLAYER_1, amount_of_rounds).unwrap();
                for _ in 0..game.amount_of_rounds.unwrap() {
                    complete_round(&mut game);
                    game.continue_to_next_round(PLAYER_1).unwrap();
                }
            }
            _ => panic!("Unsupported desired state for unit tests"),
        }
        assert_eq!(game.state(), state);

        game
    }

    fn send_players_words(game: &mut Game) {
        for player in players() {
            game.add_player_words(&player, words()).unwrap();
        }
    }

    fn complete_round(game: &mut Game) {
        send_players_words(game);
        for word in words() {
            for player in players() {
                // For simplicity in the test setup, we'll iterate over all the words, even if they are already used, ignore such error
                let _ = game.set_player_voting_word(&player, Some(word.to_string()));
            }
            game.accept_players_voting_words(PLAYER_1).unwrap();
        }
    }
}
