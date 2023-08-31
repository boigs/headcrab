use serde::Serialize;
use uuid::Uuid;

use super::player::Player;

#[derive(Debug, Serialize, Clone)]
pub struct Lobby {
    players: Vec<Player>,
}

impl Lobby {
    pub fn new() -> Self {
        Lobby { players: vec![] }
    }

    pub fn players(&self) -> &[Player] {
        &self.players
    }

    pub fn add_player(&mut self, player: Player) {
        self.players.push(player);
    }

    pub fn remove_player(&mut self, id: &Uuid) -> Option<Player> {
        if let Some(index) = self.players.iter().position(|x| x.id() == id) {
            Some(self.players.remove(index))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Lobby;
    use crate::player::Player;

    #[test]
    fn add_player_works() {
        let mut lobby = Lobby::new();
        let player = Player::new("any-player");

        lobby.add_player(player.clone());

        assert_eq!(lobby.players().len(), 1);
        assert_eq!(lobby.players().first().unwrap(), &player);
    }

    #[test]
    fn remove_player_works() {
        let mut lobby = Lobby::new();
        let player = Player::new("any-player");
        let other_player = Player::new("other-player");

        lobby.add_player(player.clone());
        lobby.add_player(other_player.clone());

        assert_eq!(lobby.players().len(), 2);

        lobby.remove_player(player.id());

        assert_eq!(lobby.players().len(), 1);
        assert_eq!(lobby.players().first().unwrap(), &other_player);
    }
}
