use tokio::sync::broadcast;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::error::RecvError;
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

        // An error can be returned at this point if:
        //  1. the game exists in the GameFactory actor
        //  2. but the (individual) Game actor has been dropped
        //  3. the user navigated to this game's URL in an attempt to re-join (and there aren't any other players in the game)
        self.send_command(
            GameCommand::AddPlayer {
                nickname: nickname.to_string(),
                response_tx: tx,
            },
            "The Game is not alive. Can't add Player to Game",
        )
        .await?;

        match rx.await {
            Ok(GameEvent::PlayerAdded { broadcast_rx }) => {
                Ok(GameWideEventReceiver { broadcast_rx })
            }
            error => Err(GameClient::handle_event_error(error)),
        }
    }

    pub async fn remove_player(&self, nickname: &str) -> Result<(), Error> {
        self.send_command(
            GameCommand::DisconnectPlayer {
                nickname: nickname.to_string(),
            },
            "Tried to send GameCommand:RemovePlayer but GameActor is not listening",
        )
        .await
    }

    pub async fn start_game(&self, nickname: &str) -> Result<(), Error> {
        let (tx, rx): (OneshotSender<GameEvent>, OneshotReceiver<GameEvent>) = oneshot::channel();

        self.send_command(
            GameCommand::StartGame {
                nickname: nickname.to_string(),
                response_tx: tx,
            },
            "Tried to send GameCommand:StartGame but GameActor is not listening",
        )
        .await?;

        match rx.await {
            Ok(GameEvent::Ok) => Ok(()),
            error => Err(GameClient::handle_event_error(error)),
        }
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
        let (tx, rx): (OneshotSender<GameEvent>, OneshotReceiver<GameEvent>) = oneshot::channel();

        self.send_command(
            GameCommand::AddPlayerWords {
                nickname: player.to_string(),
                words,
                response_tx: tx,
            },
            &format!("Could not send words of player {player}"),
        )
        .await?;

        match rx.await {
            Ok(GameEvent::Ok) => Ok(()),
            error => Err(GameClient::handle_event_error(error)),
        }
    }

    pub async fn add_player_word_submission(
        &self,
        player: &str,
        word: Option<String>,
    ) -> Result<(), Error> {
        let (tx, rx): (OneshotSender<GameEvent>, OneshotReceiver<GameEvent>) = oneshot::channel();

        self.send_command(
            GameCommand::AddPlayerWordSubmission {
                nickname: player.to_string(),
                word,
                response_tx: tx,
            },
            &format!("Could not send player word submission {player}"),
        )
        .await?;

        match rx.await {
            Ok(GameEvent::Ok) => Ok(()),
            error => Err(GameClient::handle_event_error(error)),
        }
    }

    async fn send_command(&self, command: GameCommand, error_message: &str) -> Result<(), Error> {
        self.game_tx.send(command).await.map_err(|error| {
            Error::log_and_create_internal(&format!("{error_message}. Error: '{error}'"))
        })
    }

    fn handle_event_error(error: Result<GameEvent, RecvError>) -> Error {
        match error {
            Ok(GameEvent::Error { error }) => error,
            Ok(unexpected_response) => Error::log_and_create_internal(&format!(
                "Received an unexpected GameEvent. GameEvent: '{unexpected_response}'."
            )),
            _ => Error::log_and_create_internal(
                "Sent a command to the Game actor, but the actor channel died.",
            ),
        }
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
