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

const CURRENT_PLAYER_CANNOT_SUBMIT_VOTING_WORD: &str =
    "The current player cannot submit a voting word";
const PLAYER_CANNOT_SUBMIT_NON_EXISTING_OR_USED_VOTING_WORD: &str =
    "The player cannot submit a non-existing or an already used word for voting";
const PLAYER_CANNOT_SUBMIT_VOTING_WORD_WHEN_CURRENT_VOTING_ITEM_IS_NOT_CHOSEN: &str =
    "The player cannot submit a voting word when the current voting item is not chosen";

impl Round {
    pub fn new(word: &str, players: Vec<String>) -> Self {
        Round {
            word: word.to_string(),
            player_words: HashMap::new(),
            players,
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

    pub fn next_voting_item(&mut self) -> Option<VotingItem> {
        for player in &self.players {
            if let Some(words) = self.player_words.get_mut(player) {
                for word in words {
                    if !word.is_used {
                        word.is_used = true;
                        self.player_voting_words
                            .insert(player.to_string(), Some(word.word.clone()));
                        self.voting_item = Some(VotingItem {
                            player_nickname: player.clone(),
                            word: word.word.clone(),
                        });
                        return self.voting_item.clone();
                    }
                }
            }
        }
        self.voting_item = None;
        self.voting_item.clone()
    }

    pub fn add_player_voting_word(
        &mut self,
        nickname: &str,
        voting_word: Option<String>,
    ) -> Result<(), Error> {
        match &self.voting_item {
            Some(voting_item) => {
                if voting_item.player_nickname == nickname {
                    return Err(Error::CommandNotAllowed(
                        CURRENT_PLAYER_CANNOT_SUBMIT_VOTING_WORD.to_owned(),
                    ));
                }
            }
            None => {
                return Err(Error::CommandNotAllowed(
                    PLAYER_CANNOT_SUBMIT_VOTING_WORD_WHEN_CURRENT_VOTING_ITEM_IS_NOT_CHOSEN
                        .to_owned(),
                ));
            }
        }

        if !self.voting_word_exists_and_is_unused(nickname, voting_word.clone()) {
            return Err(Error::CommandNotAllowed(
                PLAYER_CANNOT_SUBMIT_NON_EXISTING_OR_USED_VOTING_WORD.to_owned(),
            ));
        }

        self.player_voting_words
            .insert(nickname.to_string(), voting_word);
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
        error::Error,
        round::{
            VotingItem, CURRENT_PLAYER_CANNOT_SUBMIT_VOTING_WORD,
            PLAYER_CANNOT_SUBMIT_NON_EXISTING_OR_USED_VOTING_WORD,
            PLAYER_CANNOT_SUBMIT_VOTING_WORD_WHEN_CURRENT_VOTING_ITEM_IS_NOT_CHOSEN,
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
    fn round_voting_words_are_initialized_to_empty() {
        let round = get_round_on_writing_state();

        assert!(round.player_voting_words.is_empty());
    }

    #[test]
    fn player_can_submit_voting_word() {
        let mut round = get_round_on_voting_state();
        round.next_voting_item();

        let result = round.add_player_voting_word(PLAYER_2, Some(WORD_1.to_string()));

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn player_cannot_submit_voting_word_when_current_voting_item_is_not_chosen() {
        let mut round = get_round_on_writing_state();
        round.add_player_words(PLAYER_1, words()).unwrap();

        let result = round.add_player_voting_word(PLAYER_2, Some(WORD_1.to_string()));

        assert_eq!(
            result,
            Err(Error::CommandNotAllowed(
                PLAYER_CANNOT_SUBMIT_VOTING_WORD_WHEN_CURRENT_VOTING_ITEM_IS_NOT_CHOSEN.to_owned()
            ))
        )
    }

    #[test]
    fn current_player_cannot_submit_voting_word() {
        let mut round = get_round_on_voting_state();
        round.next_voting_item();

        let result = round.add_player_voting_word(PLAYER_1, Some(WORD_2.to_string()));

        assert_eq!(
            result,
            Err(Error::CommandNotAllowed(
                CURRENT_PLAYER_CANNOT_SUBMIT_VOTING_WORD.to_owned()
            ))
        )
    }

    #[test]
    fn player_cannot_submit_non_existent_voting_word() {
        let mut round = get_round_on_voting_state();
        round.next_voting_item();

        let result = round.add_player_voting_word(PLAYER_2, Some("non_existing_word".to_string()));

        assert_eq!(
            result,
            Err(Error::CommandNotAllowed(
                PLAYER_CANNOT_SUBMIT_NON_EXISTING_OR_USED_VOTING_WORD.to_owned()
            ))
        )
    }

    #[test]
    fn player_cannot_submit_used_voting_word() {
        let mut round = get_round_on_voting_state();
        round.next_voting_item();
        round
            .add_player_voting_word(PLAYER_2, Some(WORD_1.to_string()))
            .unwrap();
        round.compute_score();

        let result = round.add_player_voting_word(PLAYER_2, Some(WORD_1.to_string()));

        assert_eq!(
            result,
            Err(Error::CommandNotAllowed(
                PLAYER_CANNOT_SUBMIT_NON_EXISTING_OR_USED_VOTING_WORD.to_owned()
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

        round.next_voting_item().unwrap();

        round
            .add_player_voting_word(PLAYER_2, Some("p2_w1".to_string()))
            .unwrap();
        round.add_player_voting_word(PLAYER_3, None).unwrap();

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
        assert_eq!(
            round.next_voting_item(),
            Some(VotingItem {
                player_nickname: PLAYER_1.to_string(),
                word: WORD_2.to_string()
            })
        );
        assert_eq!(
            round.next_voting_item(),
            Some(VotingItem {
                player_nickname: PLAYER_2.to_string(),
                word: WORD_1.to_string()
            })
        );
        assert_eq!(
            round.next_voting_item(),
            Some(VotingItem {
                player_nickname: PLAYER_2.to_string(),
                word: WORD_2.to_string()
            })
        );
        assert_eq!(
            round.next_voting_item(),
            Some(VotingItem {
                player_nickname: PLAYER_3.to_string(),
                word: WORD_1.to_string()
            })
        );
        assert_eq!(
            round.next_voting_item(),
            Some(VotingItem {
                player_nickname: PLAYER_3.to_string(),
                word: WORD_2.to_string()
            })
        );
        assert_eq!(round.next_voting_item(), None);
        assert_eq!(round.next_voting_item(), None);
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

        let result = round.add_player_words(PLAYER_1, vec!["word".to_string(), "word".to_string()]);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::RepeatedWords);
    }

    #[test]
    fn add_words_fails_when_repeated_words_after_normalization() {
        let mut round = get_round_on_writing_state();

        let result =
            round.add_player_words(PLAYER_1, vec!["word".to_string(), "  word ".to_string()]);

        assert_eq!(result, Err(Error::RepeatedWords));
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
