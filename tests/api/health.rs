use crate::helpers::spawn_app;

#[tokio::test]
async fn health_check_works() {
    let app = spawn_app();
    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://{}/health", app.base_address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(
        "healthy".to_string(),
        response.text().await.expect("The response is not text.")
    );
}
