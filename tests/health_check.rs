use serde::Deserialize;
use std::net::{SocketAddr, TcpListener};

#[tokio::test]
async fn health_check_works() {
    let base_address = spawn_app();
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{base_address}/health_check"))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[derive(Deserialize)]
struct GameCreatedResponse {
    id: String,
}

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

fn spawn_app() -> String {
    // Binding to port 0 triggers an OS scan for an available port, this way we can run tests in parallel where each runs its own application
    let random_port_address = SocketAddr::from(([0, 0, 0, 0], 0));
    let listener =
        TcpListener::bind(random_port_address).expect("Failed to bind to bind random port.");
    let address = listener.local_addr().unwrap();

    let server = headcrab::create_web_server(listener).expect("Failed to bind address.");
    let _ = tokio::spawn(server);

    format!("http://localhost:{}", address.port())
}
