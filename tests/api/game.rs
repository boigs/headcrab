use crate::helpers::spawn_app;
use futures_util::stream::StreamExt;
use serde::Deserialize;

#[tokio::test]
async fn create_game_works() {
    let base_address = spawn_app();
    let client = reqwest::Client::new();

    let response = client
        .post(format!("http://{base_address}/game"))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    let game_created: GameCreatedResponse =
        response.json().await.expect("Failed to parse response.");
    assert!(!game_created.id.is_empty());
}

#[tokio::test]
async fn player_is_added_to_the_game_with_websocket() {
    let base_address = spawn_app();
    let client = reqwest::Client::new();
    let nickname = "dani";

    let game_id = client
        .post(format!("http://{base_address}/game"))
        .send()
        .await
        .unwrap()
        .json::<GameCreatedResponse>()
        .await
        .unwrap()
        .id;

    let (websocket, _) = tokio_tungstenite::connect_async(format!(
        "ws://{base_address}/game/{game_id}/player/{nickname}/ws"
    ))
    .await
    .expect("Failed to connect to the websocket.");

    let (_, mut rx) = websocket.split();
    let response = match rx.next().await {
        Some(Ok(message)) => message.into_text().expect("Message was not a text"),
        Some(Err(error)) => panic!("Websocket returned an error {error}"),
        _ => panic!("Websocket closed before expected."),
    };
    let game_state: GameState =
        serde_json::from_str(&response).expect("Could not deserialize the GameState response.");
    assert_eq!(game_state.players.len(), 1);
    assert_eq!(game_state.players.first().unwrap().nickname, nickname);
}

#[derive(Deserialize)]
struct GameState {
    players: Vec<Player>,
}

#[derive(Deserialize)]
struct Player {
    nickname: String,
}

#[derive(Deserialize)]
struct GameCreatedResponse {
    id: String,
}
