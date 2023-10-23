use serde::Serialize;

use crate::domain::player::Player;

#[derive(Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WsMessage {
    Error { message: String },
    GameState { players: Vec<PlayerDto> },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDto {
    nickname: String,
    is_host: bool,
}

impl Into<PlayerDto> for Player {
    fn into(self) -> PlayerDto {
        PlayerDto {
            nickname: self.nickname,
            is_host: self.is_host,
        }
    }
}
