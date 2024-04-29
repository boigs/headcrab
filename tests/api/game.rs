use std::{collections::HashMap, time::Duration};

use crate::helpers::spawn_app;
use futures_util::{
    stream::{SplitSink, SplitStream, StreamExt},
    SinkExt,
};
use serde::{Deserialize, Serialize};
use tokio::{net::TcpStream, time};
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

#[tokio::test]
async fn create_game_works() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let _ = create_game(&app.base_address, client, GameFsmState::Lobby).await;
}

#[tokio::test]
async fn when_player_already_exists_add_player_with_same_nickname_to_game_fails() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let game = create_game(&app.base_address, client, GameFsmState::Lobby).await;

    let mut player = add_player(&app.base_address, &game.id, &game.player_1.nickname)
        .await
        .unwrap();

    let result: Result<GameState, String> = player.receive_game_sate().await;
    assert_eq!(result.unwrap_err(), "PLAYER_ALREADY_EXISTS");
}

#[tokio::test]
async fn host_player_can_start_game() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let mut game = create_game(&app.base_address, client, GameFsmState::Lobby).await;

    let game_state = game.player_1.start_game().await.unwrap();

    assert_eq!(game_state.state, "PlayersSubmittingWords");
    assert_eq!(game_state.rounds.len(), 1);
    assert!(!game_state.rounds.first().unwrap().word.is_empty());
}

#[tokio::test]
async fn non_host_player_cannot_start_game() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let mut game = create_game(&app.base_address, client, GameFsmState::Lobby).await;

    let result = game.player_2.start_game().await;

    assert_eq!(result.unwrap_err(), "COMMAND_NOT_ALLOWED");
}

#[tokio::test]
async fn game_cannot_be_started_with_less_than_three_players() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let game_id = create_game_without_players(&app.base_address, client).await;

    let mut player_1 = add_player(&app.base_address, &game_id, "p1").await.unwrap();
    let _ = player_1.receive_game_sate().await;
    let _player_2 = add_player(&app.base_address, &game_id, "p2").await.unwrap();
    // Skip the second GameState after player 2 joins
    let _ = player_1.receive_game_sate().await;

    let result = player_1.start_game().await;

    assert_eq!(result.unwrap_err(), "NOT_ENOUGH_PLAYERS");
    // The game is still alive and the socket of player 1 is still open
    let game_state = player_1.receive_game_sate().await.unwrap();
    assert_eq!(game_state.state, "Lobby");
}

#[tokio::test]
async fn game_is_still_alive_when_all_players_leave() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let game = create_game(&app.base_address, client, GameFsmState::Lobby).await;

    // Drop the game reference, so the players websockets get closed and the server disconnects the player from the game
    let game_id = game.id.clone();
    drop(game);

    let mut player = add_player(&app.base_address, &game_id, "p4").await.unwrap();

    let game_state = player.receive_game_sate().await.unwrap();
    assert!(game_state.state.eq("Lobby"));
    assert_eq!(game_state.players.len(), 4);
    assert!(!game_state.players.get(0).unwrap().is_connected);
    assert!(!game_state.players.get(0).unwrap().is_host);
    assert!(game_state.players.get(3).unwrap().is_connected);
    assert!(game_state.players.get(3).unwrap().is_host);
}

#[tokio::test]
async fn game_is_closed_after_inactivity_timeout() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let game = create_game(&app.base_address, client, GameFsmState::Lobby).await;

    // Drop the game reference, so the players websockets get closed and the server disconnects the player from the game
    let game_id = game.id.clone();
    drop(game);
    // Wait until the game is closed
    sleep(app.inactivity_timeout + Duration::from_secs(1)).await;

    // Try to connect to the same game again
    let mut player = add_player(&app.base_address, &game_id, "p4").await.unwrap();
    let result = player.receive_game_sate().await;
    assert_eq!(result.unwrap_err(), "GAME_DOES_NOT_EXIST");
}

