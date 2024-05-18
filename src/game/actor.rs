use std::fmt::{Display, Formatter};
use std::time::Duration;
use tokio::sync::broadcast::error::SendError;
use tokio::sync::oneshot::Sender as OneshotSender;
use tokio::sync::{
    broadcast, mpsc,
    mpsc::{Receiver, Sender},
};
use tokio::time;

use crate::config::GameSettings;
use crate::error::Error;
use crate::game::actor_client::GameClient;
use crate::game::game_fsm::GameFsmState;
use crate::game::Game;
use crate::game_factory::actor_client::GameFactoryClient;
use crate::metrics::ACTIVE_GAMES;
use crate::player::Player;
use crate::round::Round;

pub struct GameActor {
    game: Game,
    game_rx: Receiver<GameCommand>,
    broadcast_tx: broadcast::Sender<GameWideEvent>,
    game_factory: GameFactoryClient,
    inactivity_timeout: Duration,
}

impl GameActor {
    pub fn spawn(
        id: &str,
        settings: GameSettings,
        words: Vec<String>,
        game_factory: GameFactoryClient,
    ) -> GameClient {
        let game = Game::new(id, words);
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
                inactivity_timeout: settings.inactivity_timeout(),
            }
            .start(),
        );

        GameClient { game_tx }
    }

    async fn start(mut self) {
        ACTIVE_GAMES.inc();

        loop {
            match time::timeout(self.inactivity_timeout, self.game_rx.recv()).await {
                Err(_) => {
                    if self.game.all_players_are_disconnected() {
                        log::info!(
                            "No activity detected in game {} after {} seconds. Stopping game actor.",
                            self.game.id(), self.inactivity_timeout.as_secs()
                        );
                        break;
                    }
                }
                Ok(None) => {
                    log::info!("Game channel has been dropped. Stopping game actor.");
                    break;
                }
                Ok(Some(command)) => {
                    let response = match command {
                        GameCommand::AddPlayer {
                            nickname,
                            response_tx,
                        } => {
                            let result =
                                self.game
                                    .add_player(&nickname)
                                    .map(|_| GameEvent::PlayerAdded {
                                        broadcast_rx: self.broadcast_tx.subscribe(),
                                    });
                            Some((result, nickname, response_tx))
                        }
                        GameCommand::DisconnectPlayer { nickname } => {
                            let _ = self.game.disconnect_player(&nickname);
                            None
                        }
                        GameCommand::StartGame {
                            nickname,
                            response_tx,
                            amount_of_rounds,
                        } => {
                            let result = self
                                .game
                                .start_game(&nickname, amount_of_rounds)
                                .map(|_| GameEvent::Ok);
                            Some((result, nickname, response_tx))
                        }
                        GameCommand::AddChatMessage { sender, content } => {
                            if let Err(error) = self
                                .broadcast_tx
                                .send(GameWideEvent::ChatMessage { sender, content })
                            {
                                log::error!(
                                    "Error when sending GameWideEvent::ChatMessage broadcast: {}.",
                                    error
                                );
                            }
                            continue;
                        }
                        GameCommand::AddPlayerWords {
                            nickname,
                            words,
                            response_tx,
                        } => {
                            let result = self
                                .game
                                .add_player_words(&nickname, words)
                                .map(|_| GameEvent::Ok);
                            Some((result, nickname, response_tx))
                        }
                        GameCommand::SetPlayerVotingWord {
                            nickname,
                            word,
                            response_tx,
                        } => {
                            let result = self
                                .game
                                .set_player_voting_word(&nickname, word)
                                .map(|_| GameEvent::Ok);
                            Some((result, nickname, response_tx))
                        }
                        GameCommand::AcceptPlayersVotingWords {
                            nickname,
                            response_tx,
                        } => {
                            let result = self
                                .game
                                .accept_players_voting_words(&nickname)
                                .map(|_| GameEvent::Ok);
                            Some((result, nickname, response_tx))
                        }
                        GameCommand::ContinueToNextRound {
                            nickname,
                            response_tx,
                        } => {
                            let result = self
                                .game
                                .continue_to_next_round(&nickname)
                                .map(|_| GameEvent::Ok);
                            Some((result, nickname, response_tx))
                        }
                        GameCommand::ContinueToNewGame {
                            nickname,
                            response_tx,
                        } => {
                            let result = self
                                .game
                                .continue_to_new_game(&nickname)
                                .map(|_| GameEvent::Ok);
                            Some((result, nickname, response_tx))
                        }
                    };
                    if let Some((result, nickname, response_tx)) = response {
                        let event = match result {
                            Ok(event) => event,
                            Err(error) => GameEvent::Error { error },
                        };
                        if let Err(error) = response_tx.send(event) {
                            log::error!("Sent GameEvent to Player {nickname} but the response channel is closed. Removing the Player. Error: '{error}'.");
                            let _ = self.game.disconnect_player(&nickname);
                        }
                    }
                    let _ = self.send_game_state();
                }
            }
        }

        self.stop_game().await;
        ACTIVE_GAMES.dec();
    }

    fn send_game_state(&self) -> Result<usize, SendError<GameWideEvent>> {
        self.broadcast_tx.send(GameWideEvent::GameState {
            state: self.game.state().clone(),
            players: self.game.players().to_vec(),
            rounds: self.game.rounds().to_vec(),
            amount_of_rounds: self.game.amount_of_rounds,
        })
    }

    async fn stop_game(self) {
        let game_id = self.game.id();
        if let Err(error) = self.game_factory.remove_game(game_id).await {
            log::error!("The GameFactory channel is closed, can't remove the Game. GameId: '{game_id}', Error: '{error}'.");
        }
    }
}

pub(crate) enum GameCommand {
    AddPlayer {
        nickname: String,
        response_tx: OneshotSender<GameEvent>,
    },
    DisconnectPlayer {
        nickname: String,
    },
    StartGame {
        nickname: String,
        response_tx: OneshotSender<GameEvent>,
        amount_of_rounds: u8,
    },
    AddChatMessage {
        sender: String,
        content: String,
    },
    AddPlayerWords {
        nickname: String,
        words: Vec<String>,
        response_tx: OneshotSender<GameEvent>,
    },
    SetPlayerVotingWord {
        nickname: String,
        word: Option<String>,
        response_tx: OneshotSender<GameEvent>,
    },
    AcceptPlayersVotingWords {
        nickname: String,
        response_tx: OneshotSender<GameEvent>,
    },
    ContinueToNextRound {
        nickname: String,
        response_tx: OneshotSender<GameEvent>,
    },
    ContinueToNewGame {
        nickname: String,
        response_tx: OneshotSender<GameEvent>,
    },
}

#[derive(Debug)]
pub(crate) enum GameEvent {
    PlayerAdded {
        broadcast_rx: broadcast::Receiver<GameWideEvent>,
    },
    Ok,
    Error {
        error: Error,
    },
}

impl Display for GameEvent {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}",
            match self {
                GameEvent::PlayerAdded { .. } => "GameEvent::PlayerAdded".to_string(),
                GameEvent::Ok => "GameEvent::Ok".to_string(),
                GameEvent::Error { error } => format!("Error '{error}'").to_string(),
            }
        )
    }
}

#[derive(Clone, Debug)]
pub enum GameWideEvent {
    GameState {
        state: GameFsmState,
        players: Vec<Player>,
        rounds: Vec<Round>,
        amount_of_rounds: Option<u8>,
    },
    ChatMessage {
        sender: String,
        content: String,
    },
}
