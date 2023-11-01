use tokio::sync::broadcast;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::{self, Receiver as OneshotReceiver, Sender as OneshotSender};

use super::{GameCommand, GameEvent, GameWideEvent};

#[derive(Clone, Debug)]
pub struct GameClient {
    pub(super) game_tx: Sender<GameCommand>,
}

impl GameClient {
    pub async fn add_player(&self, nickname: &str) -> Result<GameWideEventReceiver, String> {
        let (tx, rx): (OneshotSender<GameEvent>, OneshotReceiver<GameEvent>) = oneshot::channel();

        if self
            .game_tx
            .send(GameCommand::AddPlayer {
                nickname: nickname.to_string(),
                response_tx: tx,
            })
            .await
            .is_err()
        {
            // This line has been reached because:
            //  1. the game exists in the GameFactory actor
            //  2. but the (individual) Game actor has been dropped
            //  3. the user navigated to this game's URL in an attempt to re-join (and there aren't any other players in the game).
            log::error!("The Game is not alive. Can't add Player to Game.");
            return Err("The Game is not alive. Can't add Player to Game.".to_string());
        }

        match rx.await {
            Ok(GameEvent::PlayerAdded { broadcast_rx }) => {
                Ok(GameWideEventReceiver { broadcast_rx })
            }
            Ok(GameEvent::PlayerAlreadyExists) => Err("The Player already exists.".to_string()),
            _ => {
                log::error!("Player sent a GameCommand::AddPlayer to Game, but Game channel died.");
                Err(
                    "Player sent a GameCommand::AddPlayer to Game, but Game channel died."
                        .to_string(),
                )
            }
        }
    }

    pub async fn remove_player(&self, nickname: &str) -> Result<(), String> {
        match self
            .game_tx
            .send(GameCommand::RemovePlayer {
                nickname: nickname.to_string(),
            })
            .await
        {
            Ok(_) => Ok(()),
            Err(error) => {
                log::error!("Tried to send GameCommand:RemovePlayer but GameActor is not listening. Error: {error}.");
                Err(format!("Tried to send GameCommand:RemovePlayer but GameActor is not listening. Error: {error}."))
            }
        }
    }

    pub async fn start_game(&self, nickname: &str) -> Result<(), String> {
        match self
            .game_tx
            .send(GameCommand::StartGame {
                nickname: nickname.to_string(),
            })
            .await
        {
            Ok(_) => Ok(()),
            Err(error) => {
                log::error!("Tried to send GameCommand:StartGame but GameActor is not listening. Error: {error}.");
                Err(format!("Tried to send GameCommand:StartGame but GameActor is not listening. Error: {error}."))
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
                log::error!("The broadcast channel with the Game has been closed. Error: {error}.");
                Err(format!(
                    "The broadcast channel with the Game has been closed. Error: {error}."
                ))
            }
        }
    }
}
