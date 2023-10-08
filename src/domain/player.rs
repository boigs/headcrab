use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Player {
    pub nickname: String,
}

impl Player {
    pub fn new(name: &str) -> Self {
        Player {
            nickname: String::from(name),
        }
    }
}
