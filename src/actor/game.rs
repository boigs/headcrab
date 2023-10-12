pub mod client;

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
        let game = Game::new();
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
                    player,
                    response_tx,
                } => {
                    match self.game.add_player(player.clone()) {
                        Err(_) => {
                            if response_tx.send(GameEvent::PlayerAlreadyExists).is_err() {
                                log::error!("Sent GameEvent::PlayerAlreadyExists to Player but the channel is closed.");
                            }
                        }
                        Ok(_) => {
                            if response_tx
                                .send(GameEvent::PlayerAdded {
                                    broadcast_rx: self.broadcast_tx.subscribe(),
                                })
                                .is_err()
                            {
                                log::error!("Sent GameEvent::PlayerAdded to Player but the channel is closed. Removing the Player.");
                                self.game.remove_player(&player.nickname);
                            } else if self.send_game_state().is_err() {
                                log::error!("Sent GameWideEvent::GameState to Broadcast but the channel is closed. Stopping the Game.");
                                return;
                            };
                        }
                    };
                }
                GameCommand::RemovePlayer { player } => {
                    self.game.remove_player(&player.nickname);
                    if self.game.players().is_empty() {
                        log::info!(
                            "Removed Player from the Game, no more Players, stopping the Game."
                        );
                        return;
                    }
                    if self.send_game_state().is_err() {
                        log::error!("There are no Players remaining listening to this game's broadcast messages but there are player objects in the game. Stopping the Game.");
                        return;
                    }
                }
            }
        }
    }

    fn send_game_state(&self) -> Result<usize, SendError<GameWideEvent>> {
        self.broadcast_tx.send(GameWideEvent::GameState {
            players: Vec::from_iter(self.game.players().iter().map(|player| (*player).clone())),
        })
    }
}

enum GameCommand {
    AddPlayer {
        player: Player,
        response_tx: OneshotSender<GameEvent>,
    },
    RemovePlayer {
        player: Player,
    },
}

#[derive(Debug)]
enum GameEvent {
    PlayerAdded {
        broadcast_rx: broadcast::Receiver<GameWideEvent>,
    },
    PlayerAlreadyExists,
}

#[derive(Clone, Debug)]
pub enum GameWideEvent {
    GameState { players: Vec<Player> },
}
