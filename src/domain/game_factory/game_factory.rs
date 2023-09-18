use std::collections::HashMap;

use tokio::sync::mpsc::{self, Receiver, Sender};
use uuid::Uuid;

use crate::domain::game::actor::actor_handler;
use crate::domain::game::message::GameCommand;

pub struct GameManager {
    game_channels: HashMap<String, Sender<GameCommand>>,
}

impl GameManager {
    pub fn new() -> Self {
        GameManager {
            game_channels: HashMap::new(),
        }
    }

    pub fn create_new_game(&mut self) -> String {
        let (tx, rx): (Sender<GameCommand>, Receiver<GameCommand>) = mpsc::channel(128);
        tokio::spawn(actor_handler(rx));
        let id = Uuid::new_v4().to_string();
        self.game_channels.insert(id.clone(), tx);

        id
    }

    pub fn get_game(&self, game_id: &str) -> Option<&Sender<GameCommand>> {
        self.game_channels.get(game_id)
    }
}
