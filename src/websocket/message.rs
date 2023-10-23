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

impl From<Player> for PlayerDto {
    fn from(val: Player) -> Self {
        Self {
            nickname: val.nickname,
            is_host: val.is_host,
        }
    }
}
