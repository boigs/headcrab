use std::time::Duration;

use crate::helpers::{
    test_app::TestApp,
    test_game::{GameFsmState, TestGame},
};

use tokio::time;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn create_game_works() {
    let _ = TestApp::create_game(GameFsmState::Lobby).await;
}

#[tokio::test]
async fn when_player_already_exists_add_player_with_same_nickname_to_game_fails() {
    let mut game = TestApp::create_game(GameFsmState::Lobby).await;
    let player_1 = game.players[0].nickname.clone();

    let result = game.add_player(&player_1).await;

    assert_eq!(result, Err("PLAYER_ALREADY_EXISTS".to_string()));
}

#[tokio::test]
async fn host_player_can_start_game() {
    let mut game = TestApp::create_game(GameFsmState::Lobby).await;

    let state = game.players[0].start_game(3).await.unwrap();

    assert_eq!(state.state, GameFsmState::PlayersSubmittingWords);
    assert_eq!(state.rounds.len(), 1);
    assert!(!state.rounds.first().unwrap().word.is_empty());
}

#[tokio::test]
async fn non_host_player_cannot_start_game() {
    let mut game = TestApp::create_game(GameFsmState::Lobby).await;

    let result = game.players[1].start_game(3).await;

    assert_eq!(result, Err("NON_HOST_PLAYER_CANNOT_START_GAME".to_string()));
}

#[tokio::test]
async fn game_cannot_be_started_with_less_than_three_players() {
    let mut game = TestApp::create_game_without_players().await;
    let _ = game.add_player("p1").await.unwrap();
    let _ = game.add_player("p2").await.unwrap();

    let result = game.players[0].start_game(3).await;

    assert_eq!(result, Err("NOT_ENOUGH_PLAYERS".to_string()));
    // The game is still alive and the socket of player 1 is still open
    let state = game.players[0].receive_game_state().await.unwrap();
    assert_eq!(state.state, GameFsmState::Lobby);
}

#[tokio::test]
async fn game_is_started_with_the_right_settings() {
    let mut game = TestApp::create_game_without_players().await;

    let _ = game.add_player("p1").await.unwrap();
    let _ = game.add_player("p2").await.unwrap();
    let state = game.add_player("p3").await.unwrap();
    assert_eq!(state.amount_of_rounds, None);

    let state = game.players[0].start_game(3).await.unwrap();
    assert_eq!(state.amount_of_rounds, Some(3));
}

#[tokio::test]
async fn game_cannot_be_started_with_less_than_1_round() {
    let mut game = TestApp::create_game(GameFsmState::Lobby).await;
    let result = game.players[0].start_game(0).await;
    assert_eq!(result, Err("NOT_ENOUGH_ROUNDS".to_string()));

    let mut game = TestApp::create_game(GameFsmState::Lobby).await;
    let result = game.players[0].start_game(-1).await;
    assert_eq!(result, Err("UNPROCESSABLE_WEBSOCKET_MESSAGE".to_string()));
}

#[tokio::test]
async fn game_is_still_alive_when_all_players_leave() {
    let mut game = TestApp::create_game(GameFsmState::Lobby).await;

    // Drop the players reference, so the players websockets get closed and the server disconnects the player from the game
    drop(game.players);
    game.players = vec![];

    let state = game.add_player("p4").await.unwrap();

    assert_eq!(state.state, GameFsmState::Lobby);
    assert_eq!(state.players.len(), 4);
    assert!(!state.players.get(0).unwrap().is_connected);
    assert!(!state.players.get(0).unwrap().is_host);
    assert!(state.players.get(3).unwrap().is_connected);
    assert!(state.players.get(3).unwrap().is_host);
}

#[tokio::test]
async fn game_is_closed_after_inactivity_timeout() {
    let mut game = TestApp::create_game(GameFsmState::Lobby).await;

    // Drop the players reference, so the players websockets get closed and the server disconnects the player from the game
    drop(game.players);
    game.players = vec![];
    // Wait until the game is closed
    sleep(game.app.inactivity_timeout + Duration::from_secs(1)).await;

    // Try to connect to the same game again
    let result = game.add_player("p4").await;
    assert_eq!(result, Err("GAME_DOES_NOT_EXIST".to_string()));
}

#[tokio::test]
async fn unknown_websocket_text_message_is_rejected_but_game_still_alive() {
    let mut game = TestApp::create_game(GameFsmState::Lobby).await;

    let result = game.players[0]
        .send_raw_message(Message::Text("invalid".to_string()))
        .await;
    assert_eq!(result, Err("UNPROCESSABLE_WEBSOCKET_MESSAGE".to_string()));

    let state = game.add_player("p4").await;
    assert!(state.is_ok());
}

