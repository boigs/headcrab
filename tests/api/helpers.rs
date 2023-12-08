use std::{
    net::{SocketAddr, TcpListener},
    time::Duration,
};

use headcrab::config::Config;

pub struct TestApp {
    pub base_address: String,
    pub inactivity_timeout: Duration,
}

pub fn spawn_app() -> TestApp {
    // Binding to port 0 triggers an OS scan for an available port, this way we can run tests in parallel where each runs its own application
    let random_port_address = SocketAddr::from(([0, 0, 0, 0], 0));
    let listener =
        TcpListener::bind(random_port_address).expect("Failed to bind to bind random port.");
    let address = listener.local_addr().unwrap();
    let config = {
        let mut config = Config::get().expect("Failed to read configuration.");
        config.game.inactivity_timeout_seconds = 2;
        config
    };

    let server = headcrab::startup::create_web_server(config.clone(), listener)
        .expect("Failed to bind address.");
    let _ = tokio::spawn(server);

    TestApp {
        base_address: format!("localhost:{}", address.port()),
        inactivity_timeout: config.game.inactivity_timeout(),
    }
}
