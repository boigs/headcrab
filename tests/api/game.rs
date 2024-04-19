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

    create_game(&app.base_address, client).await;
}

#[tokio::test]
async fn two_different_players_can_be_added_to_game() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let game_id = create_game(&app.base_address, client).await.id;

    let nickname1 = "player1";
    let (_, mut rx1) = open_game_websocket(&app.base_address, &game_id, nickname1)
        .await
        .split();
    let game_state: GameState = receive_game_sate(&mut rx1).await;
    assert_eq!(game_state.players.len(), 1);
    assert_eq!(game_state.players.first().unwrap().nickname, nickname1);
    assert!(game_state.players.first().unwrap().is_host);

    let nickname2 = "player2";
    let (_, mut rx2) = open_game_websocket(&app.base_address, &game_id, nickname2)
        .await
        .split();
    let game_state: GameState = receive_game_sate(&mut rx2).await;
    assert_eq!(game_state.players.len(), 2);
    assert!(game_state
        .players
        .iter()
        .any(|player| player.nickname == nickname1));
    assert!(game_state
        .players
        .iter()
        .any(|player| player.nickname == nickname2));
    assert!(!game_state.players.last().unwrap().is_host);
}

#[tokio::test]
async fn when_player_already_exists_add_player_with_same_nickname_to_game_fails() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let game_id = create_game(&app.base_address, client).await.id;

    let nickname = "player";
    let (_, mut rx) = open_game_websocket(&app.base_address, &game_id, nickname)
        .await
        .split();
    let game_state: GameState = receive_game_sate(&mut rx).await;
    assert_eq!(game_state.players.len(), 1);
    assert_eq!(game_state.players.first().unwrap().nickname, nickname);

    let (_, mut rx) = open_game_websocket(&app.base_address, &game_id, nickname)
        .await
        .split();

    assert!(receive_error(&mut rx).await.eq("PLAYER_ALREADY_EXISTS"));
}

#[tokio::test]
async fn game_can_be_started() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let game_id = create_game(&app.base_address, client).await.id;

    let nickname = "p1";
    let (mut tx, mut rx) = open_game_websocket(&app.base_address, &game_id, nickname)
        .await
        .split();
    assert_eq!(receive_game_sate(&mut rx).await.state, "Lobby");
    let _player2_connection = open_game_websocket(&app.base_address, &game_id, "p2").await;
    let _ = receive_game_sate(&mut rx).await;
    let _player3_connection = open_game_websocket(&app.base_address, &game_id, "p3").await;
    let _ = receive_game_sate(&mut rx).await;

    send_start_game(
        &mut tx,
        WsMessageIn::StartGame {
            amount_of_rounds: 5,
        },
    )
    .await;

    let game_state = receive_game_sate(&mut rx).await;
    assert_eq!(game_state.state, "PlayersWritingWords");
    assert_eq!(game_state.rounds.len(), 1);
    assert!(!game_state.rounds.first().unwrap().word.is_empty());
}

#[tokio::test]
async fn non_host_player_cannot_start_game() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let game_id = create_game(&app.base_address, client).await.id;

    let nickname = "p1";
    let _player1_connection = open_game_websocket(&app.base_address, &game_id, nickname).await;
    let (mut tx, mut rx) = open_game_websocket(&app.base_address, &game_id, "p2")
        .await
        .split();
    assert_eq!(receive_game_sate(&mut rx).await.state, "Lobby");
    let _player3_connection = open_game_websocket(&app.base_address, &game_id, "p3").await;
    let _ = receive_game_sate(&mut rx).await;

    send_start_game(
        &mut tx,
        WsMessageIn::StartGame {
            amount_of_rounds: 5,
        },
    )
    .await;

    let error = receive_error(&mut rx).await;
    assert_eq!(&error, "COMMAND_NOT_ALLOWED");
}

#[tokio::test]
async fn game_cannot_be_started_with_less_than_three_players() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let game_id = create_game(&app.base_address, client).await.id;

    let nickname = "p1";
    let (mut tx, mut rx) = open_game_websocket(&app.base_address, &game_id, nickname)
        .await
        .split();
    assert_eq!(receive_game_sate(&mut rx).await.state, "Lobby");
    let _player2_connection = open_game_websocket(&app.base_address, &game_id, "p2").await;
    let _ = receive_game_sate(&mut rx).await;

    send_start_game(
        &mut tx,
        WsMessageIn::StartGame {
            amount_of_rounds: 5,
        },
    )
    .await;

    let error = receive_error(&mut rx).await;
    assert_eq!(&error, "NOT_ENOUGH_PLAYERS");
}

#[tokio::test]
async fn game_is_still_alive_when_all_players_leave() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let game_id = create_game(&app.base_address, client).await.id;
    let nickname = "player1";
    let (tx, rx) = open_game_websocket(&app.base_address, &game_id, nickname)
        .await
        .split();

    // Drop the references to the websocket so that it gets closed and the server disconnects the player from the game
    drop(tx);
    drop(rx);

    let nickname = "player2";
    let (_, mut rx) = open_game_websocket(&app.base_address, &game_id, nickname)
        .await
        .split();

    let game_state = receive_game_sate(&mut rx).await;
    assert!(game_state.state.eq("Lobby"));
    assert_eq!(game_state.players.len(), 2);
    assert_eq!(game_state.players.get(0).unwrap().is_connected, false);
    assert_eq!(game_state.players.get(1).unwrap().is_connected, true);
}

