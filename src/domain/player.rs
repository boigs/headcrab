use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub nickname: String,
    pub is_host: bool,
}

impl Player {
    pub fn new(nickname: &str) -> Self {
        Player {
            nickname: String::from(nickname),
            is_host: false,
        }
    }
}
