use prometheus::{IntGauge, Registry};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref ACTIVE_GAMES: IntGauge =
        IntGauge::new("repeti2.headcrab.active_games", "Active ongoing games").expect("metric cannot be created");
    pub static ref CONNECTED_PLAYERS: IntGauge =
        IntGauge::new("repeti2.headcrab.connected_players", "Amount of players connected")
            .expect("metric cannot be created");
}

pub fn register_metrics() {
    REGISTRY
        .register(Box::new(ACTIVE_GAMES.clone()))
        .expect("collector cannot be registered");

    REGISTRY
        .register(Box::new(CONNECTED_PLAYERS.clone()))
        .expect("collector cannot be registered");
}
