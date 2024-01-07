use tokio::sync::broadcast;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::{self, Receiver as OneshotReceiver, Sender as OneshotSender};

use crate::error::Error;
use crate::game::actor::{GameCommand, GameEvent, GameWideEvent};

#[derive(Clone, Debug)]
pub struct GameClient {
    pub(super) game_tx: Sender<GameCommand>,
}

impl GameClient {
    pub async fn add_player(&self, nickname: &str) -> Result<GameWideEventReceiver, Error> {
        let (tx, rx): (OneshotSender<GameEvent>, OneshotReceiver<GameEvent>) = oneshot::channel();

        self.game_tx
            .send(GameCommand::AddPlayer {
                nickname: nickname.to_string(),
                response_tx: tx,
            })
            .await
            // This line has been reached because:
            //  1. the game exists in the GameFactory actor
            //  2. but the (individual) Game actor has been dropped
            //  3. the user navigated to this game's URL in an attempt to re-join (and there aren't any other players in the game).
            .map_err(|error| {
                Error::log_and_create_internal(&format!(
                    "The Game is not alive. Can't add Player to Game. Error: '{error}'"
                ))
            })?;

        match rx.await {
            Ok(GameEvent::PlayerAdded { broadcast_rx }) => {
                Ok(GameWideEventReceiver { broadcast_rx })
            }
            Ok(GameEvent::Error { error }) => Err(error),
            _ => Err(Error::log_and_create_internal(
                "Player sent a GameCommand::AddPlayer to Game, but Game channel died.",
            )),
        }
    }

    pub async fn remove_player(&self, nickname: &str) -> Result<(), Error> {
        self
            .game_tx
            .send(GameCommand::DisconnectPlayer {
                nickname: nickname.to_string(),
            })
            .await
            .map_err(|error| Error::log_and_create_internal(&format!("Tried to send GameCommand:RemovePlayer but GameActor is not listening. Error: {error}.")))
    }

    pub async fn start_game(&self, nickname: &str) -> Result<(), Error> {
        self
            .game_tx
            .send(GameCommand::StartGame {
                nickname: nickname.to_string(),
            })
            .await
            .map_err(|error| Error::log_and_create_internal(&format!("Tried to send GameCommand:StartGame but GameActor is not listening. Error: {error}.")))
    }

    pub async fn send_chat_message(&self, sender: &str, content: &str) -> Result<(), Error> {
        self
            .game_tx
            .send(GameCommand::AddChatMessage {
                sender: sender.to_string(),
                content: content.to_string(),
            })
            .await
            .map_err(|error| Error::log_and_create_internal(&format!("Tried to send GameCommand::AddChatMessage but GameActor is not listening. Error: {error}.")))
    }

    pub async fn add_player_words(&self, player: &str, words: Vec<String>) -> Result<(), Error> {
        self.game_tx
            .send(GameCommand::AddPlayerWords {
                nickname: player.to_string(),
                words,
            })
            .await
            .map_err(|_| {
                Error::log_and_create_internal(&format!(
                    "Could not send words of player {0}",
                    player,
                ))
            })
    }
}

pub struct GameWideEventReceiver {
    broadcast_rx: broadcast::Receiver<GameWideEvent>,
}

impl GameWideEventReceiver {
    pub async fn next(&mut self) -> Result<GameWideEvent, Error> {
        self.broadcast_rx.recv().await.map_err(|error| {
            Error::log_and_create_internal(&format!(
                "The broadcast channel with the Game has been closed. Error: {error}."
            ))
        })
    }
}
