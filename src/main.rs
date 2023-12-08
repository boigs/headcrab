use headcrab::{config::Config, metrics, startup};
use std::{
    net::{Ipv4Addr, SocketAddr, TcpListener},
    str::FromStr,
};

#[tokio::main]
async fn main() -> Result<(), hyper::Error> {
    std_logger::Config::logfmt().init();
    metrics::register_metrics();

    let config = Config::get().expect("Failed to read configuration.");
    let address = SocketAddr::from((
        Ipv4Addr::from_str(&config.application.host).expect("Invalid host"),
        config.application.port,
    ));
    let listener = TcpListener::bind(address).expect("Failed to bind to address");
    startup::create_web_server(config, listener)?.await
}
