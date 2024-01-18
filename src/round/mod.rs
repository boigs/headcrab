use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Word {
    pub word: String,
    pub is_used: bool,
    pub score: usize,
}

impl Word {
    pub fn new(word: String) -> Self {
        Word {
            word,
            is_used: false,
            score: 0,
        }
    }
}

#[derive(Default, Debug, Clone)]
struct RoundScoreState {
    current_player: usize,
    current_word: Option<String>,
    player_word_submission: HashMap<String, Option<String>>,
}

#[derive(Debug, Clone)]
pub struct Round {
    pub word: String,
    players: Vec<String>,
    player_words: HashMap<String, Vec<Word>>,
    score: RoundScoreState,
}

impl Round {
    pub fn new(word: &str, players: Vec<String>) -> Self {
        Round {
            word: word.to_string(),
            players,
            player_words: HashMap::new(),
            score: RoundScoreState::default(),
        }
    }

    // TODO: add unit tests
    pub fn player_has_word(&self, nickname: &str, word: &str) -> bool {
        self.player_words
            .get(nickname)
            .unwrap()
            .iter()
            .any(|w| w.word == word)
    }

    // TODO: add unit tests
    pub fn add_words(&mut self, nickname: String, words: Vec<String>) {
        self.player_words
            .insert(nickname, words.into_iter().map(Word::new).collect());
    }

    // TODO: add unit tests
    pub fn have_all_players_submitted_words(&self, players: &[String]) -> bool {
        players
            .iter()
            .all(|player| self.player_words.contains_key(player))
    }

    // TODO: add unit tests
    pub fn next_player_to_score(&mut self, number_of_players: usize) -> Option<()> {
        self.score.current_player += 1;
        if self.score.current_player >= number_of_players {
            None
        } else {
            self.score.current_word = None;
            Some(())
        }
    }

    pub fn next_word_to_score(&mut self) -> Option<String> {
        if let Some(words) = self
            .player_words
            .get(&self.players[self.score.current_player])
        {
            if let Some(word) = words.iter().find(|word| !word.is_used) {
                self.score.current_word = Some(word.word.clone());
                self.add_player_word_submission(
                    self.players[self.score.current_player].clone(),
                    self.score.current_word.clone(),
                );
            } else {
                self.score.current_word = None;
            }
        } else {
            self.score.current_word = None;
        }

        self.score.current_word.clone()
    }

    pub fn add_player_word_submission(
        &mut self,
        nickname: String,
        word: Option<String>,
    ) -> Option<()> {
        if let Some(word) = word.clone() {
            // If the player does not have the word or they've used it already then it's an error
            self.player_words
                .get(&nickname)
                .and_then(|words| words.iter().find(|w| w.word == word && !w.is_used))?;
        }
        self.score.player_word_submission.insert(nickname, word);
        Some(())
    }

    pub fn players_submitted_words_count(&self) -> usize {
        self.score.player_word_submission.len()
    }

    pub fn compute_score(&mut self) {
        let score = self
            .score
            .player_word_submission
            .iter()
            .filter(|(_, submission_word)| submission_word.is_some())
            .count();
        for (submission_nickname, submission_word) in &self.score.player_word_submission {
            if let Some(submission_word) = submission_word {
                if let Some(words) = self.player_words.get_mut(submission_nickname) {
                    if let Some(word) = words.iter_mut().find(|word| &word.word == submission_word)
                    {
                        word.score = score;
                        word.is_used = true;
                    }
                }
            }
        }
        self.score.player_word_submission = HashMap::default();
    }
}

#[cfg(test)]
mod tests {
    use super::{Round, Word};

    #[test]
    fn player_cannot_submit_non_existent_word() {
        let mut round = Round::new("word", vec!["p".to_string()]);
        round.add_words(
            "p".to_string(),
            vec!["word1".to_string(), "word2".to_string()],
        );

        assert!(round
            .add_player_word_submission("p".to_string(), Some("word3".to_string()))
            .is_none());
    }

    #[test]
    fn player_cannot_submit_used_word() {
        let mut round = Round::new("word", vec!["p1".to_string()]);
        round.add_words(
            "p".to_string(),
            vec!["word1".to_string(), "word2".to_string()],
        );
        round.add_player_word_submission("p".to_string(), Some("word1".to_string()));
        round.compute_score();

        assert!(round
            .add_player_word_submission("p".to_string(), Some("word1".to_string()))
            .is_none());
    }

    #[test]
    fn compute_score_works() {
        let mut round = Round::new(
            "word",
            vec!["p1".to_string(), "p2".to_string(), "p3".to_string()],
        );
        round.add_words(
            "p1".to_string(),
            vec!["p1_word1".to_string(), "p1_word2".to_string()],
        );
        round.add_words(
            "p2".to_string(),
            vec!["p2_word1".to_string(), "p2_word2".to_string()],
        );
        round.add_words(
            "p3".to_string(),
            vec!["p3_word1".to_string(), "p3_word2".to_string()],
        );

        round.add_player_word_submission("p1".to_string(), Some("p1_word1".to_string()));
        round.add_player_word_submission("p2".to_string(), Some("p2_word1".to_string()));
        round.add_player_word_submission("p3".to_string(), None);

        round.compute_score();

        assert_eq!(get_word(&round, "p1", "p1_word1").score, 2);
        assert_eq!(get_word(&round, "p1", "p1_word2").score, 0);
        assert_eq!(get_word(&round, "p2", "p2_word1").score, 2);
        assert_eq!(get_word(&round, "p2", "p2_word2").score, 0);
        assert_eq!(get_word(&round, "p3", "p3_word1").score, 0);
        assert_eq!(get_word(&round, "p3", "p3_word2").score, 0);

        assert!(round.score.player_word_submission.is_empty());
        assert!(get_word(&round, "p1", "p1_word1").is_used);
        assert!(!get_word(&round, "p1", "p1_word2").is_used);
        assert!(get_word(&round, "p2", "p2_word1").is_used);
        assert!(!get_word(&round, "p2", "p2_word2").is_used);
        assert!(!get_word(&round, "p3", "p3_word1").is_used);
        assert!(!get_word(&round, "p3", "p3_word2").is_used);
    }

    #[test]
    fn given_all_words_are_not_used_when_choosing_next_word_then_chooses_correctly() {
        let mut round = Round::new("word", vec!["p1".to_string()]);
        round.add_words(
            "p1".to_string(),
            vec!["word1".to_string(), "word2".to_string()],
        );

        assert_eq!(round.next_word_to_score(), Some("word1".to_string()));
    }

    #[test]
    fn given_the_first_two_words_are_used_when_choosing_next_word_then_chooses_the_second_word() {
        let mut round = Round::new("word", vec!["p1".to_string()]);
        round.add_words(
            "p1".to_string(),
            vec![
                "word1".to_string(),
                "word2".to_string(),
                "word3".to_string(),
            ],
        );
        round.next_word_to_score();
        round.compute_score();

        round.next_word_to_score();
        round.compute_score();

        assert_eq!(round.next_word_to_score(), Some("word3".to_string()));
    }

    fn get_word<'a>(round: &'a Round, nickname: &str, word: &str) -> &'a Word {
        round
            .player_words
            .get(nickname)
            .unwrap()
            .iter()
            .find(|w| w.word == word)
            .unwrap()
    }
}
