use crate::helpers::spawn_app;
use futures_util::{
    stream::{SplitSink, SplitStream, StreamExt},
    SinkExt,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

#[tokio::test]
async fn create_game_works() {
    let base_address = spawn_app();
    let client = reqwest::Client::new();

    create_game(&base_address, client).await;
}

#[tokio::test]
async fn two_different_players_can_be_added_to_game() {
    let base_address = spawn_app();
    let client = reqwest::Client::new();

    let game_id = create_game(&base_address, client).await.id;

    let nickname1 = "player1";
    let (_, mut rx1) = open_game_websocket(&base_address, &game_id, nickname1)
        .await
        .split();
    let game_state: GameState = receive_game_sate(&mut rx1).await;
    assert_eq!(game_state.players.len(), 1);
    assert_eq!(game_state.players.first().unwrap().nickname, nickname1);
    assert!(game_state.players.first().unwrap().is_host);

    let nickname2 = "player2";
    let (_, mut rx2) = open_game_websocket(&base_address, &game_id, nickname2)
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
async fn add_player_to_game_fails_when_player_already_exists() {
    let base_address = spawn_app();
    let client = reqwest::Client::new();

    let game_id = create_game(&base_address, client).await.id;

    let nickname = "player";
    let (_, mut rx) = open_game_websocket(&base_address, &game_id, nickname)
        .await
        .split();
    let game_state: GameState = receive_game_sate(&mut rx).await;
    assert_eq!(game_state.players.len(), 1);
    assert_eq!(game_state.players.first().unwrap().nickname, nickname);

    let (_, mut rx) = open_game_websocket(&base_address, &game_id, nickname)
        .await
        .split();

    assert!(receive_error(&mut rx).await.eq("PLAYER_ALREADY_EXISTS"));
}

#[tokio::test]
async fn game_can_be_started() {
    let base_address = spawn_app();
    let client = reqwest::Client::new();

    let game_id = create_game(&base_address, client).await.id;

    let nickname = "player";
    let (mut tx, mut rx) = open_game_websocket(&base_address, &game_id, nickname)
        .await
        .split();
    assert_eq!(receive_game_sate(&mut rx).await.state, "Lobby");

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
async fn game_is_removed_when_all_players_leave() {
    let base_address = spawn_app();
    let client = reqwest::Client::new();

    let game_id = create_game(&base_address, client).await.id;
    let nickname = "player1";
    let (tx, rx) = open_game_websocket(&base_address, &game_id, nickname)
        .await
        .split();
    // Drop the references to the websocket so that it gets closed and the server removes the player from the game
    drop(tx);
    drop(rx);

    let nickname = "player2";
    let (_, mut rx) = open_game_websocket(&base_address, &game_id, nickname)
        .await
        .split();
    assert!(receive_error(&mut rx).await.eq("GAME_DOES_NOT_EXIST"));
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
    websocket
        .send(Message::Text(
            serde_json::to_string(&message).expect("Could not serialize message"),
        ))
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
                Ok(WsMessageOut::GameState { state, players , rounds }) => GameState { state, players, rounds },
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
