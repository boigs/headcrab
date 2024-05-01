use std::collections::HashMap;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use super::{test_app::TestApp, test_player::TestPlayer};

pub struct TestGame {
    pub app: TestApp,
    pub id: String,
    pub players: Vec<TestPlayer>,
}

impl TestGame {
    pub async fn add_player(&mut self, nickname: &str) -> Result<GameState, String> {
        let (tx, rx) = self
            .app
            .open_game_websocket(&self.id, nickname)
            .await?
            .split();
        let mut player = TestPlayer {
            nickname: nickname.to_string(),
            words: vec![format!("{nickname}_w1"), format!("{nickname}_w2")],
            tx,
            rx,
        };
        // Clear the messages on the other players
        for player in self.players.iter_mut() {
            let _ = player.receive_game_state().await.unwrap();
        }
        let state = player.receive_game_state().await?;
        self.players.push(player);
        Ok(state)
    }

    pub async fn players_send_words(&mut self) -> GameState {
        let _ = self.players[0].send_words().await.unwrap();
        let _ = self.players[1].receive_game_state().await.unwrap();
        let _ = self.players[2].receive_game_state().await.unwrap();

        let _ = self.players[1].send_words().await.unwrap();
        let _ = self.players[0].receive_game_state().await.unwrap();
        let _ = self.players[2].receive_game_state().await.unwrap();

        let _ = self.players[2].send_words().await.unwrap();
        let _ = self.players[1].receive_game_state().await.unwrap();
        self.players[0].receive_game_state().await.unwrap()
    }

    pub async fn complete_round(&mut self) {
        let state = self.players_send_words().await;
        assert_eq!(state.state, GameFsmState::PlayersSubmittingVotingWord);

        // Voting for p1_w1
        // p1: [used, unused], p2: [used, unused], p3: [unused, unused]
        let voting_word = self.players[1].words.get(0).cloned();
        let _ = self.players[1].send_voting_word(voting_word).await.unwrap();
        let _ = self.players[0].receive_game_state().await.unwrap();
        let _ = self.players[2].receive_game_state().await.unwrap();

        let _ = self.players[0].accept_players_voting_words().await.unwrap();
        let _ = self.players[1].receive_game_state().await.unwrap();
        let _ = self.players[2].receive_game_state().await.unwrap();

        // Voting for p1_w2
        // p1: [used, used], p2: [used, unused], p3: [unused, unused]
        let _ = self.players[1].send_voting_word(None).await.unwrap();
        let _ = self.players[0].receive_game_state().await.unwrap();
        let _ = self.players[2].receive_game_state().await.unwrap();

        let voting_word = self.players[2].words.get(1).cloned();
        let _ = self.players[2].send_voting_word(voting_word).await.unwrap();
        let _ = self.players[0].receive_game_state().await.unwrap();
        let _ = self.players[1].receive_game_state().await.unwrap();

        let _ = self.players[0].accept_players_voting_words().await.unwrap();
        let _ = self.players[1].receive_game_state().await.unwrap();
        let _ = self.players[2].receive_game_state().await.unwrap();

        // Voting for p2_w2
        // p1: [used, used], p2: [used, used], p3: [unused, used]
        let _ = self.players[0].accept_players_voting_words().await.unwrap();
        let _ = self.players[1].receive_game_state().await.unwrap();
        let _ = self.players[2].receive_game_state().await.unwrap();

        // Voting for p3_w1
        // p1: [used, used], p2: [used, used], p3: [used, used]
        let state: GameState = self.players[0].accept_players_voting_words().await.unwrap();
        let _ = self.players[1].receive_game_state().await.unwrap();
        let _ = self.players[2].receive_game_state().await.unwrap();

        assert_eq!(state.state, GameFsmState::EndOfRound);
    }

    pub async fn continue_to_next_round(&mut self) -> GameState {
        let (host, rest) = self.players.split_first_mut().unwrap();
        let state = host.continue_to_next_round().await.unwrap();
        for player in rest {
            let _ = player.receive_game_state().await.unwrap();
        }
        state
    }
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct GameState {
    pub state: GameFsmState,
    pub players: Vec<Player>,
    pub rounds: Vec<Round>,
    pub amount_of_rounds: Option<u8>,
}

impl GameState {
    pub fn last_round(&self) -> Round {
        self.rounds.last().unwrap().clone()
    }
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub nickname: String,
    pub is_host: bool,
    pub is_connected: bool,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Round {
    pub word: String,
    pub player_words: HashMap<String, Vec<Word>>,
    pub player_voting_words: HashMap<String, Option<String>>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Word {
    pub word: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum WsMessageOut {
    Error {
        r#type: String,
        title: String,
        detail: String,
    },
    GameState {
        state: GameFsmState,
        players: Vec<Player>,
        rounds: Vec<Round>,
        amount_of_rounds: Option<u8>,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum WsMessageIn {
    #[serde(rename_all = "camelCase")]
    StartGame {
        // We use i8 instead of u8 so that we can send a negative value to test the validation on this field
        amount_of_rounds: i8,
    },
    #[serde(rename_all = "camelCase")]
    PlayerWords {
        words: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    PlayerVotingWord {
        word: Option<String>,
    },
    AcceptPlayersVotingWords,
    ContinueToNextRound,
}

#[derive(Deserialize, Debug, PartialEq)]
pub enum GameFsmState {
    Lobby,
    PlayersSubmittingWords,
    PlayersSubmittingVotingWord,
    EndOfRound,
    EndOfGame,
}
