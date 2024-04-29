use std::time::Duration;

use crate::helpers::{test_app::TestApp, test_game::GameFsmState};

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

    let sate = game.players[0].start_game().await.unwrap();

    assert_eq!(sate.state, GameFsmState::PlayersSubmittingWords);
    assert_eq!(sate.rounds.len(), 1);
    assert!(!sate.rounds.first().unwrap().word.is_empty());
}

#[tokio::test]
async fn non_host_player_cannot_start_game() {
    let mut game = TestApp::create_game(GameFsmState::Lobby).await;

    let result = game.players[1].start_game().await;

    assert_eq!(result, Err("COMMAND_NOT_ALLOWED".to_string()));
}

#[tokio::test]
async fn game_cannot_be_started_with_less_than_three_players() {
    let mut game = TestApp::create_game_without_players().await;
    let _ = game.add_player("p1").await.unwrap();
    let _ = game.add_player("p2").await.unwrap();

    let result = game.players[0].start_game().await;

    assert_eq!(result, Err("NOT_ENOUGH_PLAYERS".to_string()));
    // The game is still alive and the socket of player 1 is still open
    let sate = game.players[0].receive_game_sate().await.unwrap();
    assert_eq!(sate.state, GameFsmState::Lobby);
}

#[tokio::test]
async fn game_is_still_alive_when_all_players_leave() {
    let mut game = TestApp::create_game(GameFsmState::Lobby).await;

    // Drop the players reference, so the players websockets get closed and the server disconnects the player from the game
    drop(game.players);
    game.players = vec![];

    let sate = game.add_player("p4").await.unwrap();

    assert_eq!(sate.state, GameFsmState::Lobby);
    assert_eq!(sate.players.len(), 4);
    assert!(!sate.players.get(0).unwrap().is_connected);
    assert!(!sate.players.get(0).unwrap().is_host);
    assert!(sate.players.get(3).unwrap().is_connected);
    assert!(sate.players.get(3).unwrap().is_host);
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

    let sate = game.add_player("p4").await;
    assert!(sate.is_ok());
}

#[tokio::test]
async fn when_sending_invalid_message_game_it_is_reject_but_game_is_still_alive() {
    let mut game = TestApp::create_game(GameFsmState::Lobby).await;

    let result = game.players[0]
        .send_raw_message(Message::Binary(vec![]))
        .await;
    assert_eq!(result, Err("UNPROCESSABLE_WEBSOCKET_MESSAGE".to_string()));

    let sate = game.add_player("p4").await;
    assert!(sate.is_ok());
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

    let sate = game.players[0]
        .send_custom_words(vec!["w1".to_string()])
        .await
        .unwrap();
    assert_eq!(sate.state, GameFsmState::PlayersSubmittingWords);

    let _ = game.players[1]
        .send_custom_words(vec!["w1".to_string()])
        .await
        .unwrap();
    let sate = game.players[0].receive_game_sate().await.unwrap();
    assert_eq!(sate.state, GameFsmState::PlayersSubmittingWords);

    let _ = game.players[2]
        .send_custom_words(vec!["w1".to_string()])
        .await
        .unwrap();
    let sate = game.players[0].receive_game_sate().await.unwrap();
    assert_eq!(sate.state, GameFsmState::PlayersSubmittingVotingWord);
    assert_eq!(sate.rounds.len(), 1);
}

