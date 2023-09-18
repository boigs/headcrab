use crate::domain::game_factory::game_factory::GameManager;
use tokio::sync::mpsc::Receiver;

use crate::domain::game_factory::message::GameFactoryCommand;
use crate::domain::game_factory::message::GameFactoryResponse::*;

pub async fn actor_handler(mut rx: Receiver<GameFactoryCommand>) {
    let mut game_manager = GameManager::new();
    println!("game manager logic");
    while let Some(message) = rx.recv().await {
        if let GameFactoryCommand::CreateGame { response_channel } = message {
            println!("Received CreateGame Message");
            let game_id = game_manager.create_new_game();
            let game_created = GameCreated { game_id };
            response_channel.send(game_created).unwrap();
        }
    }
}
