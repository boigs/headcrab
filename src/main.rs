use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    // build our application with a route
    let app = Router::new()
        .route("/", get(handler))
        .route("/tet", post(tet));

    // run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handler() -> (StatusCode, Json<Person>) {
    let user = Person {
        id: 1337,
        name: String::from("sergirk"),
    };

    (StatusCode::OK, Json(user))
}

async fn tet(Json(input): Json<Input>) -> (StatusCode, Json<Person>) {
    (
        StatusCode::OK,
        Json(Person {
            id: 3848,
            name: input.text,
        }),
    )
}

#[derive(Serialize)]
struct Person {
    id: u64,
    name: String,
}

#[derive(Deserialize)]
struct Input {
    text: String,
}
