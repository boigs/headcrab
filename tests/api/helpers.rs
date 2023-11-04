use std::net::{SocketAddr, TcpListener};

use headcrab::config::Config;

pub fn spawn_app() -> String {
    // Binding to port 0 triggers an OS scan for an available port, this way we can run tests in parallel where each runs its own application
    let random_port_address = SocketAddr::from(([0, 0, 0, 0], 0));
    let listener =
        TcpListener::bind(random_port_address).expect("Failed to bind to bind random port.");
    let address = listener.local_addr().unwrap();
    let config = Config::get().expect("Failed to read configuration.");

    let server =
        headcrab::startup::create_web_server(config, listener).expect("Failed to bind address.");
    let _ = tokio::spawn(server);

    format!("localhost:{}", address.port())
}
