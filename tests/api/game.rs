use std::time::Duration;

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
    let error = receive_error(&mut player.rx).await.unwrap();
    assert_eq!(error, "PLAYER_ALREADY_EXISTS");
}

#[tokio::test]
async fn host_player_can_start_game() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let mut game = create_game(&app.base_address, client, GameFsmState::Lobby).await;

    send_text_message(
        &mut game.player_1.tx,
        WsMessageIn::StartGame {
            amount_of_rounds: 5,
        },
    )
    .await;

    let game_state = receive_game_sate(&mut game.player_1.rx).await.unwrap();
    assert_eq!(game_state.state, "PlayersWritingWords");
    assert_eq!(game_state.rounds.len(), 1);
    assert!(!game_state.rounds.first().unwrap().word.is_empty());
}

#[tokio::test]
async fn non_host_player_cannot_start_game() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let mut game = create_game(&app.base_address, client, GameFsmState::Lobby).await;

    send_text_message(
        &mut game.player_2.tx,
        WsMessageIn::StartGame {
            amount_of_rounds: 5,
        },
    )
    .await;

    let error = receive_error(&mut game.player_2.rx).await.unwrap();
    assert_eq!(&error, "COMMAND_NOT_ALLOWED");
}

#[tokio::test]
async fn game_cannot_be_started_with_less_than_three_players() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let game_id = create_game_without_players(&app.base_address, client).await;

    let mut player_1 = add_player(&app.base_address, &game_id, "p1").await.unwrap();
    let _ = receive_game_sate(&mut player_1.rx).await.unwrap();
    let _player_2 = add_player(&app.base_address, &game_id, "p2").await.unwrap();
    // Skip the second GameState after player 2 joins
    let _ = receive_game_sate(&mut player_1.rx).await.unwrap();

    send_text_message(
        &mut player_1.tx,
        WsMessageIn::StartGame {
            amount_of_rounds: 5,
        },
    )
    .await;
    let error = receive_error(&mut player_1.rx).await.unwrap();
    assert_eq!(&error, "NOT_ENOUGH_PLAYERS");

    // The game is still alive and the socket of player 1 is still open
    let game_state = receive_game_sate(&mut player_1.rx).await.unwrap();
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

    let game_state = receive_game_sate(&mut player.rx).await.unwrap();
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
    let error = receive_error(&mut player.rx).await.unwrap();
    assert_eq!(&error, "GAME_DOES_NOT_EXIST");
}

#[tokio::test]
async fn unknown_websocket_text_message_is_rejected_but_game_still_alive() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let mut game = create_game(&app.base_address, client, GameFsmState::Lobby).await;

    send_message(&mut game.player_1.tx, Message::Text("invalid".to_string())).await;
    let error = receive_error(&mut game.player_1.rx).await.unwrap();
    assert_eq!(&error, "UNPROCESSABLE_WEBSOCKET_MESSAGE");

    let mut player = add_player(&app.base_address, &game.id, "p4").await.unwrap();
    receive_game_sate(&mut player.rx).await.unwrap();
}

#[tokio::test]
async fn when_sending_invalid_message_game_it_is_reject_but_game_is_still_alive() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let mut game: GameTest = create_game(&app.base_address, client, GameFsmState::Lobby).await;

    send_message(&mut game.player_1.tx, Message::Binary(vec![])).await;
    let error = receive_error(&mut game.player_1.rx).await.unwrap();
    assert_eq!(&error, "UNPROCESSABLE_WEBSOCKET_MESSAGE");

    let mut player = add_player(&app.base_address, &game.id, "p4").await.unwrap();
    receive_game_sate(&mut player.rx).await.unwrap();
}

#[tokio::test]
async fn repeated_words_are_not_allowed() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let mut game: GameTest =
        create_game(&app.base_address, client, GameFsmState::PlayersWritingWords).await;

    send_text_message(
        &mut game.player_1.tx,
        WsMessageIn::PlayerWords {
            words: vec!["w1".to_string(), "w1".to_string()],
        },
    )
    .await;
    let error = receive_error(&mut game.player_1.rx).await.unwrap();
    assert_eq!(&error, "REPEATED_WORDS");
}

#[tokio::test]
async fn game_goes_to_players_sending_word_submission_when_all_players_send_words() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let mut game: GameTest =
        create_game(&app.base_address, client, GameFsmState::PlayersWritingWords).await;

    send_text_message(
        &mut game.player_1.tx,
        WsMessageIn::PlayerWords {
            words: vec!["w1".to_string()],
        },
    )
    .await;
    let game_state = receive_game_sate(&mut game.player_1.rx).await.unwrap();
    assert_eq!(game_state.state, "PlayersWritingWords");

    send_text_message(
        &mut game.player_2.tx,
        WsMessageIn::PlayerWords {
            words: vec!["w1".to_string()],
        },
    )
    .await;
    let game_state = receive_game_sate(&mut game.player_1.rx).await.unwrap();
    assert_eq!(game_state.state, "PlayersWritingWords");
    send_text_message(
        &mut game.player_3.tx,
        WsMessageIn::PlayerWords {
            words: vec!["w1".to_string()],
        },
    )
    .await;
    let game_state = receive_game_sate(&mut game.player_1.rx).await.unwrap();
    assert_eq!(game_state.state, "PlayersSendingWordSubmission");
    assert_eq!(game_state.rounds.len(), 1);
}

