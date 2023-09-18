use crate::domain::game_factory::game_factory::GameManager;
use tokio::sync::mpsc::Receiver;

use crate::domain::game_factory::message::GameFactoryCommand::{self, *};
use crate::domain::game_factory::message::GameFactoryResponse::*;

pub async fn actor_handler(mut rx: Receiver<GameFactoryCommand>) {
    let mut game_manager = GameManager::new();

    while let Some(message) = rx.recv().await {
        match message {
            CreateGame { response_channel } => {
                let game_id = game_manager.create_new_game();
                let game_created = GameCreated { game_id };
                response_channel.send(game_created).unwrap();
            }
            GetGameActor {
                game_id,
                response_channel,
            } => {
                let response = game_manager.get_game(&game_id).map_or_else(
                    || GameNotFound,
                    |channel| GameActor {
                        game_channel: channel.clone(),
                    },
                );

                response_channel.send(response);
            }
        }
    }
}
