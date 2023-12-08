use crate::domain::game_fsm::{GameFsm, GameFsmInput};
use crate::domain::player::Player;

use crate::domain::error::Error;
use crate::domain::game_fsm::GameFsmState;
use crate::domain::round::Round;

use rust_fsm::StateMachine;

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

    pub fn add_player(&mut self, nickname: &str) -> Result<(), Error> {
        if let Some(player) = self.get_player_mut(nickname) {
            if player.is_connected {
                return Err(Error::PlayerAlreadyExists(nickname.to_string()));
            } else {
                player.is_connected = true;
            }
        } else {
            let new_player = Player::new(nickname);
            self.players.push(new_player);
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
            self.process_event(&GameFsmInput::StartGame)
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

    fn process_event(&mut self, event: &GameFsmInput) -> Result<(), Error> {
        match self.fsm.consume(event) {
            Ok(_) => match self.fsm.state() {
                GameFsmState::CreatingNewRound => {
                    self.start_new_round();
                    self.process_event(&GameFsmInput::StartRound)
                }
                GameFsmState::PlayersWritingWords => Ok(()),
                GameFsmState::Lobby => Ok(()),
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
        let round = Round::new(&word);
        self.rounds.push(round);
    }

    fn choose_random_word() -> String {
        "alien".to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::game_fsm::GameFsmState;

    use super::Game;

    #[test]
    fn add_player_works() {
        let mut game = Game::new("id");

        let _ = game.add_player("player");

        assert_eq!(game.players().len(), 1);
        assert_eq!(game.players()[0].nickname, "player");
    }

    #[test]
    fn disconnect_player_works() {
        let mut game = Game::new("id");

        let _ = game.add_player("any-player");
        let _ = game.add_player("other-player");

        assert_eq!(game.players().len(), 2);
        assert!(game.players()[0].is_connected);
        assert!(game.players()[1].is_connected);

        game.disconnect_player("any-player")
            .expect("No player has been removed.");

        assert_eq!(game.players().len(), 2);
        assert!(!game.players()[0].is_connected);
        assert!(game.players()[1].is_connected);
    }

    #[test]
    fn disconnect_non_existing() {
        let mut game = Game::new("id");

        let removed = game.disconnect_player("player");

        assert!(removed.is_err());
    }

    #[test]
    fn only_first_player_added_is_host() {
        let mut game = Game::new("id");

        let _ = game.add_player("first_player");
        let _ = game.add_player("second_player");

        assert!(game.players()[0].is_host);
        assert!(!game.players()[1].is_host);
    }

    #[test]
    fn host_player_is_reelected_when_disconnected() {
        let mut game = Game::new("id");

        let _ = game.add_player("first_player");
        let _ = game.add_player("second_player");
        let _ = game.disconnect_player("first_player");

        assert!(!game.players()[0].is_host);
        assert!(game.players()[1].is_host);
    }

    #[test]
    fn host_player_can_start_game() {
        let mut game = Game::new("id");

        let _ = game.add_player("first_player");
        let _ = game.add_player("second_player");

        assert!(game.start_game("first_player").is_ok());
    }

    #[test]
    fn non_host_player_cannot_start_game() {
        let mut game = Game::new("id");

        let _ = game.add_player("first_player");
        let _ = game.add_player("second_player");

        assert!(game.start_game("second_player").is_err());
    }

    #[test]
    fn round_is_initialized() {
        let mut game = Game::new("id");

        let _ = game.add_player("first_player");
        let _ = game.add_player("second_player");

        assert!(game.start_game("second_player").is_err());
    }

    #[test]
    fn game_starts_in_lobby() {
        let game = Game::new("id");

        assert_eq!(game.state(), &GameFsmState::Lobby);
    }

    #[test]
    fn game_initializes_first_round() {
        let mut game = Game::new("id");
        let _ = game.add_player("first_player");
        let _ = game.start_game("first_player");

        assert_eq!(game.state(), &GameFsmState::PlayersWritingWords);
        assert_eq!(game.rounds().len(), 1);
        assert!(!game.rounds().first().unwrap().word.is_empty());
    }
}
