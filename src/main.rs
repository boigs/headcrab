use std::net::{SocketAddr, TcpListener};

use headcrab::create_web_server;

#[tokio::main]
async fn main() -> Result<(), hyper::Error> {
    let address = SocketAddr::from(([0, 0, 0, 0], 4000));
    let listener = TcpListener::bind(address).expect("Failed to bind to address");

    create_web_server(listener)?.await
}
