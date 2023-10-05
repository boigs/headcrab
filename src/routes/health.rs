use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub async fn get() -> Response {
    (StatusCode::OK, "healthy").into_response()
}
