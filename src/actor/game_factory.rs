pub mod client;

use std::fmt::{Display, Formatter};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::oneshot::Sender as OneshotSender;

use crate::actor::game::client::GameClient;
use crate::domain::game_factory::GameFactory;

use self::client::GameFactoryClient;

pub struct GameFactoryActor {
    game_factory: GameFactory,
    game_factory_rx: Receiver<GameFactoryCommand>,
}

impl GameFactoryActor {
    /// Runs the GameFactory Actor in background and returns a Client to communicate with it
    pub fn spawn() -> GameFactoryClient {
        let game_factory = GameFactory::new();
        let (game_factory_tx, game_factory_rx): (
            Sender<GameFactoryCommand>,
            Receiver<GameFactoryCommand>,
        ) = mpsc::channel(512);

        tokio::spawn(
            GameFactoryActor {
                game_factory,
                game_factory_rx,
            }
            .start(),
        );

        GameFactoryClient { game_factory_tx }
    }

    async fn start(mut self) {
        while let Some(message) = self.game_factory_rx.recv().await {
            match message {
                GameFactoryCommand::CreateGame { response_channel } => {
                    let game_id = self.game_factory.create_new_game();
                    let game_created = GameFactoryResponse::GameCreated { game_id };
                    response_channel.send(game_created).unwrap();
                }
                GameFactoryCommand::GetGameActor {
                    game_id,
                    response_channel,
                } => {
                    let response = self.game_factory.get_game(&game_id).map_or_else(
                        || GameFactoryResponse::GameNotFound,
                        |game| GameFactoryResponse::GameActor { game: game.clone() },
                    );
                    response_channel.send(response).unwrap();
                }
            }
        }
    }
}

#[derive(Debug)]
enum GameFactoryCommand {
    CreateGame {
        response_channel: OneshotSender<GameFactoryResponse>,
    },
    GetGameActor {
        game_id: String,
        response_channel: OneshotSender<GameFactoryResponse>,
    },
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
enum GameFactoryResponse {
    GameCreated { game_id: String },
    GameActor { game: GameClient },
    GameNotFound,
}

impl Display for GameFactoryResponse {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}",
            match self {
                GameFactoryResponse::GameCreated { game_id } =>
                    format!("GameCreated(game_id: {game_id})"),
                GameFactoryResponse::GameActor { game: _ } => "GameActor".to_string(),
                GameFactoryResponse::GameNotFound => "GameNotFound".to_string(),
            }
        )
    }
}
