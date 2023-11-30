#[derive(Debug, Clone)]
pub struct Round {
    pub word: String,
}

impl Round {
    pub fn new(word: &str) -> Self {
        Round {
            word: word.to_string()
        }
    }
}