#[tokio::test]
async fn unknown_websocket_text_message_is_rejected_but_game_still_alive() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let mut game = create_game(&app.base_address, client, GameFsmState::Lobby).await;

    let result = game
        .player_1
        .send_raw_message(Message::Text("invalid".to_string()))
        .await;
    assert_eq!(result.unwrap_err(), "UNPROCESSABLE_WEBSOCKET_MESSAGE");

    let mut player = add_player(&app.base_address, &game.id, "p4").await.unwrap();
    assert!(player.receive_game_sate().await.is_ok());
}

#[tokio::test]
async fn when_sending_invalid_message_game_it_is_reject_but_game_is_still_alive() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let mut game: GameTest = create_game(&app.base_address, client, GameFsmState::Lobby).await;

    let result = game
        .player_1
        .send_raw_message(Message::Binary(vec![]))
        .await;
    assert_eq!(result.unwrap_err(), "UNPROCESSABLE_WEBSOCKET_MESSAGE");

    let mut player = add_player(&app.base_address, &game.id, "p4").await.unwrap();
    assert!(player.receive_game_sate().await.is_ok());
}

#[tokio::test]
async fn repeated_words_are_not_allowed() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let mut game: GameTest = create_game(
        &app.base_address,
        client,
        GameFsmState::PlayersSubmittingWords,
    )
    .await;

    let result = game
        .player_1
        .send_words(vec!["w1".to_string(), "w1".to_string()])
        .await;

    assert_eq!(result.unwrap_err(), "REPEATED_WORDS");
}

#[tokio::test]
async fn game_goes_to_players_sending_word_submission_when_all_players_send_words() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let mut game: GameTest = create_game(
        &app.base_address,
        client,
        GameFsmState::PlayersSubmittingWords,
    )
    .await;

    let game_state = game
        .player_1
        .send_words(vec!["w1".to_string()])
        .await
        .unwrap();
    assert_eq!(game_state.state, "PlayersSubmittingWords");

    let _ = game
        .player_2
        .send_words(vec!["w1".to_string()])
        .await
        .unwrap();
    let game_state = game.player_1.receive_game_sate().await.unwrap();
    assert_eq!(game_state.state, "PlayersSubmittingWords");

    let _ = game
        .player_3
        .send_words(vec!["w1".to_string()])
        .await
        .unwrap();
    let game_state = game.player_1.receive_game_sate().await.unwrap();
    assert_eq!(game_state.state, "PlayersSubmittingVotingWord");
    assert_eq!(game_state.rounds.len(), 1);
}

