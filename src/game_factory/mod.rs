pub mod actor;
pub mod actor_client;

use rand::distributions::{Alphanumeric, DistString};
use std::collections::HashMap;

use crate::config::GameSettings;
use crate::error::domain_error::DomainError;
use crate::error::Error;
use crate::game::actor::GameActor;
use crate::game::actor_client::GameClient;
use crate::game_factory::actor_client::GameFactoryClient;

pub struct GameFactory {
    game_channels: HashMap<String, GameClient>,
    game_settings: GameSettings,
    words: Vec<String>,
}

impl GameFactory {
    pub fn new(game_settings: GameSettings, words: Vec<String>) -> Self {
        GameFactory {
            game_channels: HashMap::default(),
            game_settings,
            words,
        }
    }

    pub fn create_new_game(&mut self, game_factory: GameFactoryClient) -> String {
        let id = self.create_unique_game_id();
        self.game_channels.insert(
            id.clone(),
            GameActor::spawn(
                &id,
                self.game_settings.clone(),
                self.words.clone(),
                game_factory,
            ),
        );

        id
    }

    pub fn remove_game(&mut self, game_id: &str) -> Option<GameClient> {
        self.game_channels.remove(game_id)
    }

    pub fn get_game(&self, game_id: &str) -> Result<&GameClient, Error> {
        match self.game_channels.get(game_id) {
            Some(game) => Ok(game),
            None => Err(Error::Domain(DomainError::GameDoesNotExist(
                game_id.to_string(),
            ))),
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
    use crate::{
        config::GameSettings,
        error::{domain_error::DomainError, Error},
    };

    use super::GameFactory;

    #[test]
    fn add_player_works() {
        let game_factory = GameFactory::new(
            GameSettings {
                inactivity_timeout_seconds: 1,
            },
            vec![],
        );

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

    #[test]
    fn get_game_fails_when_game_does_not_exist() {
        let game_factory = GameFactory::new(
            GameSettings {
                inactivity_timeout_seconds: 1,
            },
            vec![],
        );

        let result = game_factory.get_game("invalid_game");

        assert_eq!(
            result.unwrap_err(),
            Error::Domain(DomainError::GameDoesNotExist("invalid_game".to_string()))
        );
    }
}
