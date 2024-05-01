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
    pub amount_of_rounds: Option<u8>,
}

const NON_HOST_PLAYER_CANNOT_START_GAME: &str = "Only the host player can start the game";
const GAME_CANNOT_BE_STARTED_WITH_LESS_THAN_ONE_ROUND: &str =
    "The game cannot be started with less than 1 round";
const INVALID_STATE_FOR_WORDS_SUBMISSION: &str = "The player can only send their words submission when the game is on the PlayersSubmittingWords state";
const INVALID_STATE_FOR_VOTING_WORD_SUBMISSION: &str = "The player can only send their voting word when the game is on the PlayersSubmittingVotingWord state";
const NON_HOST_PLAYER_CANNOT_CONTINUE_TO_NEXT_ROUND: &str =
    "Only the host player can continue to the next round";
const NON_HOST_PLAYER_CANNOT_CONTINUE_TO_NEXT_VOTING_ITEM: &str =
    "Only the host player can continue to the next voting item";

impl Game {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            fsm: StateMachine::default(),
            players: Vec::default(),
            rounds: Vec::default(),
            amount_of_rounds: None,
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

    pub fn start_game(&mut self, nickname: &str, amount_of_rounds: u8) -> Result<(), Error> {
        if self.is_host(nickname) {
            if amount_of_rounds < 1 {
                Err(Error::CommandNotAllowed(
                    GAME_CANNOT_BE_STARTED_WITH_LESS_THAN_ONE_ROUND.to_owned(),
                ))
            } else if self.get_connected_players().len() < 3 {
                Err(Error::NotEnoughPlayers)
            } else {
                self.amount_of_rounds = Some(amount_of_rounds);
                self.process_event(&GameFsmInput::StartGame)
            }
        } else {
            Err(Error::CommandNotAllowed(
                NON_HOST_PLAYER_CANNOT_START_GAME.to_owned(),
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
                    if self.rounds.len()
                        >= self
                            .amount_of_rounds
                            .expect("The amount of rounds should be set in this state")
                            .into()
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
        let word = Game::choose_random_word();
        let round = Round::new(
            &word,
            self.players()
                .iter()
                .map(|player| player.nickname.clone())
                .collect(),
        );
        self.rounds.push(round);
    }

    fn choose_random_word() -> String {
        "alien".to_string()
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
            return Err(Error::CommandNotAllowed(
                INVALID_STATE_FOR_VOTING_WORD_SUBMISSION.to_owned(),
            ));
        }
        self.get_current_round_mut()
            .set_player_voting_word(nickname, word)
    }

    pub fn add_player_words(&mut self, nickname: &str, words: Vec<String>) -> Result<(), Error> {
        if self.fsm.state() != &GameFsmState::PlayersSubmittingWords {
            return Err(Error::CommandNotAllowed(
                INVALID_STATE_FOR_WORDS_SUBMISSION.to_owned(),
            ));
        }

        if let Some(round) = self.rounds.last_mut() {
            round.add_player_words(nickname, words)?;
            let connected_players: Vec<String> = self
                .players
                .iter()
                .filter(|player| player.is_connected)
                .map(|player| player.nickname.clone())
                .collect();
            if round.have_all_players_submitted_words(&connected_players) {
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
            Err(Error::CommandNotAllowed(
                NON_HOST_PLAYER_CANNOT_CONTINUE_TO_NEXT_VOTING_ITEM.to_owned(),
            ))
        }
    }

    pub fn continue_to_next_round(&mut self, nickname: &str) -> Result<(), Error> {
        if self.is_host(nickname) {
            self.process_event(&GameFsmInput::ContinueToNextRound)
        } else {
            Err(Error::CommandNotAllowed(
                NON_HOST_PLAYER_CANNOT_CONTINUE_TO_NEXT_ROUND.to_owned(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Game;
    use crate::{
        error::Error,
        game::{
            game_fsm::GameFsmState, INVALID_STATE_FOR_VOTING_WORD_SUBMISSION,
            INVALID_STATE_FOR_WORDS_SUBMISSION, NON_HOST_PLAYER_CANNOT_CONTINUE_TO_NEXT_ROUND,
            NON_HOST_PLAYER_CANNOT_CONTINUE_TO_NEXT_VOTING_ITEM, NON_HOST_PLAYER_CANNOT_START_GAME,
        },
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
        let mut game = Game::new("id");
        game.add_player(PLAYER_1).unwrap();

        let result = game.start_game(PLAYER_1, 3);

        assert_eq!(result, Err(Error::NotEnoughPlayers));
    }

    #[test]
    fn game_cannot_be_started_with_less_than_one_round() {
        let mut game = Game::new("id");
        game.add_player(PLAYER_1).unwrap();

        let result = game.start_game(PLAYER_1, 0);

        assert_eq!(
            result,
            Err(Error::CommandNotAllowed(
                "The game cannot be started with less than 1 round".to_string()
            ))
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
            Err(Error::CommandNotAllowed(
                NON_HOST_PLAYER_CANNOT_START_GAME.to_owned()
            ))
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
        let game = Game::new("id");

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
    fn add_player_words_fails_when_state_is_not_players_submitting_words() {
        let mut game = get_game(&GameFsmState::Lobby);

        let result = game.add_player_words(PLAYER_1, words());

        assert_eq!(
            result,
            Err(Error::CommandNotAllowed(
                INVALID_STATE_FOR_WORDS_SUBMISSION.to_owned()
            ))
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
            Err(Error::CommandNotAllowed(
                INVALID_STATE_FOR_VOTING_WORD_SUBMISSION.to_owned()
            ))
        );
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
            Err(Error::CommandNotAllowed(
                NON_HOST_PLAYER_CANNOT_CONTINUE_TO_NEXT_ROUND.to_owned()
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
            Err(Error::CommandNotAllowed(
                NON_HOST_PLAYER_CANNOT_CONTINUE_TO_NEXT_VOTING_ITEM.to_owned()
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

        assert_eq!(result, Err(Error::GameAlreadyInProgress));
    }

    #[test]
    fn existing_player_can_rejoin_after_game_is_started() {
        let mut game = get_game(&GameFsmState::PlayersSubmittingWords);

        let _ = game.disconnect_player(PLAYER_2);

        let result = game.add_player(PLAYER_2);

        assert_eq!(result, Ok(()));
    }

    fn get_game(state: &GameFsmState) -> Game {
        get_game_with_rounds(state, 3)
    }

    fn get_game_with_rounds(state: &GameFsmState, amount_of_rounds: u8) -> Game {
        let mut game = Game::new("id");
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
