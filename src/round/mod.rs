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

const CURRENT_PLAYER_CANNOT_SUBMIT_VOTING_WORD: &str =
    "The current player cannot submit a voting word";
const PLAYER_CANNOT_SUBMIT_NON_EXISTING_OR_USED_VOTING_WORD: &str =
    "The player cannot submit a non-existing or an already used word for voting";
const PLAYER_CANNOT_SUBMIT_VOTING_WORD_WHEN_CURRENT_PLAYER_IS_NOT_CHOSEN: &str =
    "The player cannot submit a voting word when the current player is not chosen";
const PLAYER_CANNOT_SUBMIT_VOTING_WORD_WHEN_CURRENT_WORD_IS_NOT_CHOSEN: &str =
    "The player cannot submit a voting word when the current word is not chosen";

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
            let words = self.player_words.get_mut(&nickname)?;
            let next_word = words.iter_mut().find(|word| !word.is_used)?;
            next_word.is_used = true;
            Some((nickname, next_word.word.to_string()))
        };

        if let Some((nickname, next_word)) = nickname_and_next_word.clone() {
            self.score
                .player_word_submission
                .insert(nickname.to_string(), Some(next_word));
        }

        self.score.current_word = nickname_and_next_word.map(|(_, next_word)| next_word);
        self.score.current_word.clone()
    }

    pub fn add_player_word_submission(
        &mut self,
        nickname: &str,
        word: Option<String>,
    ) -> Result<(), Error> {
        match self.score.clone().current_player {
            Some((_, player)) => {
                if player == nickname {
                    return Err(Error::CommandNotAllowed(
                        CURRENT_PLAYER_CANNOT_SUBMIT_VOTING_WORD.to_owned(),
                    ));
                }
            }
            None => {
                return Err(Error::CommandNotAllowed(
                    PLAYER_CANNOT_SUBMIT_VOTING_WORD_WHEN_CURRENT_PLAYER_IS_NOT_CHOSEN.to_owned(),
                ));
            }
        }
        if self.score.current_word.is_none() {
            return Err(Error::CommandNotAllowed(
                PLAYER_CANNOT_SUBMIT_VOTING_WORD_WHEN_CURRENT_WORD_IS_NOT_CHOSEN.to_owned(),
            ));
        }
        if !self.voting_word_exists_and_is_unused(nickname, word.clone()) {
            return Err(Error::CommandNotAllowed(
                PLAYER_CANNOT_SUBMIT_NON_EXISTING_OR_USED_VOTING_WORD.to_owned(),
            ));
        }

        self.score
            .player_word_submission
            .insert(nickname.to_string(), word);
        Ok(())
    }

    fn voting_word_exists_and_is_unused(
        &self,
        nickname: &str,
        voting_word: Option<String>,
    ) -> bool {
        match voting_word {
            Some(voting_word) => self
                .player_words
                .get(nickname)
                .and_then(|words| {
                    words
                        .iter()
                        .find(|word| word.word == voting_word && !word.is_used)
                })
                .is_some(),
            None => true,
        }
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
    use crate::{
        error::Error,
        round::{
            CURRENT_PLAYER_CANNOT_SUBMIT_VOTING_WORD,
            PLAYER_CANNOT_SUBMIT_NON_EXISTING_OR_USED_VOTING_WORD,
            PLAYER_CANNOT_SUBMIT_VOTING_WORD_WHEN_CURRENT_PLAYER_IS_NOT_CHOSEN,
            PLAYER_CANNOT_SUBMIT_VOTING_WORD_WHEN_CURRENT_WORD_IS_NOT_CHOSEN,
        },
    };

    use super::{Round, Word};

    static PLAYER_1: &str = "p1";
    static PLAYER_2: &str = "p2";
    static PLAYER_3: &str = "p3";

    static WORD_1: &str = "w1";
    static WORD_2: &str = "w2";
    fn words() -> Vec<String> {
        vec![WORD_1, WORD_2]
            .iter()
            .map(|word| word.to_string())
            .collect()
    }

    #[test]
    fn player_can_send_voting_word() {
        let mut round = get_round_on_voting_state();

        let result = round.add_player_word_submission(PLAYER_2, Some(WORD_1.to_string()));

        assert!(result.is_ok());
    }

    #[test]
    fn player_cannot_submit_word_when_current_player_is_not_chosen() {
        let mut round = get_round_on_writing_state();
        round.add_words(PLAYER_1, words()).unwrap();

        let result = round.add_player_word_submission(PLAYER_2, Some(WORD_1.to_string()));

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            Error::CommandNotAllowed(
                PLAYER_CANNOT_SUBMIT_VOTING_WORD_WHEN_CURRENT_PLAYER_IS_NOT_CHOSEN.to_owned()
            )
        )
    }

    #[test]
    fn player_cannot_submit_word_when_current_word_is_not_chosen() {
        let mut round = get_round_on_writing_state();
        round.add_words(PLAYER_1, words()).unwrap();
        round.next_player_to_score().unwrap();

        let result = round.add_player_word_submission(PLAYER_2, Some(WORD_1.to_string()));

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            Error::CommandNotAllowed(
                PLAYER_CANNOT_SUBMIT_VOTING_WORD_WHEN_CURRENT_WORD_IS_NOT_CHOSEN.to_owned()
            )
        )
    }

    #[test]
    fn current_player_cannot_submit_word() {
        let mut round = get_round_on_voting_state();

        let result = round.add_player_word_submission(PLAYER_1, Some(WORD_2.to_string()));

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            Error::CommandNotAllowed(CURRENT_PLAYER_CANNOT_SUBMIT_VOTING_WORD.to_owned())
        )
    }

    #[test]
    fn player_cannot_submit_non_existent_word() {
        let mut round = get_round_on_voting_state();

        let result =
            round.add_player_word_submission(PLAYER_2, Some("non_existing_word".to_string()));

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            Error::CommandNotAllowed(
                PLAYER_CANNOT_SUBMIT_NON_EXISTING_OR_USED_VOTING_WORD.to_owned()
            )
        )
    }

    // TODO: fix
    #[test]
    fn player_cannot_submit_used_word() {
        let mut round = get_round_on_voting_state();
        round
            .add_player_word_submission(PLAYER_2, Some(WORD_1.to_string()))
            .unwrap();
        round.compute_score();

        let result = round.add_player_word_submission(PLAYER_2, Some(WORD_1.to_string()));

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            Error::CommandNotAllowed(
                PLAYER_CANNOT_SUBMIT_NON_EXISTING_OR_USED_VOTING_WORD.to_owned()
            )
        )
    }

    #[test]
    fn compute_score_works() {
        let mut round = get_round_on_writing_state();
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
        round.next_player_to_score().unwrap();
        round.next_word_to_score().unwrap();
        round
            .add_player_word_submission(PLAYER_2, Some("p2_word1".to_string()))
            .unwrap();
        round.add_player_word_submission(PLAYER_3, None).unwrap();

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
        let result = Round::new(
            "word",
            vec![
                PLAYER_1.to_string(),
                PLAYER_2.to_string(),
                PLAYER_3.to_string(),
            ],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn given_create_round_when_less_than_three_players_then_error() {
        let result = Round::new("word", vec![PLAYER_1.to_string()]);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            Error::Internal("Cannot create a round for less than 3 players".to_string())
        );
    }

    #[test]
    fn given_some_players_when_choosing_next_player_then_chooses_correctly() {
        let mut round = get_round_on_writing_state();

        assert_eq!(round.next_player_to_score(), Some(PLAYER_1.to_string()));
        assert_eq!(round.next_player_to_score(), Some(PLAYER_2.to_string()));
        assert_eq!(round.next_player_to_score(), Some(PLAYER_3.to_string()));
    }

    #[test]
    fn given_some_players_when_choosing_after_last_player_then_chooses_correctly() {
        let mut round = get_round_on_writing_state();
        round.next_player_to_score();
        round.next_player_to_score();
        round.next_player_to_score();

        assert_eq!(round.next_player_to_score(), None);
        assert_eq!(round.next_player_to_score(), None);
    }

    #[test]
    fn when_current_player_is_none_next_word_to_score_is_none() {
        let mut round = get_round_on_writing_state();
        round.add_words(PLAYER_1, words()).unwrap();

        assert_eq!(round.next_word_to_score(), None);
    }

    #[test]
    fn given_all_words_are_not_used_when_choosing_next_word_then_chooses_correctly() {
        let mut round = get_round_on_writing_state();
        round.add_words(PLAYER_1, words()).unwrap();
        round.next_player_to_score();

        assert_eq!(round.next_word_to_score(), Some(WORD_1.to_string()));
        let word = round
            .player_words
            .get(PLAYER_1)
            .unwrap()
            .iter()
            .find(|word| word.word == WORD_1)
            .unwrap();
        assert!(word.is_used);
    }

    #[test]
    fn given_the_first_two_words_are_used_when_choosing_next_word_then_chooses_the_second_word() {
        let mut round = get_round_on_writing_state();
        round
            .add_words(
                PLAYER_1,
                vec![
                    WORD_1.to_string(),
                    WORD_2.to_string(),
                    "last_word".to_string(),
                ],
            )
            .unwrap();
        round.next_player_to_score();

        round.next_word_to_score();
        round.compute_score();

        round.next_word_to_score();
        round.compute_score();

        assert_eq!(round.next_word_to_score(), Some("last_word".to_string()));
    }

    #[test]
    fn given_no_more_words_when_choosing_next_word_then_chooses_correctly() {
        let mut round = get_round_on_writing_state();
        round.add_words(PLAYER_1, words()).unwrap();
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
    fn have_all_players_submitted_words_is_true() {
        let mut round = get_round_on_writing_state();
        round.add_words(PLAYER_1, words()).unwrap();
        round.add_words(PLAYER_2, words()).unwrap();

        assert!(round
            .have_all_players_submitted_words(&vec![PLAYER_1.to_string(), PLAYER_2.to_string()]));
    }

    #[test]
    fn have_all_players_submitted_words_is_true_when_empty_words() {
        let mut round = get_round_on_writing_state();
        round.add_words(PLAYER_1, vec!["".to_string()]).unwrap();
        round.add_words(PLAYER_2, vec!["".to_string()]).unwrap();

        assert!(round
            .have_all_players_submitted_words(&vec![PLAYER_1.to_string(), PLAYER_2.to_string()]));
    }

    #[test]
    fn have_all_players_submitted_words_is_false() {
        let mut round = get_round_on_writing_state();
        round.add_words(PLAYER_1, words()).unwrap();

        assert!(!round
            .have_all_players_submitted_words(&vec![PLAYER_1.to_string(), PLAYER_2.to_string()]));
    }

    #[test]
    fn add_words_succeeds_when_unique_words() {
        let mut round = get_round_on_writing_state();

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
        let mut round = get_round_on_writing_state();

        let result = round.add_words(PLAYER_1, vec!["word".to_string(), "word".to_string()]);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::RepeatedWords);
    }

    #[test]
    fn add_words_fails_when_repeated_words_after_normalization() {
        let mut round = get_round_on_writing_state();

        let result = round.add_words(PLAYER_1, vec!["word".to_string(), "  word ".to_string()]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::RepeatedWords);
    }

    #[test]
    fn words_are_normalized() {
        let mut round = get_round_on_writing_state();

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
        let mut round = get_round_on_writing_state();

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

    fn get_round_on_writing_state() -> Round {
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

    fn get_round_on_voting_state() -> Round {
        let mut round = get_round_on_writing_state();
        round.add_words(PLAYER_1, words()).unwrap();
        round.add_words(PLAYER_2, words()).unwrap();
        round.add_words(PLAYER_3, words()).unwrap();
        round.next_player_to_score().unwrap();
        round.next_word_to_score().unwrap();
        round
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
