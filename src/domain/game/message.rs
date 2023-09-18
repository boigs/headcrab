pub enum GameCommand {
    AddPlayer { nickname: String, },
}

pub enum GameResponse {
    PlayerAdded,
}
