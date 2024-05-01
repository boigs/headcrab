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
            "GameCommand::AddPlayer",
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
            "GameCommand::DisconnectPlayer",
        )
        .await
    }

    pub async fn start_game(&self, nickname: &str, amount_of_rounds: u8) -> Result<(), Error> {
        let (tx, rx): (OneshotSender<GameEvent>, OneshotReceiver<GameEvent>) = oneshot::channel();

        self.send_command(
            GameCommand::StartGame {
                nickname: nickname.to_string(),
                response_tx: tx,
                amount_of_rounds,
            },
            "GameCommand::StartGame",
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

    pub async fn add_player_words(&self, nickname: &str, words: Vec<String>) -> Result<(), Error> {
        let (tx, rx): (OneshotSender<GameEvent>, OneshotReceiver<GameEvent>) = oneshot::channel();

        self.send_command(
            GameCommand::AddPlayerWords {
                nickname: nickname.to_string(),
                words,
                response_tx: tx,
            },
            "GameCommand::AddPlayerWords",
        )
        .await?;

        match rx.await {
            Ok(GameEvent::Ok) => Ok(()),
            error => Err(GameClient::handle_event_error(error)),
        }
    }

    pub async fn add_player_voting_word(
        &self,
        nickname: &str,
        word: Option<String>,
    ) -> Result<(), Error> {
        let (tx, rx): (OneshotSender<GameEvent>, OneshotReceiver<GameEvent>) = oneshot::channel();

        self.send_command(
            GameCommand::SetPlayerVotingWord {
                nickname: nickname.to_string(),
                word,
                response_tx: tx,
            },
            "GameCommand::SetPlayerVotingWord",
        )
        .await?;

        match rx.await {
            Ok(GameEvent::Ok) => Ok(()),
            error => Err(GameClient::handle_event_error(error)),
        }
    }

    pub async fn accept_players_voting_words(&self, nickname: &str) -> Result<(), Error> {
        let (tx, rx): (OneshotSender<GameEvent>, OneshotReceiver<GameEvent>) = oneshot::channel();

        self.send_command(
            GameCommand::AcceptPlayersVotingWords {
                nickname: nickname.to_string(),
                response_tx: tx,
            },
            "GameCommand::AcceptPlayersVotingWords",
        )
        .await?;

        match rx.await {
            Ok(GameEvent::Ok) => Ok(()),
            error => Err(GameClient::handle_event_error(error)),
        }
    }

    pub async fn continue_to_next_round(&self, nickname: &str) -> Result<(), Error> {
        let (tx, rx): (OneshotSender<GameEvent>, OneshotReceiver<GameEvent>) = oneshot::channel();

        self.send_command(
            GameCommand::ContinueToNextRound {
                nickname: nickname.to_string(),
                response_tx: tx,
            },
            "GameCommand::ContinueToNextRound",
        )
        .await?;

        match rx.await {
            Ok(GameEvent::Ok) => Ok(()),
            error => Err(GameClient::handle_event_error(error)),
        }
    }

    async fn send_command(&self, command: GameCommand, command_name: &str) -> Result<(), Error> {
        self.game_tx.send(command).await.map_err(|error| {
            Error::log_and_create_internal(&format!("The Game channel is closed, cloud not send command '{command_name}'. Error: '{error}'"))
        })
    }

    fn handle_event_error(error: Result<GameEvent, RecvError>) -> Error {
        match error {
            Ok(GameEvent::Error { error }) => error,
            Ok(unexpected_event) => Error::log_and_create_internal(&format!(
                "Received an unexpected GameEvent. GameEvent: '{unexpected_event}'."
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
