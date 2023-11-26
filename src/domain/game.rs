use crate::domain::game_fsm::{GameFsm, GameFsmInput};
use crate::domain::player::Player;

use crate::domain::error::Error;
use rust_fsm::StateMachine;

use super::game_fsm::GameFsmState;

pub struct Game {
    id: String,
    fsm: StateMachine<GameFsm>,
    players: Vec<Player>,
}

impl Game {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            fsm: StateMachine::default(),
            players: Vec::default(),
        }
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn state(&self) -> &GameFsmState {
        self.fsm.state()
    }

    pub fn players(&self) -> &[Player] {
        &self.players
    }

    pub fn add_player(&mut self, nickname: &str) -> Result<(), Error> {
        if self
            .players
            .iter()
            .any(|player| nickname == player.nickname)
        {
            Err(Error::PlayerAlreadyExists(nickname.to_string()))
        } else {
            let new_player = Player::new(nickname);
            self.players.push(new_player);
            self.assign_host();
            Ok(())
        }
    }

    pub fn remove_player(&mut self, nickname: &str) -> Option<Player> {
        if let Some(index) = self.players.iter().position(|x| x.nickname == nickname) {
            let removed_player = self.players.remove(index);
            self.assign_host();
            Some(removed_player)
        } else {
            None
        }
    }

    pub fn start_game(&mut self, nickname: &str) {
        if self.is_host(nickname) {
            self.process_event(&GameFsmInput::StartGame)
        }
    }

    fn assign_host(&mut self) {
        if self.players.iter().all(|player| !player.is_host) {
            if let Some(first_player) = self.players.first_mut() {
                first_player.is_host = true;
            }
        }
    }

    fn is_host(&self, nickname: &str) -> bool {
        self.players
            .iter()
            .find(|player| player.nickname == nickname)
            .map(|player| player.is_host)
            .unwrap_or(false)
    }

    fn process_event(&mut self, event: &GameFsmInput) {
        if let Err(error) = self.fsm.consume(event) {
            log::error!(
                "The fsm in state {:?} can't transition with an event {:?}. Error: '{error}'.",
                self.fsm.state(),
                event
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Game;

    #[test]
    fn add_player_works() {
        let mut game = Game::new("id");

        let _ = game.add_player("player");

        assert_eq!(game.players().len(), 1);
        assert_eq!(game.players()[0].nickname, "player");
    }

    #[test]
    fn remove_player_works() {
        let mut game = Game::new("id");

        let _ = game.add_player("any-player");
        let _ = game.add_player("other-player");

        assert_eq!(game.players().len(), 2);

        let removed = game
            .remove_player("any-player")
            .expect("No player has been removed.");

        assert_eq!(game.players().len(), 1);
        assert_eq!(game.players()[0].nickname, "other-player");
        assert_eq!(removed.nickname, "any-player");
    }

    #[test]
    fn remove_non_existing() {
        let mut game = Game::new("id");

        let removed = game.remove_player("player");

        assert_eq!(removed, None);
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
    fn host_player_is_reelected_when_removed() {
        let mut game = Game::new("id");

        let _ = game.add_player("first_player");
        let _ = game.add_player("second_player");
        let _ = game.remove_player("first_player");

        assert!(game.players()[0].is_host);
    }
}
