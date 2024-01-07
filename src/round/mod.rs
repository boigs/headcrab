use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Round {
    pub word: String,
    player_words: HashMap<String, Vec<String>>,
}

impl Round {
    pub fn new(word: &str) -> Self {
        Round {
            word: word.to_string(),
            player_words: HashMap::new(),
        }
    }

    pub fn add_words(&mut self, nickname: String, words: Vec<String>) {
        self.player_words.insert(nickname, words);
    }

    pub fn have_all_players_submitted_words(&self, players: &[String]) -> bool {
        players
            .iter()
            .all(|player| self.player_words.contains_key(player))
    }
}
