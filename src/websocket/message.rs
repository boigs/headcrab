use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    game::game_fsm::GameFsmState,
    player::Player,
    round::{Round, VotingItem, Word},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub(crate) enum WsMessageOut {
    #[serde(rename_all = "camelCase")]
    Error {
        r#type: String,
        title: String,
        detail: String,
    },
    #[serde(rename_all = "camelCase")]
    GameState {
        state: String,
        players: Vec<PlayerDto>,
        rounds: Vec<RoundDto>,
        amount_of_rounds: Option<u8>,
    },
    #[serde(rename_all = "camelCase")]
    ChatMessage { sender: String, content: String },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum WsMessageIn {
    #[serde(rename_all = "camelCase")]
    StartGame {
        amount_of_rounds: u8,
    },
    #[serde(rename_all = "camelCase")]
    ChatMessage {
        content: String,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlayerDto {
    nickname: String,
    is_host: bool,
    is_connected: bool,
}

impl From<Player> for PlayerDto {
    fn from(val: Player) -> Self {
        Self {
            nickname: val.nickname,
            is_host: val.is_host,
            is_connected: val.is_connected,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoundDto {
    pub word: String,
    pub player_words: HashMap<String, Vec<WordDto>>,
    pub player_voting_words: HashMap<String, Option<String>>,
    pub voting_item: Option<VotingItemDto>,
}

impl From<Round> for RoundDto {
    fn from(val: Round) -> Self {
        Self {
            word: val.word,
            player_words: val
                .player_words
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        v.iter().map(|word| word.clone().into()).collect(),
                    )
                })
                .collect(),
            player_voting_words: val.player_voting_words,
            voting_item: val.voting_item.map(|voting_item| voting_item.into()),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WordDto {
    word: String,
    is_used: bool,
    score: usize,
}

impl From<Word> for WordDto {
    fn from(val: Word) -> Self {
        Self {
            word: val.word.clone(),
            is_used: val.is_used,
            score: val.score,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VotingItemDto {
    player_nickname: String,
    word: String,
}

impl From<VotingItem> for VotingItemDto {
    fn from(val: VotingItem) -> Self {
        Self {
            player_nickname: val.player_nickname,
            word: val.word,
        }
    }
}

pub fn state_to_string(state: GameFsmState) -> String {
    match state {
        GameFsmState::Lobby => "Lobby".to_string(),
        GameFsmState::CreatingNewRound => "CreatingNewRound".to_string(),
        GameFsmState::PlayersSubmittingWords => "PlayersSubmittingWords".to_string(),
        GameFsmState::ChooseNextVotingItem => "ChooseNextVotingItem".to_string(),
        GameFsmState::PlayersSubmittingVotingWord => "PlayersSubmittingVotingWord".to_string(),
        GameFsmState::EndOfRound => "EndOfRound".to_string(),
        GameFsmState::EndOfGame => "EndOfGame".to_string(),
    }
}
