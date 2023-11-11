use rand::distributions::{Alphanumeric, DistString};
use std::collections::HashMap;

use crate::actor::game::client::GameClient;
use crate::actor::game::GameActor;

#[derive(Default)]
pub struct GameFactory {
    game_channels: HashMap<String, GameClient>,
}

impl GameFactory {
    pub fn create_new_game(&mut self) -> String {
        let id = self.create_unique_game_id();
        self.game_channels.insert(id.clone(), GameActor::spawn());

        id
    }

    pub fn get_game(&self, game_id: &str) -> Option<&GameClient> {
        self.game_channels.get(game_id)
    }

    fn create_unique_game_id(&self) -> String {
        loop {
            let id = Alphanumeric
                .sample_string(&mut rand::thread_rng(), 5)
                .replace('O', "P")
                .replace('0', "1")
                .replace('I', "J")
                .replace('l', "m");
            if !self.game_channels.contains_key(&id) {
                return id;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GameFactory;

    #[test]
    fn add_player_works() {
        let game_factory = GameFactory::new();

        let id = game_factory.create_unique_game_id();

        assert_eq!(id.len(), 5);
        for char in id.chars() {
            assert!(
                ('0'..='9').contains(&char)
                    || ('A'..='Z').contains(&char)
                    || ('a'..='z').contains(&char)
            )
        }
    }
}