async fn create_game(base_address: &str, client: reqwest::Client, state: GameFsmState) -> GameTest {
    let game_id = create_game_without_players(base_address, client).await;

    let mut player_1 = add_player(base_address, &game_id, "p1").await.unwrap();
    let game_state = receive_game_sate(&mut player_1.rx).await.unwrap();
    assert_eq!(game_state.state, "Lobby");
    assert_eq!(game_state.players.len(), 1);
    assert_eq!(
        game_state.players.get(0).unwrap().nickname,
        player_1.nickname
    );
    assert!(game_state.players.get(0).unwrap().is_host);

    // Make sure to read the events the other players receive when new players join, so that we leave a "clean" response channel for the tests
    let mut player_2 = add_player(base_address, &game_id, "p2").await.unwrap();
    let _ = receive_game_sate(&mut player_1.rx).await.unwrap();
    let game_state = receive_game_sate(&mut player_2.rx).await.unwrap();
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
    let _ = receive_game_sate(&mut player_1.rx).await.unwrap();
    let _ = receive_game_sate(&mut player_2.rx).await.unwrap();
    let game_state = receive_game_sate(&mut player_3.rx).await.unwrap();
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
        GameFsmState::PlayersWritingWords => {
            send_text_message(
                &mut player_1.tx,
                WsMessageIn::StartGame {
                    amount_of_rounds: 5,
                },
            )
            .await;
            let game_state = receive_game_sate(&mut player_1.rx).await.unwrap();
            let _ = receive_game_sate(&mut player_2.rx).await.unwrap();
            let _ = receive_game_sate(&mut player_3.rx).await.unwrap();
            assert_eq!(game_state.state, "PlayersWritingWords")
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

async fn send_text_message(
    websocket: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    message: WsMessageIn,
) {
    send_message(
        websocket,
        Message::Text(serde_json::to_string(&message).expect("Could not serialize message")),
    )
    .await;
}

async fn send_message(
    websocket: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    message: Message,
) {
    websocket
        .send(message)
        .await
        .expect("Could not send message");
}

// It's important for the receiver to be a reference, otherwise this method takes ownership of it and when it ends it closes the websocket
async fn receive_game_sate(
    receiver: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
) -> Result<GameState, String> {
    match receiver.next().await {
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
                Ok(unexpected_message) => {
                    Err(format!("The message was not a WsMessage::GameState. Message: '{unexpected_message:?}'."))
                }
                Err(error) => Err(format!("Could not parse the message. Error: '{error}'.")),
            }
        }
        Some(Err(error)) => Err(format!("Websocket returned an error {error}")),
        None => Err("Websocket closed before expected.".to_string()),
    }
}

async fn receive_error(
    receiver: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
) -> Result<String, String> {
    match receiver.next().await {
        Some(Ok(message)) => {
            match serde_json::from_str(message.to_text().expect("Message was not a text")) {
                Ok(WsMessageOut::Error {
                    r#type,
                    title,
                    detail,
                }) => {
                    assert!(!title.is_empty());
                    assert!(!detail.is_empty());
                    Ok(r#type)
                }
                Ok(unexpected_message) => Err(format!(
                    "The message was not a WsMessage::Error. Message: '{unexpected_message:?}'."
                )),
                Err(error) => Err(format!("Could not parse the message. Error: '{error}'.")),
            }
        }
        Some(Err(error)) => Err(format!("Websocket returned an error {error}")),
        None => Err("Websocket closed before expected.".to_string()),
    }
}

async fn sleep(duration: Duration) {
    let mut timer = time::interval(duration);
    timer.tick().await;
    timer.tick().await;
}

enum GameFsmState {
    Lobby,
    PlayersWritingWords,
}

struct GameTest {
    id: String,
    player_1: PlayerTest,
    player_2: PlayerTest,
    player_3: PlayerTest,
}

struct PlayerTest {
    nickname: String,
    tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    rx: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

#[derive(Deserialize)]
struct GameState {
    state: String,
    players: Vec<Player>,
    rounds: Vec<Round>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Player {
    nickname: String,
    is_host: bool,
    is_connected: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Round {
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
    StartGame { amount_of_rounds: u8 },
    #[serde(rename_all = "camelCase")]
    PlayerWords { words: Vec<String> },
}