#[tokio::test]
async fn player_visibility_of_other_players_words_is_correct() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let mut game: GameTest = create_game(
        &app.base_address,
        client,
        GameFsmState::PlayersSubmittingWords,
    )
    .await;

    // Player 1 prespective after entering on the voting state
    let game_state = game.players_send_words().await;
    assert_eq!(game_state.state, "PlayersSubmittingVotingWord");
    // Submitted words
    let words = game_state.last_round().player_words;
    assert_eq!(words.len(), 3);
    let p1_words = words.get(&game.player_1.nickname).unwrap();
    assert_eq!(p1_words.len(), 2);
    assert_eq!(p1_words.get(0).unwrap().word, "p1_w1");
    assert_eq!(p1_words.get(1).unwrap().word, "p1_w2");
    let p2_words = words.get(&game.player_2.nickname).unwrap();
    assert_eq!(p2_words.len(), 0);
    let p3_words = words.get(&game.player_3.nickname).unwrap();
    assert_eq!(p3_words.len(), 0);
    // Voting words
    let voting_words = game_state.last_round().player_voting_words;
    assert_eq!(voting_words.len(), 1);
    let p1_word = voting_words.get(&game.player_1.nickname).unwrap().clone();
    assert_eq!(p1_word, Some("p1_w1".to_string()));
    assert!(!voting_words.contains_key(&game.player_2.nickname));
    assert!(!voting_words.contains_key(&game.player_3.nickname));

    // Player 2 prespective after sending a word
    let game_state = game.player_2.send_voting_word(None).await.unwrap();
    let _ = game.player_1.receive_game_sate().await.unwrap();
    let _ = game.player_3.receive_game_sate().await.unwrap();
    let player_words = game_state.last_round().player_words;
    assert_eq!(player_words.len(), 3);
    let p1_words = player_words.get(&game.player_1.nickname).unwrap();
    assert_eq!(p1_words.len(), 2);
    assert_eq!(p1_words.get(0).unwrap().word, "p1_w1");
    assert_eq!(p1_words.get(1).unwrap().word, "p1_w2");
    let p2_words = player_words.get(&game.player_2.nickname).unwrap();
    assert_eq!(p2_words.len(), 2);
    assert_eq!(p2_words.get(0).unwrap().word, "p2_w1");
    assert_eq!(p2_words.get(1).unwrap().word, "p2_w2");
    let p3_words = player_words.get(&game.player_3.nickname).unwrap();
    assert_eq!(p3_words.len(), 0);
    // Voting words
    let voting_words = game_state.last_round().player_voting_words;
    assert_eq!(voting_words.len(), 2);
    let p1_word = voting_words.get(&game.player_1.nickname).unwrap().clone();
    assert_eq!(p1_word, Some("p1_w1".to_string()));
    let p2_word = voting_words.get(&game.player_2.nickname).unwrap().clone();
    assert_eq!(p2_word, None);
    assert!(!voting_words.contains_key(&game.player_3.nickname));

    // Player 3 prespective after sending a word
    let game_state = game
        .player_3
        .send_voting_word(Some("p3_w2".to_string()))
        .await
        .unwrap();
    let _ = game.player_1.receive_game_sate().await.unwrap();
    let _ = game.player_2.receive_game_sate().await.unwrap();
    // Submitted words
    let player_words = game_state.last_round().player_words;
    assert_eq!(player_words.len(), 3);
    let p1_words = player_words.get(&game.player_1.nickname).unwrap();
    assert_eq!(p1_words.len(), 2);
    assert_eq!(p1_words.get(0).unwrap().word, "p1_w1");
    assert_eq!(p1_words.get(1).unwrap().word, "p1_w2");
    let p2_words = player_words.get(&game.player_2.nickname).unwrap();
    assert_eq!(p2_words.len(), 0);
    let p3_words = player_words.get(&game.player_3.nickname).unwrap();
    assert_eq!(p3_words.len(), 2);
    assert_eq!(p3_words.get(0).unwrap().word, "p3_w1");
    assert_eq!(p3_words.get(1).unwrap().word, "p3_w2");
    // Voting words
    let voting_words = game_state.last_round().player_voting_words;
    assert_eq!(voting_words.len(), 3);
    let p1_word = voting_words.get(&game.player_1.nickname).unwrap().clone();
    assert_eq!(p1_word, Some("p1_w1".to_string()));
    let p2_word = voting_words.get(&game.player_2.nickname).unwrap().clone();
    assert_eq!(p2_word, None);
    let p3_word = voting_words.get(&game.player_3.nickname).unwrap().clone();
    assert_eq!(p3_word, Some("p3_w2".to_string()));

    // Player 2 prespective after sending a word
    let game_state = game
        .player_2
        .send_voting_word(Some("p2_w1".to_string()))
        .await
        .unwrap();
    let _ = game.player_1.receive_game_sate().await.unwrap();
    let _ = game.player_3.receive_game_sate().await.unwrap();
    // Submitted words
    let player_words = game_state.last_round().player_words;
    assert_eq!(player_words.len(), 3);
    let p1_words = player_words.get(&game.player_1.nickname).unwrap();
    assert_eq!(p1_words.len(), 2);
    assert_eq!(p1_words.get(0).unwrap().word, "p1_w1");
    assert_eq!(p1_words.get(1).unwrap().word, "p1_w2");
    let p2_words = player_words.get(&game.player_2.nickname).unwrap();
    assert_eq!(p2_words.len(), 2);
    assert_eq!(p2_words.get(0).unwrap().word, "p2_w1");
    assert_eq!(p2_words.get(1).unwrap().word, "p2_w2");
    let p3_words = player_words.get(&game.player_3.nickname).unwrap();
    assert_eq!(p3_words.len(), 0);
    // Voting words
    let voting_words = game_state.last_round().player_voting_words;
    assert_eq!(voting_words.len(), 3);
    let p1_word = voting_words.get(&game.player_1.nickname).unwrap().clone();
    assert_eq!(p1_word, Some("p1_w1".to_string()));
    let p2_word = voting_words.get(&game.player_2.nickname).unwrap().clone();
    assert_eq!(p2_word, Some("p2_w1".to_string()));
    let p3_word = voting_words.get(&game.player_3.nickname).unwrap().clone();
    assert_eq!(p3_word, Some("p3_w2".to_string()));

    // Advance to next voting item
    // Player 1 prespective
    let game_state = game.player_1.accept_players_voting_words().await.unwrap();
    // Submitted words
    let words = game_state.last_round().player_words;
    assert_eq!(words.len(), 3);
    let p1_words = words.get(&game.player_1.nickname).unwrap();
    assert_eq!(p1_words.len(), 2);
    assert_eq!(p1_words.get(0).unwrap().word, "p1_w1");
    assert_eq!(p1_words.get(1).unwrap().word, "p1_w2");
    let p2_words = words.get(&game.player_2.nickname).unwrap();
    assert_eq!(p2_words.len(), 1);
    assert_eq!(p2_words.get(0).unwrap().word, "p2_w1");
    let p3_words = words.get(&game.player_3.nickname).unwrap();
    assert_eq!(p3_words.len(), 1);
    assert_eq!(p3_words.get(0).unwrap().word, "p3_w2");
    // Voting words
    let voting_words = game_state.last_round().player_voting_words;
    assert_eq!(voting_words.len(), 1);
    let p1_word = voting_words.get(&game.player_1.nickname).unwrap().clone();
    assert_eq!(p1_word, Some("p1_w2".to_string()));
    assert!(!voting_words.contains_key(&game.player_2.nickname));
    assert!(!voting_words.contains_key(&game.player_3.nickname));

    // Player 2 prespective
    let game_state = &game.player_2.receive_game_sate().await.unwrap();
    // Submitted words
    let words = game_state.last_round().player_words;
    assert_eq!(words.len(), 3);
    let p1_words = words.get(&game.player_1.nickname).unwrap();
    assert_eq!(p1_words.len(), 2);
    assert_eq!(p1_words.get(0).unwrap().word, "p1_w1");
    assert_eq!(p1_words.get(1).unwrap().word, "p1_w2");
    let p2_words = words.get(&game.player_2.nickname).unwrap();
    assert_eq!(p2_words.len(), 2);
    assert_eq!(p2_words.get(0).unwrap().word, "p2_w1");
    assert_eq!(p2_words.get(1).unwrap().word, "p2_w2");
    let p3_words = words.get(&game.player_3.nickname).unwrap();
    assert_eq!(p3_words.len(), 1);
    assert_eq!(p3_words.get(0).unwrap().word, "p3_w2");
    // Voting words
    let voting_words = game_state.last_round().player_voting_words;
    assert_eq!(voting_words.len(), 1);
    let p1_word = voting_words.get(&game.player_1.nickname).unwrap().clone();
    assert_eq!(p1_word, Some("p1_w2".to_string()));
    assert!(!voting_words.contains_key(&game.player_2.nickname));
    assert!(!voting_words.contains_key(&game.player_3.nickname));

    // Player 3 prespective
    let game_state = &game.player_3.receive_game_sate().await.unwrap();
    // Submitted words
    let words = game_state.last_round().player_words;
    assert_eq!(words.len(), 3);
    let p1_words = words.get(&game.player_1.nickname).unwrap();
    assert_eq!(p1_words.len(), 2);
    assert_eq!(p1_words.get(0).unwrap().word, "p1_w1");
    assert_eq!(p1_words.get(1).unwrap().word, "p1_w2");
    let p2_words = words.get(&game.player_2.nickname).unwrap();
    assert_eq!(p2_words.len(), 1);
    assert_eq!(p2_words.get(0).unwrap().word, "p2_w1");
    let p3_words = words.get(&game.player_3.nickname).unwrap();
    assert_eq!(p3_words.len(), 2);
    assert_eq!(p3_words.get(0).unwrap().word, "p3_w1");
    assert_eq!(p3_words.get(1).unwrap().word, "p3_w2");
    // Voting words
    let voting_words = game_state.last_round().player_voting_words;
    assert_eq!(voting_words.len(), 1);
    let p1_word = voting_words.get(&game.player_1.nickname).unwrap().clone();
    assert_eq!(p1_word, Some("p1_w2".to_string()));
    assert!(!voting_words.contains_key(&game.player_2.nickname));
    assert!(!voting_words.contains_key(&game.player_3.nickname));
}

