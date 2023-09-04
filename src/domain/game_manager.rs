use std::collections::HashMap;

use uuid::Uuid;

use crate::domain::game::Game;
use crate::domain::player::Player;

pub struct GameManager {
    games: HashMap<String, Game>,
}

impl GameManager {
    pub fn new() -> Self {
        GameManager {
            games: HashMap::new(),
        }
    }

    pub fn create_new_game(&mut self, host_nickname: &str) -> String {
        let id = Uuid::new_v4().to_string();
        self.games.insert(id.clone(), Game::new());
        self.add_player(&id, host_nickname);

        id
    }

    pub fn get_game(&self, game_id: &str) -> Option<&Game> {
        self.games.get(game_id)
    }

    pub fn add_player(&mut self, game_id: &str, nickname: &str) -> String {
        let player = Player::new(nickname);
        let id = player.nickname.clone();
        self.games.get_mut(game_id).unwrap().add_player(player);

        id
    }

    pub fn remove_player(&mut self, game_id: &str, id: &str) -> Option<Player> {
        self.games.get_mut(game_id).unwrap().remove_player(id)
    }
}