#[tokio::test]
async fn game_is_closed_after_inactivity_timeout() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let game_id = create_game(&app.base_address, client).await.id;
    let nickname = "player1";
    let (tx, _) = open_game_websocket(&app.base_address, &game_id, nickname)
        .await
        .split();

    // Drop the references to the websocket so that it gets closed and the server disconnects the player from the game
    drop(tx);

    // Wait until the game is closed
    sleep(app.inactivity_timeout + Duration::from_secs(1)).await;

    // Try to connect to the same game again
    let nickname = "player2";
    let (_, mut rx) = open_game_websocket(&app.base_address, &game_id, nickname)
        .await
        .split();

    let error = receive_error(&mut rx).await;
    assert_eq!(&error, "GAME_DOES_NOT_EXIST");
}

#[tokio::test]
async fn unknown_websocket_text_message_is_rejected_but_game_still_alive() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let game_id = create_game(&app.base_address, client).await.id;

    let nickname = "player";
    let (mut tx, mut rx) = open_game_websocket(&app.base_address, &game_id, nickname)
        .await
        .split();
    assert_eq!(receive_game_sate(&mut rx).await.state, "Lobby");

    send_message(&mut tx, Message::Text("invalid".to_string())).await;
    let error = receive_error(&mut rx).await;
    assert_eq!(&error, "UNPROCESSABLE_WEBSOCKET_MESSAGE");

    let nickname = "player2";
    let (mut _tx, mut rx) = open_game_websocket(&app.base_address, &game_id, nickname)
        .await
        .split();
    receive_game_sate(&mut rx).await;
}

#[tokio::test]
async fn when_sending_invalid_message_game_it_is_reject_but_game_is_still_alive() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let game_id = create_game(&app.base_address, client).await.id;

    let nickname = "player";
    let (mut tx, mut rx) = open_game_websocket(&app.base_address, &game_id, nickname)
        .await
        .split();
    assert_eq!(receive_game_sate(&mut rx).await.state, "Lobby");

    send_message(&mut tx, Message::Binary(vec![])).await;
    let error = receive_error(&mut rx).await;
    assert_eq!(&error, "UNPROCESSABLE_WEBSOCKET_MESSAGE");

    let nickname = "player2";
    let (mut _tx, mut rx) = open_game_websocket(&app.base_address, &game_id, nickname)
        .await
        .split();
    receive_game_sate(&mut rx).await;
}

#[tokio::test]
async fn when_attempting_to_start_game_with_one_player_then_websocket_is_not_closed() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let game_id = create_game(&app.base_address, client).await.id;

    let nickname = "p1";
    let (mut tx, mut rx) = open_game_websocket(&app.base_address, &game_id, nickname)
        .await
        .split();
    assert_eq!(receive_game_sate(&mut rx).await.state, "Lobby");
    let _player2_connection = open_game_websocket(&app.base_address, &game_id, "p2").await;
    let _ = receive_game_sate(&mut rx).await;

    send_start_game(
        &mut tx,
        WsMessageIn::StartGame {
            amount_of_rounds: 5,
        },
    )
    .await;

    let error = receive_error(&mut rx).await;
    assert_eq!(&error, "NOT_ENOUGH_PLAYERS");

    assert_eq!(receive_game_sate(&mut rx).await.state, "Lobby");
}

async fn create_game(base_address: &str, client: reqwest::Client) -> GameCreatedResponse {
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

    game_created_response
}

async fn open_game_websocket(
    base_address: &str,
    game_id: &str,
    nickname: &str,
) -> WebSocketStream<MaybeTlsStream<TcpStream>> {
    tokio_tungstenite::connect_async(format!(
        "ws://{base_address}/game/{game_id}/player/{nickname}/ws"
    ))
    .await
    .expect("WebSocket could not be created.")
    .0
}

async fn send_start_game(
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
) -> GameState {
    match receiver.next().await {
        Some(Ok(message)) => {
            match serde_json::from_str(message.to_text().expect("Message was not a text")) {
                Ok(WsMessageOut::GameState {
                    state,
                    players,
                    rounds,
                }) => GameState {
                    state,
                    players,
                    rounds,
                },
                _ => panic!("The message was not a WsMessage::GameState"),
            }
        }
        Some(Err(error)) => panic!("Websocket returned an error {}", error),
        _ => panic!("Websocket closed before expected."),
    }
}

async fn receive_error(
    receiver: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
) -> String {
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
                    r#type
                }
                _ => panic!("The message was not a WsMessage::Error"),
            }
        }
        Some(Err(error)) => panic!("Websocket returned an error {}", error),
        _ => panic!("Websocket closed before expected."),
    }
}

async fn sleep(duration: Duration) {
    let mut timer = time::interval(duration);
    timer.tick().await;
    timer.tick().await;
}

#[derive(Deserialize)]
struct GameState {
    state: String,
    players: Vec<Player>,
    rounds: Vec<Round>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Player {
    nickname: String,
    is_host: bool,
    is_connected: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Round {
    word: String,
}

#[derive(Deserialize)]
struct GameCreatedResponse {
    id: String,
}

#[derive(Deserialize)]
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
}
