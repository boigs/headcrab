use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Player {
    id: Uuid,
    pub nickname: String,
}

impl Player {
    pub fn new(name: &str) -> Self {
        let id = Uuid::new_v4();
        Player {
            id,
            nickname: String::from(name),
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }
}
