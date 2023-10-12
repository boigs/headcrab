use headcrab::startup;
use std::net::{SocketAddr, TcpListener};

#[tokio::main]
async fn main() -> Result<(), hyper::Error> {
    //env_logger::Builder::from_env(Env::default().default_filter_or(log::Level::Error.as_str())).init();
    env_logger::init();

    let address = SocketAddr::from(([0, 0, 0, 0], 4000));
    let listener = TcpListener::bind(address).expect("Failed to bind to address");
    startup::create_web_server(listener)?.await
}