#[tokio::test]
async fn when_sending_invalid_message_game_it_is_reject_but_game_is_still_alive() {
    let mut game = TestApp::create_game(GameFsmState::Lobby).await;

    let result = game.players[0]
        .send_raw_message(Message::Binary(vec![]))
        .await;
    assert_eq!(result, Err("UNPROCESSABLE_WEBSOCKET_MESSAGE".to_string()));

    let state = game.add_player("p4").await;
    assert!(state.is_ok());
}

#[tokio::test]
async fn repeated_words_are_not_allowed() {
    let mut game = TestApp::create_game(GameFsmState::PlayersSubmittingWords).await;

    let result = game.players[0]
        .send_custom_words(vec!["w1".to_string(), "w1".to_string()])
        .await;

    assert_eq!(result, Err("REPEATED_WORDS".to_string()));
}

#[tokio::test]
async fn game_goes_to_players_sending_word_submission_when_all_players_send_words() {
    let mut game = TestApp::create_game(GameFsmState::PlayersSubmittingWords).await;

    let state = game.players[0]
        .send_custom_words(vec!["w1".to_string()])
        .await
        .unwrap();
    assert_eq!(state.state, GameFsmState::PlayersSubmittingWords);

    let _ = game.players[1]
        .send_custom_words(vec!["w1".to_string()])
        .await
        .unwrap();
    let state = game.players[0].receive_game_state().await.unwrap();
    assert_eq!(state.state, GameFsmState::PlayersSubmittingWords);

    let _ = game.players[2]
        .send_custom_words(vec!["w1".to_string()])
        .await
        .unwrap();
    let state = game.players[0].receive_game_state().await.unwrap();
    assert_eq!(state.state, GameFsmState::PlayersSubmittingVotingWord);
    assert_eq!(state.rounds.len(), 1);
}

