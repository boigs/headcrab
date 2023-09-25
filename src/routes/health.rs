use axum::http::StatusCode;

pub async fn get() -> (StatusCode, String) {
    (StatusCode::OK, "healthy".to_string())
}
