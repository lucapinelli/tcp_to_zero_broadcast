use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Tcp {
    pub endpoint: String,
    pub message_termination_byte: u8,
}

#[derive(Debug, Deserialize)]
pub struct Zero {
    pub pub_endpoint: String,
    pub pub_topic: String,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub tcp: Tcp,
    pub zero: Zero,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();

        // merge the "default" configuration file
        s.merge(File::with_name("config/default"))?;

        // add in a local configuration file (if any)
        s.merge(File::with_name("config/local").required(false))?;

        // deserialize (and thus freeze) the entire configuration
        s.try_into()
    }
}
