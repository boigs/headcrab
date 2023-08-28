use super::player::Player;

#[derive(Debug)]
pub struct Lobby {
    players: Vec<Player>,
}

impl Lobby {
    pub fn new() -> Self {
        Lobby { players: vec![] }
    }

    pub fn players(&self) -> &Vec<Player> {
        &self.players
    }

    pub fn add_player(&mut self, player: Player) {
        self.players.push(player);
    }
}
