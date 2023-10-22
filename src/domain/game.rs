use crate::domain::player::Player;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct Game {
    players: Vec<Player>,
}

impl Game {
    pub fn new() -> Self {
        Game { players: vec![] }
    }

    pub fn players(&self) -> &[Player] {
        &self.players
    }

    pub fn add_player(&mut self, nickname: &str) -> Result<(), ()> {
        if self
            .players
            .iter()
            .any(|player| nickname == player.nickname)
        {
            Err(())
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

    fn assign_host(&mut self) {
        if !self.players.is_empty() && self.players.iter().all(|player| !player.is_host) {
            self.players.first_mut().unwrap().is_host = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Game;

    #[test]
    fn add_player_works() {
        let mut game = Game::new();

        let _ = game.add_player("player");

        assert_eq!(game.players().len(), 1);
        assert_eq!(game.players()[0].nickname, "player");
    }

    #[test]
    fn remove_player_works() {
        let mut game = Game::new();

        let _ = game.add_player("any-player");
        let _ = game.add_player("other-player");

        assert_eq!(game.players().len(), 2);

        let removed = game.remove_player("any-player").unwrap();

        assert_eq!(game.players().len(), 1);
        assert_eq!(game.players()[0].nickname, "other-player");
        assert_eq!(removed.nickname, "any-player");
    }

    #[test]
    fn remove_non_existing() {
        let mut game = Game::new();

        let removed = game.remove_player("player");

        assert_eq!(removed, None);
    }

    #[test]
    fn only_first_player_added_is_host() {
        let mut game = Game::new();

        let _ = game.add_player("first_player");
        let _ = game.add_player("second_player");

        assert!(game.players()[0].is_host);
        assert!(!game.players()[1].is_host);
    }

    #[test]
    fn host_player_is_reelected_when_removed() {
        let mut game = Game::new();

        let _ = game.add_player("first_player");
        let _ = game.add_player("second_player");
        let _ = game.remove_player("first_player");

        assert!(game.players()[0].is_host);
    }
}
