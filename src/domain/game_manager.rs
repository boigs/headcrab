use std::collections::HashMap;
use tokio::sync::mpsc::Receiver;

use uuid::Uuid;

use crate::domain::game::Game;
use crate::domain::message::Message;
use crate::domain::message::Message::GameCreated;
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

    pub fn create_new_game(&mut self) -> String {
        let id = Uuid::new_v4().to_string();
        self.games.insert(id.clone(), Game::new());

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

pub async fn actor(mut rx: Receiver<Message>) {
    let mut game_manager = GameManager::new();
    println!("game manager logic");
    while let Some(message) = rx.recv().await {
        if let Message::CreateGame { sender } = message {
            println!("Received CreateGame Message");
            let game_id = game_manager.create_new_game();
            let game_created = GameCreated { game_id };
            sender.send(game_created).unwrap();
        }
    }
}