#[tokio::test]
async fn player_visibility_of_other_players_words_is_correct() {
    let mut game = TestApp::create_game(GameFsmState::PlayersSubmittingWords).await;

    // Player 1 prespective after entering on the voting state
    let sate = game.players_send_words().await;
    assert_eq!(sate.state, GameFsmState::PlayersSubmittingVotingWord);
    // Submitted words
    let words = sate.last_round().player_words;
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
    let voting_words = sate.last_round().player_voting_words;
    assert_eq!(voting_words.len(), 1);
    let p1_word = voting_words.get(&game.players[0].nickname).unwrap().clone();
    assert_eq!(p1_word, Some("p1_w1".to_string()));
    assert!(!voting_words.contains_key(&game.players[1].nickname));
    assert!(!voting_words.contains_key(&game.players[2].nickname));

    // Player 2 prespective after sending a word
    let sate = game.players[1].send_voting_word(None).await.unwrap();
    let _ = game.players[0].receive_game_sate().await.unwrap();
    let _ = game.players[2].receive_game_sate().await.unwrap();
    let player_words = sate.last_round().player_words;
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
    let voting_words = sate.last_round().player_voting_words;
    assert_eq!(voting_words.len(), 2);
    let p1_word = voting_words.get(&game.players[0].nickname).unwrap().clone();
    assert_eq!(p1_word, Some("p1_w1".to_string()));
    let p2_word = voting_words.get(&game.players[1].nickname).unwrap().clone();
    assert_eq!(p2_word, None);
    assert!(!voting_words.contains_key(&game.players[2].nickname));

    // Player 3 prespective after sending a word
    let sate = game.players[2]
        .send_voting_word(Some("p3_w2".to_string()))
        .await
        .unwrap();
    let _ = game.players[0].receive_game_sate().await.unwrap();
    let _ = game.players[1].receive_game_sate().await.unwrap();
    // Submitted words
    let player_words = sate.last_round().player_words;
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
    let voting_words = sate.last_round().player_voting_words;
    assert_eq!(voting_words.len(), 3);
    let p1_word = voting_words.get(&game.players[0].nickname).unwrap().clone();
    assert_eq!(p1_word, Some("p1_w1".to_string()));
    let p2_word = voting_words.get(&game.players[1].nickname).unwrap().clone();
    assert_eq!(p2_word, None);
    let p3_word = voting_words.get(&game.players[2].nickname).unwrap().clone();
    assert_eq!(p3_word, Some("p3_w2".to_string()));

    // Player 2 prespective after sending a word
    let sate = game.players[1]
        .send_voting_word(Some("p2_w1".to_string()))
        .await
        .unwrap();
    let _ = game.players[0].receive_game_sate().await.unwrap();
    let _ = game.players[2].receive_game_sate().await.unwrap();
    // Submitted words
    let player_words = sate.last_round().player_words;
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
    let voting_words = sate.last_round().player_voting_words;
    assert_eq!(voting_words.len(), 3);
    let p1_word = voting_words.get(&game.players[0].nickname).unwrap().clone();
    assert_eq!(p1_word, Some("p1_w1".to_string()));
    let p2_word = voting_words.get(&game.players[1].nickname).unwrap().clone();
    assert_eq!(p2_word, Some("p2_w1".to_string()));
    let p3_word = voting_words.get(&game.players[2].nickname).unwrap().clone();
    assert_eq!(p3_word, Some("p3_w2".to_string()));

    // Advance to next voting item
    // Player 1 prespective
    let sate = game.players[0].accept_players_voting_words().await.unwrap();
    // Submitted words
    let words = sate.last_round().player_words;
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
    let voting_words = sate.last_round().player_voting_words;
    assert_eq!(voting_words.len(), 1);
    let p1_word = voting_words.get(&game.players[0].nickname).unwrap().clone();
    assert_eq!(p1_word, Some("p1_w2".to_string()));
    assert!(!voting_words.contains_key(&game.players[1].nickname));
    assert!(!voting_words.contains_key(&game.players[2].nickname));

    // Player 2 prespective
    let sate = &game.players[1].receive_game_sate().await.unwrap();
    // Submitted words
    let words = sate.last_round().player_words;
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
    let voting_words = sate.last_round().player_voting_words;
    assert_eq!(voting_words.len(), 1);
    let p1_word = voting_words.get(&game.players[0].nickname).unwrap().clone();
    assert_eq!(p1_word, Some("p1_w2".to_string()));
    assert!(!voting_words.contains_key(&game.players[1].nickname));
    assert!(!voting_words.contains_key(&game.players[2].nickname));

    // Player 3 prespective
    let sate = &game.players[2].receive_game_sate().await.unwrap();
    // Submitted words
    let words = sate.last_round().player_words;
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
    let voting_words = sate.last_round().player_voting_words;
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

    game.complete_round().await;
    let sate = game.continue_to_next_round().await;
    assert_eq!(sate.state, GameFsmState::PlayersSubmittingWords);

    game.complete_round().await;
    let sate = game.continue_to_next_round().await;
    assert_eq!(sate.state, GameFsmState::PlayersSubmittingWords);

    game.complete_round().await;
    let sate = game.continue_to_next_round().await;
    assert_eq!(sate.state, GameFsmState::EndOfGame);
}

async fn sleep(duration: Duration) {
    let mut timer = time::interval(duration);
    timer.tick().await;
    timer.tick().await;
}