#[tokio::test]
async fn player_visibility_of_other_players_words_is_correct() {
    let mut game = TestApp::create_game(GameFsmState::PlayersSubmittingWords).await;

    // Player 1 prespective after entering on the voting state
    let state = game.players_send_words().await;
    assert_eq!(state.state, GameFsmState::PlayersSubmittingVotingWord);
    // Submitted words
    let words = state.last_round().player_words;
    assert_eq!(words.len(), 3);
    let p1_words = words.get(&game.players[0].nickname).unwrap();
    assert_eq!(p1_words.len(), 2);
    assert_eq!(p1_words.get(0).unwrap().word, "p1_w1");
    assert_eq!(p1_words.get(1).unwrap().word, "p1_w2");
    let p2_words = words.get(&game.players[1].nickname).unwrap();
    assert_eq!(p2_words.len(), 0);
    let p3_words = words.get(&game.players[2].nickname).unwrap();
    assert_eq!(p3_words.len(), 0);
    // Voting words
    let voting_words = state.last_round().player_voting_words;
    assert_eq!(voting_words.len(), 1);
    let p1_word = voting_words.get(&game.players[0].nickname).unwrap().clone();
    assert_eq!(p1_word, Some("p1_w1".to_string()));
    assert!(!voting_words.contains_key(&game.players[1].nickname));
    assert!(!voting_words.contains_key(&game.players[2].nickname));

    // Player 2 prespective after sending a word
    let state = game.players[1].send_voting_word(None).await.unwrap();
    let _ = game.players[0].receive_game_state().await.unwrap();
    let _ = game.players[2].receive_game_state().await.unwrap();
    let player_words = state.last_round().player_words;
    assert_eq!(player_words.len(), 3);
    let p1_words = player_words.get(&game.players[0].nickname).unwrap();
    assert_eq!(p1_words.len(), 2);
    assert_eq!(p1_words.get(0).unwrap().word, "p1_w1");
    assert_eq!(p1_words.get(1).unwrap().word, "p1_w2");
    let p2_words = player_words.get(&game.players[1].nickname).unwrap();
    assert_eq!(p2_words.len(), 2);
    assert_eq!(p2_words.get(0).unwrap().word, "p2_w1");
    assert_eq!(p2_words.get(1).unwrap().word, "p2_w2");
    let p3_words = player_words.get(&game.players[2].nickname).unwrap();
    assert_eq!(p3_words.len(), 0);
    // Voting words
    let voting_words = state.last_round().player_voting_words;
    assert_eq!(voting_words.len(), 2);
    let p1_word = voting_words.get(&game.players[0].nickname).unwrap().clone();
    assert_eq!(p1_word, Some("p1_w1".to_string()));
    let p2_word = voting_words.get(&game.players[1].nickname).unwrap().clone();
    assert_eq!(p2_word, None);
    assert!(!voting_words.contains_key(&game.players[2].nickname));

    // Player 3 prespective after sending a word
    let state = game.players[2]
        .send_voting_word(Some("p3_w2".to_string()))
        .await
        .unwrap();
    let _ = game.players[0].receive_game_state().await.unwrap();
    let _ = game.players[1].receive_game_state().await.unwrap();
    // Submitted words
    let player_words = state.last_round().player_words;
    assert_eq!(player_words.len(), 3);
    let p1_words = player_words.get(&game.players[0].nickname).unwrap();
    assert_eq!(p1_words.len(), 2);
    assert_eq!(p1_words.get(0).unwrap().word, "p1_w1");
    assert_eq!(p1_words.get(1).unwrap().word, "p1_w2");
    let p2_words = player_words.get(&game.players[1].nickname).unwrap();
    assert_eq!(p2_words.len(), 0);
    let p3_words = player_words.get(&game.players[2].nickname).unwrap();
    assert_eq!(p3_words.len(), 2);
    assert_eq!(p3_words.get(0).unwrap().word, "p3_w1");
    assert_eq!(p3_words.get(1).unwrap().word, "p3_w2");
    // Voting words
    let voting_words = state.last_round().player_voting_words;
    assert_eq!(voting_words.len(), 3);
    let p1_word = voting_words.get(&game.players[0].nickname).unwrap().clone();
    assert_eq!(p1_word, Some("p1_w1".to_string()));
    let p2_word = voting_words.get(&game.players[1].nickname).unwrap().clone();
    assert_eq!(p2_word, None);
    let p3_word = voting_words.get(&game.players[2].nickname).unwrap().clone();
    assert_eq!(p3_word, Some("p3_w2".to_string()));

    // Player 2 prespective after sending a word
    let state = game.players[1]
        .send_voting_word(Some("p2_w1".to_string()))
        .await
        .unwrap();
    let _ = game.players[0].receive_game_state().await.unwrap();
    let _ = game.players[2].receive_game_state().await.unwrap();
    // Submitted words
    let player_words = state.last_round().player_words;
    assert_eq!(player_words.len(), 3);
    let p1_words = player_words.get(&game.players[0].nickname).unwrap();
    assert_eq!(p1_words.len(), 2);
    assert_eq!(p1_words.get(0).unwrap().word, "p1_w1");
    assert_eq!(p1_words.get(1).unwrap().word, "p1_w2");
    let p2_words = player_words.get(&game.players[1].nickname).unwrap();
    assert_eq!(p2_words.len(), 2);
    assert_eq!(p2_words.get(0).unwrap().word, "p2_w1");
    assert_eq!(p2_words.get(1).unwrap().word, "p2_w2");
    let p3_words = player_words.get(&game.players[2].nickname).unwrap();
    assert_eq!(p3_words.len(), 0);
    // Voting words
    let voting_words = state.last_round().player_voting_words;
    assert_eq!(voting_words.len(), 3);
    let p1_word = voting_words.get(&game.players[0].nickname).unwrap().clone();
    assert_eq!(p1_word, Some("p1_w1".to_string()));
    let p2_word = voting_words.get(&game.players[1].nickname).unwrap().clone();
    assert_eq!(p2_word, Some("p2_w1".to_string()));
    let p3_word = voting_words.get(&game.players[2].nickname).unwrap().clone();
    assert_eq!(p3_word, Some("p3_w2".to_string()));

    // Advance to next voting item
    // Player 1 prespective
    let state = game.players[0].accept_players_voting_words().await.unwrap();
    // Submitted words
    let words = state.last_round().player_words;
    assert_eq!(words.len(), 3);
    let p1_words = words.get(&game.players[0].nickname).unwrap();
    assert_eq!(p1_words.len(), 2);
    assert_eq!(p1_words.get(0).unwrap().word, "p1_w1");
    assert_eq!(p1_words.get(1).unwrap().word, "p1_w2");
    let p2_words = words.get(&game.players[1].nickname).unwrap();
    assert_eq!(p2_words.len(), 1);
    assert_eq!(p2_words.get(0).unwrap().word, "p2_w1");
    let p3_words = words.get(&game.players[2].nickname).unwrap();
    assert_eq!(p3_words.len(), 1);
    assert_eq!(p3_words.get(0).unwrap().word, "p3_w2");
    // Voting words
    let voting_words = state.last_round().player_voting_words;
    assert_eq!(voting_words.len(), 1);
    let p1_word = voting_words.get(&game.players[0].nickname).unwrap().clone();
    assert_eq!(p1_word, Some("p1_w2".to_string()));
    assert!(!voting_words.contains_key(&game.players[1].nickname));
    assert!(!voting_words.contains_key(&game.players[2].nickname));

    // Player 2 prespective
    let state = &game.players[1].receive_game_state().await.unwrap();
    // Submitted words
    let words = state.last_round().player_words;
    assert_eq!(words.len(), 3);
    let p1_words = words.get(&game.players[0].nickname).unwrap();
    assert_eq!(p1_words.len(), 2);
    assert_eq!(p1_words.get(0).unwrap().word, "p1_w1");
    assert_eq!(p1_words.get(1).unwrap().word, "p1_w2");
    let p2_words = words.get(&game.players[1].nickname).unwrap();
    assert_eq!(p2_words.len(), 2);
    assert_eq!(p2_words.get(0).unwrap().word, "p2_w1");
    assert_eq!(p2_words.get(1).unwrap().word, "p2_w2");
    let p3_words = words.get(&game.players[2].nickname).unwrap();
    assert_eq!(p3_words.len(), 1);
    assert_eq!(p3_words.get(0).unwrap().word, "p3_w2");
    // Voting words
    let voting_words = state.last_round().player_voting_words;
    assert_eq!(voting_words.len(), 1);
    let p1_word = voting_words.get(&game.players[0].nickname).unwrap().clone();
    assert_eq!(p1_word, Some("p1_w2".to_string()));
    assert!(!voting_words.contains_key(&game.players[1].nickname));
    assert!(!voting_words.contains_key(&game.players[2].nickname));

    // Player 3 prespective
    let state = &game.players[2].receive_game_state().await.unwrap();
    // Submitted words
    let words = state.last_round().player_words;
    assert_eq!(words.len(), 3);
    let p1_words = words.get(&game.players[0].nickname).unwrap();
    assert_eq!(p1_words.len(), 2);
    assert_eq!(p1_words.get(0).unwrap().word, "p1_w1");
    assert_eq!(p1_words.get(1).unwrap().word, "p1_w2");
    let p2_words = words.get(&game.players[1].nickname).unwrap();
    assert_eq!(p2_words.len(), 1);
    assert_eq!(p2_words.get(0).unwrap().word, "p2_w1");
    let p3_words = words.get(&game.players[2].nickname).unwrap();
    assert_eq!(p3_words.len(), 2);
    assert_eq!(p3_words.get(0).unwrap().word, "p3_w1");
    assert_eq!(p3_words.get(1).unwrap().word, "p3_w2");
    // Voting words
    let voting_words = state.last_round().player_voting_words;
    assert_eq!(voting_words.len(), 1);
    let p1_word = voting_words.get(&game.players[0].nickname).unwrap().clone();
    assert_eq!(p1_word, Some("p1_w2".to_string()));
    assert!(!voting_words.contains_key(&game.players[1].nickname));
    assert!(!voting_words.contains_key(&game.players[2].nickname));
}

