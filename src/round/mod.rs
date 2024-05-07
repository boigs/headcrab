use std::collections::HashMap;

use crate::error::{domain_error::DomainError, Error};

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

#[derive(Default, Debug, Clone, PartialEq)]
pub struct VotingItem {
    pub player_nickname: String,
    pub word: String,
}

#[derive(Debug, Clone)]
pub struct Round {
    pub word: String,
    players: Vec<String>,
    pub player_words: HashMap<String, Vec<Word>>,
    pub player_voting_words: HashMap<String, Option<String>>,
    pub voting_item: Option<VotingItem>,
}

impl Round {
    pub fn new(word: &str, players: Vec<String>) -> Self {
        Round {
            word: word.to_string(),
            players,
            player_words: HashMap::new(),
            player_voting_words: HashMap::new(),
            voting_item: None,
        }
    }

    pub fn add_player_words(&mut self, nickname: &str, words: Vec<String>) -> Result<(), Error> {
        let normalized_words: Vec<String> = words
            .iter()
            .map(|word| word.trim().to_string())
            .filter(|word| !word.is_empty())
            .collect();
        let mut word_count: HashMap<String, u8> = HashMap::new();
        for word in normalized_words.clone() {
            let count = word_count.get(&word).unwrap_or(&0).to_owned();
            word_count.insert(word, count + 1);
        }
        let repeated_words: Vec<String> = word_count
            .iter()
            .filter(|(_, count)| **count > 1)
            .map(|(word, _)| word.to_string())
            .collect();

        if repeated_words.is_empty() {
            self.player_words.insert(
                nickname.to_string(),
                normalized_words.into_iter().map(Word::new).collect(),
            );
            Ok(())
        } else {
            Err(Error::Domain(DomainError::RepeatedWords {
                nickname: nickname.to_string(),
                repeated_words,
            }))
        }
    }

    pub fn have_all_players_submitted_words(&self, players: &[String]) -> bool {
        players
            .iter()
            .all(|player| self.player_words.contains_key(player))
    }

    pub fn next_voting_item(&mut self) -> Option<VotingItem> {
        self.voting_item = self.find_next_voting_item();
        if let Some(ref voting_item) = self.voting_item {
            self.player_voting_words.insert(
                voting_item.player_nickname.to_string(),
                Some(voting_item.word.to_string()),
            );
            for nickname in self.get_players_to_auto_skip(voting_item.clone()) {
                self.player_voting_words.insert(nickname, None);
            }
        }
        self.voting_item.clone()
    }

    fn find_next_voting_item(&self) -> Option<VotingItem> {
        self.players
            .iter()
            .flat_map(|nickname| {
                self.player_words
                    .get(nickname)
                    .unwrap_or(&vec![])
                    .iter()
                    .filter(|word| !word.is_used)
                    .map(|word| VotingItem {
                        player_nickname: nickname.to_string(),
                        word: word.word.to_string(),
                    })
                    .collect::<Vec<_>>()
            })
            .next()
    }

    fn get_players_to_auto_skip(&self, voting_item: VotingItem) -> Vec<String> {
        self.player_words
            .iter()
            .filter(|(nickname, words)| {
                **nickname != voting_item.player_nickname && words.iter().all(|word| word.is_used)
            })
            .map(|(nickname, _)| nickname.to_string())
            .collect()
    }

