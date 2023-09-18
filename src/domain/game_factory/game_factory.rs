use std::collections::HashMap;

use tokio::sync::mpsc::{self, Receiver, Sender};
use uuid::Uuid;

use crate::domain::game::{self, message::GameCommand};

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
        tokio::spawn(game::actor::handler(rx));
        let id = Uuid::new_v4().to_string();
        self.game_channels.insert(id.clone(), tx);

        id
    }

    pub fn get_game(&self, game_id: &str) -> Option<&Sender<GameCommand>> {
        self.game_channels.get(game_id)
    }
}
