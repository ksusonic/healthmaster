use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub interval_seconds: u64,
    pub telegram: Telegram,
    pub targets: Vec<Target>,
}

#[derive(Deserialize)]
pub struct Telegram {
    pub chat_id: i64,
    #[serde(skip)]
    pub bot_token: String,
}

impl std::fmt::Debug for Telegram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Telegram")
            .field("chat_id", &self.chat_id)
            .field("bot_token", &"<obfuscated>")
            .finish()
    }
}

#[derive(Debug, Deserialize)]
pub struct Target {
    pub name: String,
    pub url: String,
    pub timeout_ms: u64,
}

impl Config {
    pub fn from_path_and_env(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let content = &fs::read_to_string(&path).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Config file not found at path: {}", path.as_ref().display()),
            )
        })?;
        let mut config: Config = toml::from_str(content).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to parse config file: {e}"),
            )
        })?;

        config.telegram.bot_token = std::env::var("TELEGRAM_BOT_TOKEN").map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "TELEGRAM_BOT_TOKEN env var is not set",
            )
        })?;

        Ok(config)
    }
}
