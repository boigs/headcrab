pub mod client;

use crate::domain::error::Error;
use crate::domain::game_fsm::GameFsmState;
use crate::domain::{game::Game, player::Player};
use tokio::sync::broadcast::error::SendError;
use tokio::sync::oneshot::Sender as OneshotSender;
use tokio::sync::{
    broadcast, mpsc,
    mpsc::{Receiver, Sender},
};

use self::client::GameClient;

pub struct GameActor {
    game: Game,
    game_rx: Receiver<GameCommand>,
    broadcast_tx: broadcast::Sender<GameWideEvent>,
}

impl GameActor {
    pub fn spawn() -> GameClient {
        let game = Game::default();
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
                                self.game.remove_player(&nickname);
                                continue;
                            }

                            if let Err(error) = self.send_game_state() {
                                log::error!("Sent GameWideEvent::GameState to Broadcast but the response channel is closed. Stopping the Game. Error: {error}");
                                return;
                            };
                        }
                    };
                }
                GameCommand::RemovePlayer { nickname } => {
                    let _ = self.game.remove_player(&nickname);
                    if self.game.players().is_empty() {
                        log::info!(
                            "Removed Player from the Game, no more Players, stopping the Game."
                        );
                        return;
                    }
                    if let Err(error) = self.send_game_state() {
                        log::error!("There are no Players remaining listening to this game's broadcast messages but there are player objects in the game. Stopping the Game. Error: '{error}'.");
                        return;
                    }
                }
                GameCommand::StartGame { nickname } => {
                    self.game.start_game(&nickname);
                    if let Err(error) = self.send_game_state() {
                        log::error!("There are no Players remaining listening to this game's broadcast messages but there are player objects in the game. Stopping the Game. Error: '{error}'.");
                        return;
                    }
                }
                GameCommand::AddChatMessage { sender, content } => {
                    if let Err(error) = self
                        .broadcast_tx
                        .send(GameWideEvent::ChatMessage { sender, content })
                    {
                        log::error!("There are no Players remaining listening to this game's broadcast messages but there are player objects in the game. Stopping the Game. Error: '{error}'.");
                        return;
                    }
                }
            }
        }
    }

    fn send_game_state(&self) -> Result<usize, SendError<GameWideEvent>> {
        self.broadcast_tx.send(GameWideEvent::GameState {
            state: self.game.state().clone(),
            players: Vec::from_iter(self.game.players().iter().map(|player| (*player).clone())),
        })
    }
}

enum GameCommand {
    AddPlayer {
        nickname: String,
        response_tx: OneshotSender<GameEvent>,
    },
    RemovePlayer {
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
    },
    ChatMessage {
        sender: String,
        content: String,
    },
}
