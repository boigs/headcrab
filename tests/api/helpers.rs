use std::{net::SocketAddr, time::Duration};

use headcrab::config::Config;
use tokio::net::TcpListener;

pub struct TestApp {
    pub base_address: String,
    pub inactivity_timeout: Duration,
}

pub async fn spawn_app() -> TestApp {
    // Binding to port 0 triggers an OS scan for an available port, this way we can run tests in parallel where each runs its own application
    let random_port_address = SocketAddr::from(([0, 0, 0, 0], 0));
    let listener = TcpListener::bind(random_port_address)
        .await
        .expect("Failed to bind to bind random port.");
    let address = listener.local_addr().unwrap();
    std::env::set_var("ENVIRONMENT", "dev");
    let config = {
        let mut config = Config::get().expect("Failed to read configuration.");
        config.game.inactivity_timeout_seconds = 1;
        config
    };

    let server = headcrab::startup::create_web_server(config.clone(), listener);
    let _ = tokio::spawn(server);

    TestApp {
        base_address: format!("localhost:{}", address.port()),
        inactivity_timeout: config.game.inactivity_timeout(),
    }
}
