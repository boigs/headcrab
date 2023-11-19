use crate::domain::error::Error;
use tokio::sync::broadcast;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::{self, Receiver as OneshotReceiver, Sender as OneshotSender};

use super::{GameCommand, GameEvent, GameWideEvent};

#[derive(Clone, Debug)]
pub struct GameClient {
    pub(super) game_tx: Sender<GameCommand>,
}

impl GameClient {
    pub async fn add_player(&self, nickname: &str) -> Result<GameWideEventReceiver, Error> {
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
            return Err(Error::Internal(
                "The Game is not alive. Can't add Player to Game.".to_string(),
            ));
        }

        match rx.await {
            Ok(GameEvent::PlayerAdded { broadcast_rx }) => {
                Ok(GameWideEventReceiver { broadcast_rx })
            }
            Ok(GameEvent::Error { error }) => Err(error),
            _ => {
                log::error!("Player sent a GameCommand::AddPlayer to Game, but Game channel died.");
                Err(Error::Internal(
                    "Player sent a GameCommand::AddPlayer to Game, but Game channel died."
                        .to_string(),
                ))
            }
        }
    }

    pub async fn remove_player(&self, nickname: &str) -> Result<(), Error> {
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
                Err(Error::Internal(format!("Tried to send GameCommand:RemovePlayer but GameActor is not listening. Error: {error}.")))
            }
        }
    }

    pub async fn start_game(&self, nickname: &str) -> Result<(), Error> {
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
                Err(Error::Internal(format!("Tried to send GameCommand:StartGame but GameActor is not listening. Error: {error}.")))
            }
        }
    }

    pub async fn send_chat_message(&self, sender: &str, content: &str) -> Result<(), Error> {
        match self
            .game_tx
            .send(GameCommand::AddChatMessage {
                sender: sender.to_string(),
                content: content.to_string(),
            })
            .await
        {
            Ok(_) => Ok(()),
            Err(error) => {
                Err(Error::Internal(format!("Tried to send GameCommand::AddChatMessage but GameActor is not listening. Error: {error}.")))
            }
        }
    }
}

pub struct GameWideEventReceiver {
    broadcast_rx: broadcast::Receiver<GameWideEvent>,
}

impl GameWideEventReceiver {
    pub async fn next(&mut self) -> Result<GameWideEvent, Error> {
        match self.broadcast_rx.recv().await {
            Ok(game_wide_event) => Ok(game_wide_event),
            Err(error) => {
                log::error!("The broadcast channel with the Game has been closed. Error: {error}.");
                Err(Error::Internal(format!(
                    "The broadcast channel with the Game has been closed. Error: {error}."
                )))
            }
        }
    }
}