#[tokio::test]
async fn players_can_complete_a_round() {
    let mut game = TestApp::create_game(GameFsmState::PlayersSubmittingWords).await;

    game.complete_round().await;
}

#[tokio::test]
async fn players_can_complete_a_game() {
    let mut game = TestApp::create_game(GameFsmState::PlayersSubmittingWords).await;
    let mut state = GameFsmState::PlayersSubmittingWords;

    for _ in 0..TestGame::AMOUNT_OF_ROUNDS {
        game.complete_round().await;
        state = game.continue_to_next_round().await.state;
    }

    assert_eq!(state, GameFsmState::EndOfGame);
}

#[tokio::test]
async fn players_play_again_a_game() {
    let mut game = TestApp::create_game(GameFsmState::PlayersSubmittingWords).await;
    let mut state = GameFsmState::PlayersSubmittingWords;

    for _ in 0..TestGame::AMOUNT_OF_ROUNDS {
        game.complete_round().await;
        state = game.continue_to_next_round().await.state;
    }
    assert_eq!(state, GameFsmState::EndOfGame);
    state = game.play_again().await.state;

    assert_eq!(state, GameFsmState::Lobby);
}

#[tokio::test]
async fn cannot_reject_word_outside_submitting_voting_word_state() {
    let mut game = TestApp::create_game(GameFsmState::PlayersSubmittingWords).await;

    let host_player = game.players.get_mut(0).unwrap();

    let error = host_player
        .reject_matched_word("p2", "any-word")
        .await
        .unwrap_err();

    assert_eq!(&error, "INVALID_STATE_FOR_REJECTING_MATCHED_WORDS");
}

