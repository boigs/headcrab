use std::collections::{HashMap, HashSet};

use crate::error::Error;

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
pub struct RoundScoreState {
    pub current_player: Option<(usize, String)>,
    all_players_done: bool,
    pub current_word: Option<String>,
    pub player_word_submission: HashMap<String, Option<String>>,
}

#[derive(Debug, Clone)]
pub struct Round {
    pub word: String,
    players: Vec<String>,
    pub player_words: HashMap<String, Vec<Word>>,
    pub score: RoundScoreState,
}

impl Round {
    pub fn new(word: &str, players: Vec<String>) -> Result<Self, Error> {
        if players.len() < 3 {
            return Err(Error::log_and_create_internal(
                "Cannot create a round for less than 3 players",
            ));
        }
        Ok(Round {
            word: word.to_string(),
            players,
            player_words: HashMap::new(),
            score: RoundScoreState::default(),
        })
    }

    pub fn player_has_word(&self, nickname: &str, word: &str) -> bool {
        self.player_words
            .get(nickname)
            .unwrap()
            .iter()
            .any(|w| w.word == word)
    }

    pub fn add_words(&mut self, nickname: &str, words: Vec<String>) -> Result<(), Error> {
        let normalized_words: Vec<String> = words
            .iter()
            .map(|word| word.trim().to_string())
            .filter(|word| !word.is_empty())
            .collect();
        let unique_words = normalized_words
            .clone()
            .into_iter()
            .collect::<HashSet<String>>();

        if unique_words.len() == normalized_words.len() {
            self.player_words.insert(
                nickname.to_string(),
                normalized_words.into_iter().map(Word::new).collect(),
            );
            Ok(())
        } else {
            Err(Error::RepeatedWords)
        }
    }

    pub fn have_all_players_submitted_words(&self, players: &[String]) -> bool {
        players
            .iter()
            .all(|player| self.player_words.contains_key(player))
    }

    pub fn next_player_to_score(&mut self) -> Option<String> {
        if self.score.all_players_done || self.players.is_empty() {
            return None;
        }
        match self.score.current_player {
            None => {
                self.score.current_player = Some((0, self.players.first().unwrap().clone()));
            }
            Some((index, _)) => {
                if index >= self.players.len() - 1 {
                    self.score.all_players_done = true;
                    self.score.current_player = None;
                } else {
                    let new_index = index + 1;
                    self.score.current_player =
                        Some((new_index, self.players.get(new_index).unwrap().clone()));
                }
            }
        }
        self.score
            .current_player
            .clone()
            .map(|(_, nickname)| nickname)
    }

    pub fn next_word_to_score(&mut self) -> Option<String> {
        let nickname_and_next_word = {
            let (_, nickname) = self.score.current_player.clone()?;
            let words = self.player_words.get(&nickname)?;
            let next_word = words
                .iter()
                .find(|word| !word.is_used)
                .map(|word| word.word.clone())?;
            Some((nickname, next_word))
        };

        if let Some((nickname, next_word)) = nickname_and_next_word.clone() {
            self.add_player_word_submission(&nickname, Some(next_word));
        }

        self.score.current_word = nickname_and_next_word.map(|(_, next_word)| next_word);
        self.score.current_word.clone()
    }

