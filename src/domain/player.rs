use serde::Serialize;

pub mod actor;

#[derive(Debug, Clone, Serialize, PartialEq)]
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