#[tokio::test]
async fn players_can_complete_a_round() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let mut game: GameTest = create_game(
        &app.base_address,
        client,
        GameFsmState::PlayersSubmittingWords,
    )
    .await;

    game.complete_round().await;
}

#[tokio::test]
async fn players_can_complete_a_game() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let mut game: GameTest = create_game(
        &app.base_address,
        client,
        GameFsmState::PlayersSubmittingWords,
    )
    .await;

    game.complete_round().await;
    let game_state = game.continue_to_next_round().await;
    assert_eq!(game_state.state, "PlayersSubmittingWords");

    game.complete_round().await;
    let game_state = game.continue_to_next_round().await;
    assert_eq!(game_state.state, "PlayersSubmittingWords");

    game.complete_round().await;
    let game_state = game.continue_to_next_round().await;
    assert_eq!(game_state.state, "EndOfGame");
}

async fn create_game(base_address: &str, client: reqwest::Client, state: GameFsmState) -> GameTest {
    let game_id = create_game_without_players(base_address, client).await;

    let mut player_1 = add_player(base_address, &game_id, "p1").await.unwrap();
    let game_state = player_1.receive_game_sate().await.unwrap();
    assert_eq!(game_state.state, "Lobby");
    assert_eq!(game_state.players.len(), 1);
    assert_eq!(
        game_state.players.get(0).unwrap().nickname,
        player_1.nickname
    );
    assert!(game_state.players.get(0).unwrap().is_host);

    // Make sure to read the events the other players receive when new players join, so that we leave a "clean" response channel for the tests
    let mut player_2 = add_player(base_address, &game_id, "p2").await.unwrap();
    let _ = player_1.receive_game_sate().await.unwrap();
    let game_state = player_2.receive_game_sate().await.unwrap();
    assert_eq!(game_state.state, "Lobby");
    assert_eq!(game_state.players.len(), 2);
    assert_eq!(
        game_state.players.get(0).unwrap().nickname,
        player_1.nickname
    );
    assert_eq!(
        game_state.players.get(1).unwrap().nickname,
        player_2.nickname
    );
    assert!(!game_state.players.get(1).unwrap().is_host);

    let mut player_3 = add_player(base_address, &game_id, "p3").await.unwrap();
    let _ = player_1.receive_game_sate().await.unwrap();
    let _ = player_2.receive_game_sate().await.unwrap();
    let game_state = player_3.receive_game_sate().await.unwrap();
    assert_eq!(game_state.state, "Lobby");
    assert_eq!(game_state.players.len(), 3);
    assert_eq!(
        game_state.players.get(0).unwrap().nickname,
        player_1.nickname
    );
    assert_eq!(
        game_state.players.get(1).unwrap().nickname,
        player_2.nickname
    );
    assert_eq!(
        game_state.players.get(2).unwrap().nickname,
        player_3.nickname
    );
    assert!(!game_state.players.get(2).unwrap().is_host);

    match state {
        GameFsmState::Lobby => {}
        GameFsmState::PlayersSubmittingWords => {
            let game_state = player_1.start_game().await.unwrap();
            let _ = player_2.receive_game_sate().await.unwrap();
            let _ = player_3.receive_game_sate().await.unwrap();
            assert_eq!(game_state.state, "PlayersSubmittingWords")
        }
    }

    GameTest {
        id: game_id.clone(),
        player_1,
        player_2,
        player_3,
    }
}

