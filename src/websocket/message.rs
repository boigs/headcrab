use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    game::game_fsm::GameFsmState,
    player::Player,
    round::{Round, RoundScoreState, Word},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub(crate) enum WsMessageOut {
    Error {
        r#type: String,
        title: String,
        detail: String,
    },
    GameState {
        state: String,
        players: Vec<PlayerDto>,
        rounds: Vec<RoundDto>,
    },
    ChatMessage {
        sender: String,
        content: String,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum WsMessageIn {
    #[serde(rename_all = "camelCase")]
    StartGame {
        amount_of_rounds: u8,
    },
    ChatMessage {
        content: String,
    },
    PlayerWords {
        words: Vec<String>,
    },
    PlayerWordSubmission {
        // TODO: handle empty word (skip) submissions
        word: String,
    },
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
    pub score: RoundScoreStateDto,
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
            score: val.score.into(),
        }
    }
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

impl From<RoundScoreState> for RoundScoreStateDto {
    fn from(val: RoundScoreState) -> Self {
        Self {
            current_player: val.current_player.clone().map(|tuple| tuple.1),
            current_word: val.current_word.clone(),
            player_word_submission: val.player_word_submission.clone(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoundScoreStateDto {
    current_player: Option<String>,
    current_word: Option<String>,
    player_word_submission: HashMap<String, Option<String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WordDto {
    word: String,
    is_used: bool,
    score: usize,
}

pub fn state_to_string(state: GameFsmState) -> String {
    match state {
        GameFsmState::Lobby => "Lobby".to_string(),
        GameFsmState::CreatingNewRound => "CreatingNewRound".to_string(),
        GameFsmState::PlayersWritingWords => "PlayersWritingWords".to_string(),
        GameFsmState::ScoreCounting => "WordCounting".to_string(),
        GameFsmState::ChooseNextPlayer => "ChooseNextPlayer".to_string(),
        GameFsmState::ChooseNextWord => "ChooseNextWord".to_string(),
        GameFsmState::EndOfGame => "EndOfGame".to_string(),
        GameFsmState::PlayersSendingWordSubmission => "PlayersSendingWordSubmission".to_string(),
        /*GameFsmState::EndOfGame => "EndOfGame".to_string(),
        ,*/
    }
}