#[tokio::test]
async fn non_host_player_cannot_reject_word() {
    let mut game = TestApp::create_game(GameFsmState::PlayersSubmittingVotingWord).await;

    let non_host_player = game.players.get_mut(1).unwrap();

    let error = non_host_player
        .reject_matched_word("p1", "any-word")
        .await
        .unwrap_err();

    assert_eq!(&error, "NON_HOST_CANNOT_REJECT_MATCHED_WORDS");
}

#[tokio::test]
async fn cannot_reject_word_for_player_that_does_not_exist() {
    let mut game = TestApp::create_game(GameFsmState::PlayersSubmittingVotingWord).await;

    let host_player = game.players.get_mut(0).unwrap();

    let error = host_player
        .reject_matched_word("p99", "any-word")
        .await
        .unwrap_err();

    assert_eq!(&error, "REJECTED_MATCHED_PLAYER_DOES_NOT_EXIST");
}

#[tokio::test]
async fn cannot_reject_word_when_word_does_not_exist() {
    let mut game = TestApp::create_game(GameFsmState::PlayersSubmittingVotingWord).await;

    let host_player = game.players.get_mut(0).unwrap();

    let error = host_player
        .reject_matched_word("p2", "non-existent-word-for-p2")
        .await
        .unwrap_err();

    assert_eq!(&error, "REJECTED_MATCHED_WORD_DOES_NOT_EXIST");
}

#[tokio::test]
async fn cannot_reject_word_when_player_has_not_used_it_in_matching() {
    let mut game = TestApp::create_game(GameFsmState::PlayersSubmittingVotingWord).await;

    let rejected_word = game.players[1].words[0].clone();
    let host_player = game.players.get_mut(0).unwrap();

    let error = host_player
        .reject_matched_word("p2", &rejected_word) // p2 has not submitted matching word yet
        .await
        .unwrap_err();

    assert_eq!(&error, "REJECTED_MATCHED_WORD_WAS_NOT_PICKED_BY_PLAYER");
}

#[tokio::test]
async fn cannot_reject_word_when_player_has_voted_with_skip() {
    let mut game = TestApp::create_game(GameFsmState::PlayersSubmittingVotingWord).await;

    let rejected_word = game.players.get(1).unwrap().words[0].clone();

    let player_to_reject_word = game.players.get_mut(1).unwrap();
    let player_to_reject_word_nickname = player_to_reject_word.nickname.clone();
    player_to_reject_word.send_voting_word(None).await.unwrap();

    let _ = game.players.get_mut(0).unwrap().receive_game_state().await;
    let _ = game.players.get_mut(2).unwrap().receive_game_state().await;

    let host_player = game.players.get_mut(0).unwrap();
    let error = host_player
        .reject_matched_word(&player_to_reject_word_nickname, &rejected_word)
        .await
        .unwrap_err();

    assert_eq!(&error, "REJECTED_MATCHED_WORD_WAS_NOT_PICKED_BY_PLAYER");
}

#[tokio::test]
async fn rejected_word_is_represented_correctly_in_voting_item() {
    let mut game = TestApp::create_game(GameFsmState::PlayersSubmittingVotingWord).await;

    let rejected_word = game.players.get(1).unwrap().words[0].clone();

    let player_to_reject_word = game.players.get_mut(1).unwrap();
    let player_to_reject_word_nickname = player_to_reject_word.nickname.clone();
    let state = player_to_reject_word
        .send_voting_word(Some(rejected_word.clone()))
        .await
        .unwrap();

    assert_eq!(
        state
            .rounds
            .last()
            .unwrap()
            .player_voting_words
            .get("p2")
            .unwrap(),
        &Some(rejected_word.clone()),
    );

    let _ = game.players.get_mut(0).unwrap().receive_game_state().await;
    let _ = game.players.get_mut(2).unwrap().receive_game_state().await;

    let host_player = game.players.get_mut(0).unwrap();
    let state = host_player
        .reject_matched_word(&player_to_reject_word_nickname, &rejected_word)
        .await
        .unwrap();

    let voting_item = state.rounds.last().unwrap().voting_item.as_ref().unwrap();
    let p2_rejected_words = voting_item
        .rejected_matches
        .get(&player_to_reject_word_nickname)
        .unwrap();

    assert!(p2_rejected_words.contains(&rejected_word));
}