    pub fn set_player_voting_word(
        &mut self,
        nickname: &str,
        word: Option<String>,
    ) -> Result<(), Error> {
        match &self.voting_item {
            Some(voting_item) => {
                if voting_item.player_nickname == nickname {
                    return Err(Error::Domain(
                        DomainError::VotingItemPlayerCannotSubmitVotingWord(nickname.to_string()),
                    ));
                }
            }
            None => {
                return Err(Error::Domain(
                    DomainError::PlayerCannotSubmitVotingWordWhenVotingItemIsNone(
                        nickname.to_string(),
                    ),
                ));
            }
        }
        if !self.voting_word_exists_and_is_unused(nickname, word.clone()) {
            return Err(Error::Domain(
                DomainError::PlayerCannotSubmitNonExistingOrUsedVotingWord(nickname.to_string()),
            ));
        }

        self.player_voting_words.insert(nickname.to_string(), word);
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

    pub fn compute_score(&mut self) {
        let score = self
            .player_voting_words
            .iter()
            .filter(|(_, submission_word)| submission_word.is_some())
            .count();
        let score = if score > 1 { score } else { 0 };

        for (submission_nickname, submission_word) in &self.player_voting_words {
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
        self.player_voting_words = HashMap::default();
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        error::{domain_error::DomainError, Error},
        round::VotingItem,
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
    fn round_voting_words_are_initialized_to_empty() {
        let round = get_round_on_writing_state();

        assert!(round.player_voting_words.is_empty());
    }

    #[test]
    fn player_can_submit_voting_word() {
        let mut round = get_round_on_voting_state();
        round.next_voting_item();

        let result = round.set_player_voting_word(PLAYER_2, Some(WORD_1.to_string()));

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn player_cannot_submit_voting_word_when_current_voting_item_is_not_chosen() {
        let mut round = get_round_on_writing_state();
        round.add_player_words(PLAYER_1, words()).unwrap();

        let result = round.set_player_voting_word(PLAYER_2, Some(WORD_1.to_string()));

        assert_eq!(
            result,
            Err(Error::Domain(
                DomainError::PlayerCannotSubmitVotingWordWhenVotingItemIsNone(PLAYER_2.to_string())
            ))
        )
    }

    #[test]
    fn current_player_cannot_submit_voting_word() {
        let mut round = get_round_on_voting_state();
        round.next_voting_item();

        let result = round.set_player_voting_word(PLAYER_1, Some(WORD_2.to_string()));

        assert_eq!(
            result,
            Err(Error::Domain(
                DomainError::VotingItemPlayerCannotSubmitVotingWord(PLAYER_1.to_string())
            ))
        )
    }

    #[test]
    fn player_cannot_submit_non_existent_voting_word() {
        let mut round = get_round_on_voting_state();
        round.next_voting_item();

        let result = round.set_player_voting_word(PLAYER_2, Some("non_existing_word".to_string()));

        assert_eq!(
            result,
            Err(Error::Domain(
                DomainError::PlayerCannotSubmitNonExistingOrUsedVotingWord(PLAYER_2.to_string())
            ))
        )
    }

    #[test]
    fn player_cannot_submit_used_voting_word() {
        let mut round = get_round_on_voting_state();
        round.next_voting_item();
        round
            .set_player_voting_word(PLAYER_2, Some(WORD_1.to_string()))
            .unwrap();
        round.compute_score();

        let result = round.set_player_voting_word(PLAYER_2, Some(WORD_1.to_string()));

        assert_eq!(
            result,
            Err(Error::Domain(
                DomainError::PlayerCannotSubmitNonExistingOrUsedVotingWord(PLAYER_2.to_string())
            ))
        )
    }

    #[test]
    fn compute_score_works() {
        let mut round = get_round_on_writing_state();
        round
            .add_player_words(PLAYER_1, vec!["p1_w1".to_string(), "p1_w2".to_string()])
            .unwrap();
        round
            .add_player_words(PLAYER_2, vec!["p2_w1".to_string(), "p2_w2".to_string()])
            .unwrap();
        round
            .add_player_words(PLAYER_3, vec!["p3_w1".to_string(), "p3_w2".to_string()])
            .unwrap();

        let voting_item = round.next_voting_item().unwrap();

        assert_eq!(&voting_item.word, "p1_w1");

        round
            .set_player_voting_word(PLAYER_2, Some("p2_w1".to_string()))
            .unwrap();
        round.set_player_voting_word(PLAYER_3, None).unwrap();

        round.compute_score();

        assert_eq!(get_word(&round, PLAYER_1, "p1_w1").score, 2);
        assert_eq!(get_word(&round, PLAYER_1, "p1_w2").score, 0);
        assert_eq!(get_word(&round, PLAYER_2, "p2_w1").score, 2);
        assert_eq!(get_word(&round, PLAYER_2, "p2_w2").score, 0);
        assert_eq!(get_word(&round, PLAYER_3, "p3_w1").score, 0);
        assert_eq!(get_word(&round, PLAYER_3, "p3_w2").score, 0);
        assert!(round.player_voting_words.is_empty());
        assert!(get_word(&round, PLAYER_1, "p1_w1").is_used);
        assert!(!get_word(&round, PLAYER_1, "p1_w2").is_used);
        assert!(get_word(&round, PLAYER_2, "p2_w1").is_used);
        assert!(!get_word(&round, PLAYER_2, "p2_w2").is_used);
        assert!(!get_word(&round, PLAYER_3, "p3_w1").is_used);
        assert!(!get_word(&round, PLAYER_3, "p3_w2").is_used);
    }

    #[test]
    fn computed_score_is_0_when_players_skip_voting() {
        let mut round = get_round_on_writing_state();
        round
            .add_player_words(PLAYER_1, vec!["p1_w1".to_string()])
            .unwrap();
        round
            .add_player_words(PLAYER_2, vec!["p2_w1".to_string()])
            .unwrap();
        round
            .add_player_words(PLAYER_3, vec!["p3_w1".to_string()])
            .unwrap();

        let voting_item = round.next_voting_item().unwrap();

        assert_eq!(&voting_item.word, "p1_w1");

        round.set_player_voting_word(PLAYER_2, None).unwrap();
        round.set_player_voting_word(PLAYER_3, None).unwrap();

        round.compute_score();

        assert_eq!(get_word(&round, PLAYER_1, "p1_w1").score, 0);
    }

    #[test]
    fn choose_next_voting_item_is_none_when_no_player_words() {
        let mut round = get_round_on_writing_state();

        assert_eq!(round.next_voting_item(), None);
        assert_eq!(round.next_voting_item(), None);
    }

    #[test]
    fn choose_next_voting_item_returns_items_in_order() {
        let mut round = get_round_on_voting_state();

        assert_eq!(
            round.next_voting_item(),
            Some(VotingItem {
                player_nickname: PLAYER_1.to_string(),
                word: WORD_1.to_string()
            })
        );
        round.compute_score();
        assert_eq!(
            round.next_voting_item(),
            Some(VotingItem {
                player_nickname: PLAYER_1.to_string(),
                word: WORD_2.to_string()
            })
        );
        round.compute_score();
        assert_eq!(
            round.next_voting_item(),
            Some(VotingItem {
                player_nickname: PLAYER_2.to_string(),
                word: WORD_1.to_string()
            })
        );
        round.compute_score();
        assert_eq!(
            round.next_voting_item(),
            Some(VotingItem {
                player_nickname: PLAYER_2.to_string(),
                word: WORD_2.to_string()
            })
        );
        round.compute_score();
        assert_eq!(
            round.next_voting_item(),
            Some(VotingItem {
                player_nickname: PLAYER_3.to_string(),
                word: WORD_1.to_string()
            })
        );
        round.compute_score();
        assert_eq!(
            round.next_voting_item(),
            Some(VotingItem {
                player_nickname: PLAYER_3.to_string(),
                word: WORD_2.to_string()
            })
        );
        round.compute_score();
        assert_eq!(round.next_voting_item(), None);
        round.compute_score();
        assert_eq!(round.next_voting_item(), None);
    }

    #[test]
    fn next_voting_item_skips_first_players_when_no_words() {
        let mut round = get_round_on_writing_state();
        round.add_player_words(PLAYER_1, vec![]).unwrap();
        round.add_player_words(PLAYER_2, vec![]).unwrap();
        round.add_player_words(PLAYER_3, words()).unwrap();

        assert_eq!(
            round.next_voting_item(),
            Some(VotingItem {
                player_nickname: PLAYER_3.to_string(),
                word: WORD_1.to_string()
            })
        )
    }

    #[test]
    fn have_all_players_submitted_words_is_true() {
        let mut round = get_round_on_writing_state();
        round.add_player_words(PLAYER_1, words()).unwrap();
        round.add_player_words(PLAYER_2, words()).unwrap();

        assert!(round
            .have_all_players_submitted_words(&vec![PLAYER_1.to_string(), PLAYER_2.to_string()]));
    }

    #[test]
    fn have_all_players_submitted_words_is_true_when_empty_words() {
        let mut round = get_round_on_writing_state();
        round
            .add_player_words(PLAYER_1, vec!["".to_string()])
            .unwrap();
        round
            .add_player_words(PLAYER_2, vec!["".to_string()])
            .unwrap();

        assert!(round
            .have_all_players_submitted_words(&vec![PLAYER_1.to_string(), PLAYER_2.to_string()]));
    }

    #[test]
    fn have_all_players_submitted_words_is_false() {
        let mut round = get_round_on_writing_state();
        round.add_player_words(PLAYER_1, words()).unwrap();

        assert!(!round
            .have_all_players_submitted_words(&vec![PLAYER_1.to_string(), PLAYER_2.to_string()]));
    }

    #[test]
    fn add_words_succeeds_when_unique_words() {
        let mut round = get_round_on_writing_state();

        let result = round.add_player_words(
            PLAYER_1,
            vec![
                "word1".to_string(),
                "word2".to_string(),
                "".to_string(),
                "   ".to_string(),
            ],
        );

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn add_words_fails_when_repeated_words_before_normalization() {
        let mut round = get_round_on_writing_state();

        let result = round.add_player_words(
            PLAYER_1,
            vec![
                "word2".to_string(),
                "word".to_string(),
                "word".to_string(),
                "unique".to_string(),
                "word2".to_string(),
            ],
        );

        assert_repeated_words_error(
            result,
            PLAYER_1,
            vec!["word".to_string(), "word2".to_string()],
        );
    }

    #[test]
    fn add_words_fails_when_repeated_words_after_normalization() {
        let mut round = get_round_on_writing_state();

        let result = round.add_player_words(
            PLAYER_1,
            vec![
                " word2  ".to_string(),
                "word ".to_string(),
                "   word  ".to_string(),
                " unique".to_string(),
                " word2".to_string(),
            ],
        );

        assert_repeated_words_error(
            result,
            PLAYER_1,
            vec!["word".to_string(), "word2".to_string()],
        );
    }

    fn assert_repeated_words_error(
        result: Result<(), Error>,
        expected_nickname: &str,
        expected_repeated_words: Vec<String>,
    ) {
        assert!(result.is_err());
        let (actual_nickname, actual_repeated_words) = match result.unwrap_err() {
            Error::Domain(DomainError::RepeatedWords {
                nickname,
                repeated_words,
            }) => (nickname, repeated_words),
            _ => panic!("The error is not a DomainError::RepeatedWords error."),
        };
        assert_eq!(actual_nickname, expected_nickname);
        assert_eq!(actual_repeated_words.len(), expected_repeated_words.len());
        actual_repeated_words
            .iter()
            .for_each(|word| assert!(expected_repeated_words.contains(word)));
    }

    #[test]
    fn words_are_normalized() {
        let mut round = get_round_on_writing_state();

        round
            .add_player_words(
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
            .add_player_words(
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

    #[test]
    fn on_next_voting_item_players_without_words_auto_skip() {
        let mut round = get_round_on_writing_state();

        round.add_player_words(PLAYER_1, words()).unwrap();
        round.add_player_words(PLAYER_2, vec![]).unwrap();
        round.add_player_words(PLAYER_3, words()).unwrap();

        let _ = round.next_voting_item().unwrap();

        assert_eq!(
            round.player_voting_words.get(PLAYER_1),
            Some(&Some(WORD_1.to_string()))
        );
        assert_eq!(round.player_voting_words.get(PLAYER_2), Some(&None));
        assert_eq!(round.player_voting_words.get(PLAYER_3), None);
    }

    #[test]
    fn on_next_voting_item_players_without_words_auto_skip_for_first_players() {
        let mut round = get_round_on_writing_state();

        round.add_player_words(PLAYER_1, vec![]).unwrap();
        round.add_player_words(PLAYER_2, vec![]).unwrap();
        round.add_player_words(PLAYER_3, words()).unwrap();

        let _ = round.next_voting_item().unwrap();

        assert_eq!(round.player_voting_words.get(PLAYER_1), Some(&None));
        assert_eq!(round.player_voting_words.get(PLAYER_2), Some(&None));
        assert_eq!(
            round.player_voting_words.get(PLAYER_3),
            Some(&Some(WORD_1.to_string()))
        );
    }

    #[test]
    fn on_next_voting_item_players_with_only_used_words_auto_skip() {
        let mut round = get_round_on_writing_state();

        round.add_player_words(PLAYER_1, words()).unwrap();
        round.add_player_words(PLAYER_2, words()).unwrap();
        round.add_player_words(PLAYER_3, words()).unwrap();

        let _ = round.next_voting_item().unwrap();
        let _ = round.set_player_voting_word(PLAYER_2, None).unwrap();
        let _ = round
            .set_player_voting_word(PLAYER_3, Some(WORD_1.to_string()))
            .unwrap();
        round.compute_score();
        let _ = round.next_voting_item().unwrap();
        let _ = round.set_player_voting_word(PLAYER_2, None).unwrap();
        let _ = round
            .set_player_voting_word(PLAYER_3, Some(WORD_2.to_string()))
            .unwrap();
        round.compute_score();
        let _ = round.next_voting_item().unwrap();

        assert_eq!(round.player_voting_words.get(PLAYER_1), Some(&None));
        assert_eq!(
            round.player_voting_words.get(PLAYER_2),
            Some(&Some(WORD_1.to_string()))
        );
        assert_eq!(round.player_voting_words.get(PLAYER_3), Some(&None));
    }

    #[test]
    fn on_next_voting_item_last_word_of_current_player_is_not_auto_skipped() {
        let mut round = get_round_on_writing_state();

        round.add_player_words(PLAYER_1, words()).unwrap();
        round.add_player_words(PLAYER_2, vec![]).unwrap();
        round.add_player_words(PLAYER_3, words()).unwrap();

        let _ = round.next_voting_item().unwrap();
        assert_eq!(
            round.player_voting_words.get(PLAYER_1),
            Some(&Some(WORD_1.to_string()))
        );

        let _ = round.next_voting_item().unwrap();
        assert_eq!(
            round.player_voting_words.get(PLAYER_1),
            Some(&Some(WORD_1.to_string()))
        );
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
    }

    fn get_round_on_voting_state() -> Round {
        let mut round = get_round_on_writing_state();
        round.add_player_words(PLAYER_1, words()).unwrap();
        round.add_player_words(PLAYER_2, words()).unwrap();
        round.add_player_words(PLAYER_3, words()).unwrap();
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