    pub fn add_player_word_submission(
        &mut self,
        nickname: &str,
        word: Option<String>,
    ) -> Option<()> {
        if let Some(word) = word.clone() {
            // If the player does not have the word or they've used it already then it's an error
            self.player_words
                .get(nickname)
                .and_then(|words| words.iter().find(|w| w.word == word && !w.is_used))?;
        }
        self.score
            .player_word_submission
            .insert(nickname.to_string(), word);
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
    use crate::error::Error;

    use super::{Round, Word};

    static PLAYER_1: &str = "p1";
    static PLAYER_2: &str = "p2";
    static PLAYER_3: &str = "p3";

    #[test]
    fn player_cannot_submit_non_existent_word() {
        let mut round = get_round();

        round
            .add_words(PLAYER_1, vec!["word1".to_string(), "word2".to_string()])
            .unwrap();

        assert!(round
            .add_player_word_submission(PLAYER_1, Some("word3".to_string()))
            .is_none());
    }

    #[test]
    fn player_cannot_submit_used_word() {
        let mut round = get_round();
        round
            .add_words(PLAYER_1, vec!["word1".to_string(), "word2".to_string()])
            .unwrap();
        round.add_player_word_submission(PLAYER_1, Some("word1".to_string()));
        round.compute_score();

        assert!(round
            .add_player_word_submission(PLAYER_1, Some("word1".to_string()))
            .is_none());
    }

    #[test]
    fn compute_score_works() {
        let mut round = get_round();
        round
            .add_words(
                PLAYER_1,
                vec!["p1_word1".to_string(), "p1_word2".to_string()],
            )
            .unwrap();
        round
            .add_words(
                PLAYER_2,
                vec!["p2_word1".to_string(), "p2_word2".to_string()],
            )
            .unwrap();
        round
            .add_words(
                PLAYER_3,
                vec!["p3_word1".to_string(), "p3_word2".to_string()],
            )
            .unwrap();

        round.add_player_word_submission(PLAYER_1, Some("p1_word1".to_string()));
        round.add_player_word_submission(PLAYER_2, Some("p2_word1".to_string()));
        round.add_player_word_submission(PLAYER_3, None);

        round.compute_score();

        assert_eq!(get_word(&round, PLAYER_1, "p1_word1").score, 2);
        assert_eq!(get_word(&round, PLAYER_1, "p1_word2").score, 0);
        assert_eq!(get_word(&round, PLAYER_2, "p2_word1").score, 2);
        assert_eq!(get_word(&round, PLAYER_2, "p2_word2").score, 0);
        assert_eq!(get_word(&round, PLAYER_3, "p3_word1").score, 0);
        assert_eq!(get_word(&round, PLAYER_3, "p3_word2").score, 0);

        assert!(round.score.player_word_submission.is_empty());
        assert!(get_word(&round, PLAYER_1, "p1_word1").is_used);
        assert!(!get_word(&round, PLAYER_1, "p1_word2").is_used);
        assert!(get_word(&round, PLAYER_2, "p2_word1").is_used);
        assert!(!get_word(&round, PLAYER_2, "p2_word2").is_used);
        assert!(!get_word(&round, PLAYER_3, "p3_word1").is_used);
        assert!(!get_word(&round, PLAYER_3, "p3_word2").is_used);
    }

    #[test]
    fn given_create_round_when_three_players_or_more_then_ok() {
        let round = Round::new(
            "word",
            vec!["p1".to_string(), "p2".to_string(), "p3".to_string()],
        );
        assert!(round.is_ok());
    }

    #[test]
    fn given_create_round_when_less_than_three_players_then_error() {
        let round = Round::new("word", vec!["p1".to_string()]);
        assert!(round.is_err());
    }

    #[test]
    fn given_some_players_when_choosing_next_player_then_chooses_correctly() {
        let mut round = get_round();
        assert_eq!(round.next_player_to_score(), Some(PLAYER_1.to_string()));
        assert_eq!(round.next_player_to_score(), Some(PLAYER_2.to_string()));
        assert_eq!(round.next_player_to_score(), Some(PLAYER_3.to_string()));
    }

    #[test]
    fn given_some_players_when_choosing_after_last_player_then_chooses_correctly() {
        let mut round = get_round();
        round.next_player_to_score();
        round.next_player_to_score();
        round.next_player_to_score();
        assert_eq!(round.next_player_to_score(), None);
        assert_eq!(round.next_player_to_score(), None);
    }

    #[test]
    fn given_words_but_no_player_chosen_when_choosing_next_word_then_chooses_correctly() {
        let mut round = get_round();
        round
            .add_words(PLAYER_1, vec!["word1".to_string(), "word2".to_string()])
            .unwrap();

        assert_eq!(round.next_word_to_score(), None);
    }

    #[test]
    fn given_all_words_are_not_used_when_choosing_next_word_then_chooses_correctly() {
        let mut round = get_round();
        round
            .add_words(PLAYER_1, vec!["word1".to_string(), "word2".to_string()])
            .unwrap();
        round.next_player_to_score();

        assert_eq!(round.next_word_to_score(), Some("word1".to_string()));
    }

    #[test]
    fn given_the_first_two_words_are_used_when_choosing_next_word_then_chooses_the_second_word() {
        let mut round = get_round();
        round
            .add_words(
                PLAYER_1,
                vec![
                    "word1".to_string(),
                    "word2".to_string(),
                    "word3".to_string(),
                ],
            )
            .unwrap();
        round.next_player_to_score();

        round.next_word_to_score();
        round.compute_score();

        round.next_word_to_score();
        round.compute_score();

        assert_eq!(round.next_word_to_score(), Some("word3".to_string()));
    }

    #[test]
    fn given_no_more_words_when_choosing_next_word_then_chooses_correctly() {
        let mut round = get_round();
        round
            .add_words(PLAYER_1, vec!["word1".to_string(), "word2".to_string()])
            .unwrap();
        round.next_player_to_score();

        round.next_word_to_score();
        round.compute_score();

        round.next_word_to_score();
        round.compute_score();

        round.next_word_to_score();
        round.compute_score();

        assert_eq!(round.next_word_to_score(), None);
    }

    #[test]
    fn player_has_word_is_true() {
        let mut round = get_round();
        round
            .add_words(
                PLAYER_1,
                vec![
                    "word1".to_string(),
                    "word2".to_string(),
                    "word3".to_string(),
                ],
            )
            .unwrap();

        assert!(round.player_has_word(PLAYER_1, "word1"));
    }

    #[test]
    fn player_has_word_is_false() {
        let mut round = get_round();
        round
            .add_words(
                PLAYER_1,
                vec![
                    "word1".to_string(),
                    "word2".to_string(),
                    "word3".to_string(),
                ],
            )
            .unwrap();

        assert!(!round.player_has_word(PLAYER_1, "word4"));
    }

    #[test]
    fn have_all_players_submitted_words_is_true() {
        let mut round = get_round();
        round
            .add_words(PLAYER_1, vec!["word1".to_string()])
            .unwrap();
        round
            .add_words(PLAYER_2, vec!["word1".to_string()])
            .unwrap();

        assert!(round
            .have_all_players_submitted_words(&vec![PLAYER_1.to_string(), PLAYER_2.to_string()]));
    }

    #[test]
    fn have_all_players_submitted_words_is_true_when_empty_words() {
        let mut round = get_round();
        round
            .add_words(PLAYER_1, vec!["".to_string()])
            .unwrap();
        round
            .add_words(PLAYER_2, vec!["".to_string()])
            .unwrap();

        assert!(round
            .have_all_players_submitted_words(&vec![PLAYER_1.to_string(), PLAYER_2.to_string()]));
    }

    #[test]
    fn have_all_players_submitted_words_is_false() {
        let mut round = get_round();
        round
            .add_words(PLAYER_1, vec!["word1".to_string()])
            .unwrap();

        assert!(!round
            .have_all_players_submitted_words(&vec![PLAYER_1.to_string(), PLAYER_2.to_string()]));
    }

    #[test]
    fn add_words_succeeds_when_unique_words() {
        let mut round = get_round();

        let result = round.add_words(
            PLAYER_1,
            vec![
                "word1".to_string(),
                "word2".to_string(),
                "".to_string(),
                "   ".to_string(),
            ],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn add_words_fails_when_repeated_words_before_normalization() {
        let mut round = get_round();

        let result = round.add_words(PLAYER_1, vec!["word1".to_string(), "word1".to_string()]);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), Error::RepeatedWords);
    }

    #[test]
    fn add_words_fails_when_repeated_words_after_normalization() {
        let mut round = get_round();

        let result = round.add_words(PLAYER_1, vec!["word1".to_string(), "  word1 ".to_string()]);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), Error::RepeatedWords);
    }

    #[test]
    fn words_are_normalized() {
        let mut round = get_round();

        round
            .add_words(
                PLAYER_1,
                vec![
                    "  wOrd1 ".to_string(),
                    "Word2".to_string(),
                    " word  34".to_string(),
                ],
            )
            .unwrap();
        let words = round.player_words.get(PLAYER_1).unwrap();

        assert_eq!(words.len(), 3);
        assert_eq!(words[0].word, "wOrd1");
        assert_eq!(words[1].word, "Word2");
        assert_eq!(words[2].word, "word  34");
    }

    #[test]
    fn empty_words_are_filtered() {
        let mut round = get_round();

        round
            .add_words(
                PLAYER_1,
                vec![
                    "word1".to_string(),
                    "".to_string(),
                    "word2".to_string(),
                    "   ".to_string(),
                    "word3".to_string(),
                ],
            )
            .unwrap();
        let words = round.player_words.get(PLAYER_1).unwrap();

        assert_eq!(words.len(), 3);
        assert_eq!(words[0].word, "word1");
        assert_eq!(words[1].word, "word2");
        assert_eq!(words[2].word, "word3");
    }

    fn get_round() -> Round {
        Round::new(
            "word",
            vec![
                PLAYER_1.to_string(),
                PLAYER_2.to_string(),
                PLAYER_3.to_string(),
            ],
        )
        .unwrap()
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