#[tokio::test]
async fn player_matched_word_becomes_none_in_match_after_being_rejected() {
    let mut game = TestApp::create_game(GameFsmState::PlayersSubmittingVotingWord).await;

    let rejected_word = game.players.get(1).unwrap().words[0].clone();

    let player_to_reject_word = game.players.get_mut(1).unwrap();
    let player_to_reject_word_nickname = player_to_reject_word.nickname.clone();
    let state = player_to_reject_word
        .send_voting_word(Some(rejected_word.clone()))
        .await
        .unwrap();

    assert_eq!(
        state
            .rounds
            .last()
            .unwrap()
            .player_voting_words
            .get("p2")
            .unwrap(),
        &Some(rejected_word.clone()),
    );

    let _ = game.players.get_mut(0).unwrap().receive_game_state().await;
    let _ = game.players.get_mut(2).unwrap().receive_game_state().await;

    let host_player = game.players.get_mut(0).unwrap();
    let state = host_player
        .reject_matched_word(&player_to_reject_word_nickname, &rejected_word)
        .await
        .unwrap();

    let player_matched_words = &state.rounds.last().unwrap().player_voting_words;
    let p2_matched_word = player_matched_words
        .get(&player_to_reject_word_nickname)
        .unwrap();

    assert_eq!(p2_matched_word, &None);
}

#[tokio::test]
async fn cannot_reject_word_if_player_switched_to_different_one() {
    let mut game = TestApp::create_game(GameFsmState::PlayersSubmittingVotingWord).await;

    let some_word = game.players.get(1).unwrap().words[0].clone();
    let word_to_reject = game.players.get(1).unwrap().words[1].clone();

    {
        let player_to_reject_word = game.players.get_mut(1).unwrap();
        player_to_reject_word
            .send_voting_word(Some(some_word.clone()))
            .await
            .unwrap();
    }

    let _ = game.players.get_mut(0).unwrap().receive_game_state().await;
    let _ = game.players.get_mut(2).unwrap().receive_game_state().await;

    {
        let player_to_reject_word = game.players.get_mut(1).unwrap();
        player_to_reject_word
            .send_voting_word(Some(word_to_reject.clone()))
            .await
            .unwrap();
    }

    let _ = game.players.get_mut(0).unwrap().receive_game_state().await;
    let _ = game.players.get_mut(2).unwrap().receive_game_state().await;

    let host_player = game.players.get_mut(0).unwrap();
    let error = host_player
        .reject_matched_word("p2", &some_word)
        .await
        .unwrap_err();

    assert_eq!(&error, "REJECTED_MATCHED_WORD_WAS_NOT_PICKED_BY_PLAYER");
}

#[tokio::test]
async fn cannot_resubmit_word_that_was_rejected_previously() {
    let mut game = TestApp::create_game(GameFsmState::PlayersSubmittingVotingWord).await;

    let rejected_word = game.players.get(1).unwrap().words[0].clone();

    let player_to_reject_word = game.players.get_mut(1).unwrap();

    player_to_reject_word
        .send_voting_word(Some(rejected_word.clone()))
        .await
        .unwrap();

    let _ = game.players.get_mut(0).unwrap().receive_game_state().await;
    let _ = game.players.get_mut(2).unwrap().receive_game_state().await;
    let host_player = game.players.get_mut(0).unwrap();

    host_player
        .reject_matched_word("p2", &rejected_word)
        .await
        .unwrap();

    let _ = game.players.get_mut(1).unwrap().receive_game_state().await;
    let _ = game.players.get_mut(2).unwrap().receive_game_state().await;

    let player_to_reject_word = game.players.get_mut(1).unwrap();

    let error = player_to_reject_word
        .send_voting_word(Some(rejected_word.clone()))
        .await
        .unwrap_err();

    assert_eq!(&error, "CANNOT_RESUBMIT_REJECTED_MATCHED_WORD");
}

async fn sleep(duration: Duration) {
    let mut timer = time::interval(duration);
    timer.tick().await;
    timer.tick().await;
}
