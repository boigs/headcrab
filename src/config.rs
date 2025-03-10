use std::time::Duration;

use config::ConfigError;
use serde::Deserialize;
use serde_aux::prelude::deserialize_number_from_string;

use crate::error::Error;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub application: ApplicationSettings,
    pub game: GameSettings,
    pub words_file: String,
    pub allow_cors: bool,
}

#[derive(serde::Deserialize, Clone)]
pub struct ApplicationSettings {
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
}

#[derive(serde::Deserialize, Clone)]
pub struct GameSettings {
    pub inactivity_timeout_seconds: u64,
}

impl GameSettings {
    pub fn inactivity_timeout(&self) -> Duration {
        Duration::from_secs(self.inactivity_timeout_seconds)
    }
}

impl Config {
    pub fn get() -> Result<Config, ConfigError> {
        let base_path = std::env::current_dir().expect("Failed to determine the current directory");
        let configuration_directory = base_path.join("config");

        let environment: Environment = std::env::var("ENVIRONMENT")
            .expect("ENVIRONMENT variable is not set.")
            .try_into()
            .expect("Failed to parse ENVIRONMENT variable.");

        let environment_filename = format!("{}.yaml", environment.as_str());

        let config = config::Config::builder()
            .add_source(config::File::from(
                configuration_directory.join("base.yaml"),
            ))
            .add_source(config::File::from(
                configuration_directory.join(environment_filename),
            ))
            .build()?;

        config.try_deserialize::<Config>()
    }
}

enum Environment {
    Dev,
    Prod,
}

const DEV: &str = "dev";
const PROD: &str = "prod";

impl Environment {
    fn as_str(&self) -> &'static str {
        match self {
            Environment::Dev => DEV,
            Environment::Prod => PROD,
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = crate::error::Error;

    fn try_from(string: String) -> Result<Self, Self::Error> {
        match string.to_lowercase().as_str() {
            DEV => Ok(Self::Dev),
            PROD => Ok(Self::Prod),
            other => Err(Error::log_and_create_internal(&format!(
                "{other} is not a supported environment. Use either `{DEV}` or `{PROD}`.",
            ))),
        }
    }
}