async fn create_game_without_players(base_address: &str, client: reqwest::Client) -> String {
    let response = client
        .post(format!("http://{base_address}/game"))
        .send()
        .await
        .expect("Failed to execute CreateGame request.");
    assert!(response.status().is_success());

    let game_created_response: GameCreatedResponse = response
        .json()
        .await
        .expect("Failed to parse GameCreatedResponse.");
    assert!(!game_created_response.id.is_empty());

    game_created_response.id
}

async fn add_player(
    base_address: &str,
    game_id: &str,
    nickname: &str,
) -> Result<PlayerTest, String> {
    let (tx, rx) = open_game_websocket(base_address, game_id, nickname)
        .await?
        .split();
    Ok(PlayerTest {
        nickname: nickname.to_string(),
        words: vec![format!("{nickname}_w1"), format!("{nickname}_w2")],
        tx,
        rx,
    })
}

async fn open_game_websocket(
    base_address: &str,
    game_id: &str,
    nickname: &str,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, String> {
    tokio_tungstenite::connect_async(format!(
        "ws://{base_address}/game/{game_id}/player/{nickname}/ws"
    ))
    .await
    .map(|websocket_stream| websocket_stream.0)
    .map_err(|error| format!("WebSocket could not be created. Error: '{error}'."))
}

async fn sleep(duration: Duration) {
    let mut timer = time::interval(duration);
    timer.tick().await;
    timer.tick().await;
}

enum GameFsmState {
    Lobby,
    PlayersSubmittingWords,
}

struct GameTest {
    id: String,
    player_1: PlayerTest,
    player_2: PlayerTest,
    player_3: PlayerTest,
}

impl GameTest {
    pub async fn players_send_words(&mut self) -> GameState {
        let _ = self
            .player_1
            .send_words(self.player_1.words.clone())
            .await
            .unwrap();
        let _ = self.player_2.receive_game_sate().await.unwrap();
        let _ = self.player_3.receive_game_sate().await.unwrap();

        let _ = self
            .player_2
            .send_words(self.player_2.words.clone())
            .await
            .unwrap();
        let _ = self.player_1.receive_game_sate().await.unwrap();
        let _ = self.player_3.receive_game_sate().await.unwrap();

        let _ = self
            .player_3
            .send_words(self.player_3.words.clone())
            .await
            .unwrap();
        let _ = self.player_2.receive_game_sate().await.unwrap();
        self.player_1.receive_game_sate().await.unwrap()
    }

    pub async fn complete_round(&mut self) {
        let game_state = self.players_send_words().await;
        assert_eq!(game_state.state, "PlayersSubmittingVotingWord");

        // Voting for p1_w1
        // p1: [used, unused], p2: [used, unused], p3: [unused, unused]
        let _ = self
            .player_2
            .send_voting_word(self.player_2.words.get(0).cloned())
            .await
            .unwrap();
        let _ = self.player_1.receive_game_sate().await.unwrap();
        let _ = self.player_3.receive_game_sate().await.unwrap();

        let _ = self.player_1.accept_players_voting_words().await.unwrap();
        let _ = self.player_2.receive_game_sate().await.unwrap();
        let _ = self.player_3.receive_game_sate().await.unwrap();

        // Voting for p1_w2
        // p1: [used, used], p2: [used, unused], p3: [unused, unused]
        let _ = self.player_2.send_voting_word(None).await.unwrap();
        let _ = self.player_1.receive_game_sate().await.unwrap();
        let _ = self.player_3.receive_game_sate().await.unwrap();

        let _ = self
            .player_3
            .send_voting_word(self.player_3.words.get(1).cloned())
            .await
            .unwrap();
        let _ = self.player_1.receive_game_sate().await.unwrap();
        let _ = self.player_2.receive_game_sate().await.unwrap();

        let _ = self.player_1.accept_players_voting_words().await.unwrap();
        let _ = self.player_2.receive_game_sate().await.unwrap();
        let _ = self.player_3.receive_game_sate().await.unwrap();

        // Voting for p2_w2
        // p1: [used, used], p2: [used, used], p3: [unused, used]
        let _ = self.player_1.accept_players_voting_words().await.unwrap();
        let _ = self.player_2.receive_game_sate().await.unwrap();
        let _ = self.player_3.receive_game_sate().await.unwrap();

        // Voting for p3_w1
        // p1: [used, used], p2: [used, used], p3: [used, used]
        let game_state: GameState = self.player_1.accept_players_voting_words().await.unwrap();
        let _ = self.player_2.receive_game_sate().await.unwrap();
        let _ = self.player_3.receive_game_sate().await.unwrap();

        assert_eq!(game_state.state, "EndOfRound");
    }

    pub async fn continue_to_next_round(&mut self) -> GameState {
        let game_state = self.player_1.continue_to_next_round().await.unwrap();
        let _ = self.player_2.receive_game_sate().await.unwrap();
        let _ = self.player_3.receive_game_sate().await.unwrap();
        game_state
    }
}

struct PlayerTest {
    nickname: String,
    words: Vec<String>,
    tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    rx: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl PlayerTest {
    pub async fn receive_game_sate(&mut self) -> Result<GameState, String> {
        match self.rx.next().await {
            Some(Ok(message)) => {
                match serde_json::from_str(message.to_text().expect("Message was not a text")) {
                    Ok(WsMessageOut::GameState {
                        state,
                        players,
                        rounds,
                    }) => Ok(GameState {
                        state,
                        players,
                        rounds,
                    }),
                    Ok(WsMessageOut::Error {
                        r#type,
                        title,
                        detail,
                    }) => {
                        assert!(!title.is_empty());
                        assert!(!detail.is_empty());
                        Err(r#type)
                    }
                    Err(error) => Err(format!("Could not parse the message. Error: '{error}'.")),
                }
            }
            Some(Err(error)) => Err(format!("Websocket returned an error {error}")),
            None => Err("Websocket closed before expected.".to_string()),
        }
    }

    pub async fn start_game(&mut self) -> Result<GameState, String> {
        self.send_text_message(WsMessageIn::StartGame {
            amount_of_rounds: 3,
        })
        .await;
        self.receive_game_sate().await
    }

    pub async fn send_words(&mut self, words: Vec<String>) -> Result<GameState, String> {
        self.send_text_message(WsMessageIn::PlayerWords { words })
            .await;
        self.receive_game_sate().await
    }

    pub async fn send_voting_word(&mut self, word: Option<String>) -> Result<GameState, String> {
        self.send_text_message(WsMessageIn::PlayerVotingWord { word })
            .await;
        self.receive_game_sate().await
    }

    pub async fn accept_players_voting_words(&mut self) -> Result<GameState, String> {
        self.send_text_message(WsMessageIn::AcceptPlayersVotingWords)
            .await;
        self.receive_game_sate().await
    }

    pub async fn continue_to_next_round(&mut self) -> Result<GameState, String> {
        self.send_text_message(WsMessageIn::ContinueToNextRound)
            .await;
        self.receive_game_sate().await
    }

    pub async fn send_raw_message(&mut self, message: Message) -> Result<GameState, String> {
        self.send_message(message).await;
        self.receive_game_sate().await
    }

    async fn send_text_message(&mut self, message: WsMessageIn) {
        self.send_message(Message::Text(
            serde_json::to_string(&message).expect("Could not serialize message"),
        ))
        .await;
    }

    pub async fn send_message(&mut self, message: Message) {
        self.tx.send(message).await.expect("Could not send message");
    }
}

#[derive(Deserialize, Debug)]
struct GameState {
    state: String,
    players: Vec<Player>,
    rounds: Vec<Round>,
}

impl GameState {
    pub fn last_round(&self) -> Round {
        self.rounds.last().unwrap().clone()
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Player {
    nickname: String,
    is_host: bool,
    is_connected: bool,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct Round {
    word: String,
    player_words: HashMap<String, Vec<Word>>,
    player_voting_words: HashMap<String, Option<String>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct Word {
    word: String,
}

#[derive(Deserialize)]
struct GameCreatedResponse {
    id: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", tag = "kind")]
enum WsMessageOut {
    Error {
        r#type: String,
        title: String,
        detail: String,
    },
    GameState {
        state: String,
        players: Vec<Player>,
        rounds: Vec<Round>,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
enum WsMessageIn {
    #[serde(rename_all = "camelCase")]
    StartGame {
        amount_of_rounds: u8,
    },
    #[serde(rename_all = "camelCase")]
    PlayerWords {
        words: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    PlayerVotingWord {
        word: Option<String>,
    },
    AcceptPlayersVotingWords,
    ContinueToNextRound,
}
