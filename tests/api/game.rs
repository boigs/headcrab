use crate::helpers::spawn_app;
use serde::Deserialize;

#[tokio::test]
async fn create_game_works() {
    let base_address = spawn_app();
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{base_address}/game"))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    let game_created: GameCreatedResponse =
        response.json().await.expect("Failed to parse response.");
    assert!(!game_created.id.is_empty());
}

#[derive(Deserialize)]
struct GameCreatedResponse {
    id: String,
}
