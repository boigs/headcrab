pub mod client;

use crate::domain::error::Error;
use crate::domain::game_fsm::GameFsmState;
use crate::domain::round::Round;
use crate::domain::{game::Game, player::Player};
use tokio::sync::broadcast::error::SendError;
use tokio::sync::oneshot::Sender as OneshotSender;
use tokio::sync::{
    broadcast, mpsc,
    mpsc::{Receiver, Sender},
};

use self::client::GameClient;

use crate::actor::game_factory::client::GameFactoryClient;

pub struct GameActor {
    game: Game,
    game_rx: Receiver<GameCommand>,
    broadcast_tx: broadcast::Sender<GameWideEvent>,
    game_factory: GameFactoryClient,
}

impl GameActor {
    pub fn spawn(id: &str, game_factory: GameFactoryClient) -> GameClient {
        let game = Game::new(id);
        let (game_tx, game_rx): (Sender<GameCommand>, Receiver<GameCommand>) = mpsc::channel(128);
        let (broadcast_tx, _): (
            broadcast::Sender<GameWideEvent>,
            broadcast::Receiver<GameWideEvent>,
        ) = broadcast::channel(32);

        tokio::spawn(
            GameActor {
                game,
                game_rx,
                broadcast_tx,
                game_factory,
            }
            .start(),
        );

        GameClient { game_tx }
    }

    async fn start(mut self) {
        while let Some(command) = self.game_rx.recv().await {
            match command {
                GameCommand::AddPlayer {
                    nickname,
                    response_tx,
                } => {
                    match self.game.add_player(&nickname) {
                        Err(error) => {
                            if response_tx
                                .send(GameEvent::Error {
                                    error: error.clone(),
                                })
                                .is_err()
                            {
                                log::error!("Sent GameEvent::Error to Player but the response channel is closed.");
                            }
                        }
                        Ok(_) => {
                            if response_tx
                                .send(GameEvent::PlayerAdded {
                                    broadcast_rx: self.broadcast_tx.subscribe(),
                                })
                                .is_err()
                            {
                                log::error!("Sent GameEvent::PlayerAdded to Player but the response channel is closed. Removing the Player.");
                                let _ = self.game.disconnect_player(&nickname);
                                continue;
                            }

                            let _ = self.send_game_state();
                        }
                    };
                }
                GameCommand::DisconnectPlayer { nickname } => {
                    let _ = self.game.disconnect_player(&nickname);
                    let _ = self.send_game_state();
                }
                GameCommand::StartGame { nickname } => {
                    if let Err(Error::Internal(_)) = self.game.start_game(&nickname) {
                        return self.stop_game().await;
                    }
                    let _ = self.send_game_state();
                }
                GameCommand::AddChatMessage { sender, content } => {
                    let _ = self
                        .broadcast_tx
                        .send(GameWideEvent::ChatMessage { sender, content });
                }
            }
        }
    }

    fn send_game_state(&self) -> Result<usize, SendError<GameWideEvent>> {
        self.broadcast_tx.send(GameWideEvent::GameState {
            state: self.game.state().clone(),
            players: self.game.players().to_vec(),
            rounds: self.game.rounds().to_vec(),
        })
    }

    async fn stop_game(self) {
        let game_id = self.game.id();
        if let Err(error) = self.game_factory.remove_game(game_id).await {
            log::error!("The GameFactory channel is closed, can't remove the Game. GameId: '{game_id}', Error: '{error}'.");
        }
    }
}

enum GameCommand {
    AddPlayer {
        nickname: String,
        response_tx: OneshotSender<GameEvent>,
    },
    DisconnectPlayer {
        nickname: String,
    },
    StartGame {
        nickname: String,
    },
    AddChatMessage {
        sender: String,
        content: String,
    },
}

#[derive(Debug)]
enum GameEvent {
    PlayerAdded {
        broadcast_rx: broadcast::Receiver<GameWideEvent>,
    },
    Error {
        error: Error,
    },
}

#[derive(Clone, Debug)]
pub enum GameWideEvent {
    GameState {
        state: GameFsmState,
        players: Vec<Player>,
        rounds: Vec<Round>,
    },
    ChatMessage {
        sender: String,
        content: String,
    },
}
