use crate::domain::{game::Game, player::Player};
use tokio::sync::broadcast::error::SendError;
use tokio::sync::oneshot::{self, Receiver as OneshotReceiver, Sender as OneshotSender};
use tokio::sync::{
    broadcast, mpsc,
    mpsc::{Receiver, Sender},
};

#[derive(Clone, Debug)]
pub struct GameClient {
    game_tx: Sender<GameCommand>,
}

impl GameClient {
    pub async fn add_player(&self, player: Player) -> Result<GameWideEventReceiver, String> {
        let (tx, rx): (OneshotSender<GameEvent>, OneshotReceiver<GameEvent>) = oneshot::channel();

        if self
            .game_tx
            .send(GameCommand::AddPlayer {
                player: player.clone(),
                response_tx: tx,
            })
            .await
            .is_err()
        {
            // This line has been reached because:
            //  1. the game exists in the GameFactory actor
            //  2. but the (individual) Game actor has been dropped
            //  3. the user navigated to this game's URL in an attempt to re-join (and there aren't any other players in the game).
            // We need:
            //  1. When the game is closed (on game actor), delete the game from the game factory as well.
            //  2. send message through WS telling the client that this game does not exist.
            println!("ERROR: The Game is not alive. Can't add Player to Game.");
            return Err("ERROR: The Game is not alive. Can't add Player to Game.".to_string());
        }

        match rx.await {
            Ok(GameEvent::PlayerAdded { broadcast_rx }) => {
                Ok(GameWideEventReceiver { broadcast_rx })
            }
            Ok(GameEvent::PlayerAlreadyExists) => {
                Err("ERROR: The Player already exists.".to_string())
            }
            _ => {
                println!(
                    "ERROR: Player sent a GameCommand::AddPlayer to Game, but Game channel died."
                );
                Err(
                    "ERROR: Player sent a GameCommand::AddPlayer to Game, but Game channel died."
                        .to_string(),
                )
            }
        }
    }

    pub async fn remove_player(&self, player: Player) -> Result<(), String> {
        match self
            .game_tx
            .send(GameCommand::RemovePlayer { player })
            .await
        {
            Ok(_) => Ok(()),
            Err(error) => {
                println!("ERROR: Tried to send GameCommand:RemovePlayer but GameActor is not listening. Error: {error}.");
                Err(format!("ERROR: Tried to send GameCommand:RemovePlayer but GameActor is not listening. Error: {error}."))
            }
        }
    }
}

pub struct GameWideEventReceiver {
    broadcast_rx: broadcast::Receiver<GameWideEvent>,
}

impl GameWideEventReceiver {
    pub async fn next(&mut self) -> Result<GameWideEvent, String> {
        match self.broadcast_rx.recv().await {
            Ok(game_wide_event) => Ok(game_wide_event),
            Err(error) => {
                println!(
                    "ERROR: The broadcast channel with the Game has been closed. Error: {error}."
                );
                Err(format!(
                    "ERROR: The broadcast channel with the Game has been closed. Error: {error}."
                ))
            }
        }
    }
}

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
                                println!("ERROR: Sent GameEvent::PlayerAlreadyExists to Player but the channel is closed.");
                            }
                        }
                        Ok(_) => {
                            if response_tx
                                .send(GameEvent::PlayerAdded {
                                    broadcast_rx: self.broadcast_tx.subscribe(),
                                })
                                .is_err()
                            {
                                println!("ERROR: Sent GameEvent::PlayerAdded to Player but the channel is closed. Removing the Player.");
                                self.game.remove_player(&player.nickname);
                            } else if GameActor::send_game_state(&self.broadcast_tx, &self.game)
                                .is_err()
                            {
                                println!("ERROR: Sent GameWideEvent::GameState to Broadcast but the channel is closed. Stopping the Game.");
                                return;
                            };
                        }
                    };
                }
                GameCommand::RemovePlayer { player } => {
                    self.game.remove_player(&player.nickname);
                    if self.game.players().is_empty() {
                        println!(
                            "INFO: Removed Player from the Game, no more Players, stopping the Game."
                        );
                        return;
                    }
                    if GameActor::send_game_state(&self.broadcast_tx, &self.game).is_err() {
                        println!("ERROR: There are no Players remaining listening to this game's broadcast messages but there are player objects in the game. Stopping the Game.");
                        return;
                    }
                }
            }
        }
    }

    fn send_game_state(
        broadcast: &broadcast::Sender<GameWideEvent>,
        game: &Game,
    ) -> Result<usize, SendError<GameWideEvent>> {
        broadcast.send(GameWideEvent::GameState {
            players: Vec::from_iter(game.players().iter().map(|player| (*player).clone())),
        })
    }
}

pub enum GameCommand {
    AddPlayer {
        player: Player,
        response_tx: OneshotSender<GameEvent>,
    },
    RemovePlayer {
        player: Player,
    },
}

#[derive(Debug)]
pub enum GameEvent {
    PlayerAdded {
        broadcast_rx: broadcast::Receiver<GameWideEvent>,
    },
    PlayerAlreadyExists,
}

#[derive(Clone, Debug)]
pub enum GameWideEvent {
    GameState { players: Vec<Player> },
}
