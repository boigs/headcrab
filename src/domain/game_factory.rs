use rand::distributions::Alphanumeric;
use rand::Rng;
use std::collections::HashMap;

use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::{actor, actor::game::GameCommand};

pub struct GameFactory {
    game_channels: HashMap<String, Sender<GameCommand>>,
}

impl GameFactory {
    pub fn new() -> Self {
        GameFactory {
            game_channels: HashMap::new(),
        }
    }

    pub fn create_new_game(&mut self) -> String {
        let (tx, rx): (Sender<GameCommand>, Receiver<GameCommand>) = mpsc::channel(128);
        tokio::spawn(actor::game::handler(rx));
        let id = self.create_unique_game_id();
        self.game_channels.insert(id.clone(), tx);

        id
    }

    pub fn get_game(&self, game_id: &str) -> Option<&Sender<GameCommand>> {
        self.game_channels.get(game_id)
    }

    fn create_unique_game_id(&self) -> String {
        loop {
            let id: String = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(5)
                .map(char::from)
                .collect();
            if !self.game_channels.contains_key(&id) {
                return id;
            }
        }
    }
}
