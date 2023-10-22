use crate::helpers::spawn_app;
use futures_util::stream::{SplitStream, StreamExt};
use serde::Deserialize;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

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

    assert!(receive_error(&mut rx)
        .await
        .contains("Player already exists"));
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

// It's important for the receiver to be a reference, otherwise this method takes ownership of it and when it ends it closes the websocket
async fn receive_game_sate(
    receiver: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
) -> GameState {
    match receiver.next().await {
        Some(Ok(message)) => {
            match serde_json::from_str(message.to_text().expect("Message was not a text")) {
                Ok(WsMessage::GameState { players }) => GameState { players },
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
                Ok(WsMessage::Error { message }) => message,
                _ => panic!("The message was not a WsMessage::Error"),
            }
        }
        Some(Err(error)) => panic!("Websocket returned an error {}", error),
        _ => panic!("Websocket closed before expected."),
    }
}

#[derive(Deserialize)]
struct GameState {
    players: Vec<Player>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Player {
    nickname: String,
}

#[derive(Deserialize)]
struct GameCreatedResponse {
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
enum WsMessage {
    Error { message: String },
    GameState { players: Vec<Player> },
}
