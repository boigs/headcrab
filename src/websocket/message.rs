use serde::{Deserialize, Serialize};

use crate::domain::{game_fsm::GameFsmState, player::Player};

#[derive(Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WsMessageOut {
    Error {
        message: String,
    },
    GameState {
        state: String,
        players: Vec<PlayerDto>,
    },
    ChatText {
        text: String,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WsMessageIn {
    #[serde(rename_all = "camelCase")]
    StartGame { amount_of_rounds: u8 },
    #[serde(rename_all = "camelCase")]
    ChatText { text: String },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDto {
    nickname: String,
    is_host: bool,
}

impl From<Player> for PlayerDto {
    fn from(val: Player) -> Self {
        Self {
            nickname: val.nickname,
            is_host: val.is_host,
        }
    }
}

pub fn state_to_string(state: GameFsmState) -> String {
    match state {
        GameFsmState::Lobby => "Lobby".to_string(),
        GameFsmState::ChooseWord => "ChooseWord".to_string(),
        /*GameFsmState::EndOfGame => "EndOfGame".to_string(),
        GameFsmState::PlayersWritingWords => "PlayersWritingWords".to_string(),
        GameFsmState::WordCounting => "WordCounting".to_string(),*/
    }
}
