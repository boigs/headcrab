use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Player {
    id: Uuid,
    pub name: String,
}

impl Player {
    pub fn new(name: &str) -> Self {
        let id = Uuid::new_v4();
        Player {
            id,
            name: String::from(name),
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }
}
