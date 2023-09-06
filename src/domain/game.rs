use serde::Serialize;

use super::player::Player;

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

    pub fn add_player(&mut self, player: Player) {
        self.players.push(player);
    }

    pub fn remove_player(&mut self, nickname: &str) -> Option<Player> {
        if let Some(index) = self.players.iter().position(|x| x.nickname == nickname) {
            Some(self.players.remove(index))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Game;
    use crate::domain::player::Player;

    #[test]
    fn add_player_works() {
        let mut game = Game::new();
        let player = Player::new("any-player");

        game.add_player(player.clone());

        assert_eq!(game.players().len(), 1);
        assert_eq!(game.players().first().unwrap(), &player);
    }

    #[test]
    fn remove_player_works() {
        let mut game = Game::new();
        let player = Player::new("any-player");
        let other_player = Player::new("other-player");

        game.add_player(player.clone());
        game.add_player(other_player.clone());

        assert_eq!(game.players().len(), 2);

        let removed = game.remove_player(&player.nickname).unwrap();

        assert_eq!(game.players().len(), 1);
        assert_eq!(game.players().first().unwrap(), &other_player);
        assert_eq!(removed, player);
    }

    #[test]
    fn remove_non_existing() {
        let mut game = Game::new();

        let removed = game.remove_player("any");

        assert_eq!(removed, None);
    }
}
