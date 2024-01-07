use serde::{Deserialize, Serialize};

use crate::{game::game_fsm::GameFsmState, player::Player, round::Round};

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
    word: String,
}

impl From<Round> for RoundDto {
    fn from(val: Round) -> Self {
        Self { word: val.word }
    }
}

pub fn state_to_string(state: GameFsmState) -> String {
    match state {
        GameFsmState::Lobby => "Lobby".to_string(),
        GameFsmState::CreatingNewRound => "CreatingNewRound".to_string(),
        GameFsmState::PlayersWritingWords => "PlayersWritingWords".to_string(),
        /*GameFsmState::EndOfGame => "EndOfGame".to_string(),
        GameFsmState::PlayersWritingWords => "PlayersWritingWords".to_string(),
        GameFsmState::WordCounting => "WordCounting".to_string(),*/
    }
}
