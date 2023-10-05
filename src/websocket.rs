use axum::extract::ws::{Message, WebSocket};

pub async fn send_error_and_close(mut websocket: WebSocket, message: &str) {
    if websocket
        .send(Message::Text(message.to_string()))
        .await
        .is_err()
    {
        println!("ERROR: Sent Error '{message}' to the browser but the WebSocket is closed.")
    }
    if websocket.close().await.is_err() {
        println!("ERROR: Could not close WebSocket after sending an error.")
    }
}
