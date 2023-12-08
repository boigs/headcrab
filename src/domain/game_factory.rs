use rand::distributions::{Alphanumeric, DistString};
use std::collections::HashMap;
use std::time::Duration;

use crate::actor::game::client::GameClient;
use crate::actor::game::GameActor;

use crate::actor::game_factory::client::GameFactoryClient;
use crate::domain::error::Error;

pub struct GameFactory {
    game_channels: HashMap<String, GameClient>,
    game_inactivity_timeout: Duration,
}

impl GameFactory {
    pub fn new(game_inactivity_timeout: Duration) -> Self {
        GameFactory {
            game_channels: HashMap::default(),
            game_inactivity_timeout,
        }
    }

    pub fn create_new_game(&mut self, game_factory: GameFactoryClient) -> String {
        let id = self.create_unique_game_id();
        self.game_channels.insert(
            id.clone(),
            GameActor::spawn(&id, self.game_inactivity_timeout, game_factory),
        );

        id
    }

    pub fn remove_game(&mut self, game_id: &str) -> Option<GameClient> {
        self.game_channels.remove(game_id)
    }

    pub fn get_game(&self, game_id: &str) -> Result<&GameClient, Error> {
        match self.game_channels.get(game_id) {
            Some(game) => Ok(game),
            None => Err(Error::GameDoesNotExist(game_id.to_string())),
        }
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
    use std::time::Duration;

    use super::GameFactory;

    #[test]
    fn add_player_works() {
        let game_factory = GameFactory::new(Duration::from_secs(1));

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
