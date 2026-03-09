use envconfig::Envconfig;
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::Path;

#[derive(Envconfig)]
struct EnvConfig {
    #[envconfig(from = "TELEGRAM_CHAT_ID")]
    telegram_chat_id: i64,
    #[envconfig(from = "TELEGRAM_BOT_TOKEN")]
    telegram_bot_token: String,
    #[envconfig(from = "CLICKHOUSE_URL")]
    clickhouse_url: String,
    #[envconfig(from = "CLICKHOUSE_USER")]
    clickhouse_user: String,
    #[envconfig(from = "CLICKHOUSE_PASSWORD")]
    clickhouse_password: String,
}

pub struct Config {
    pub telegram: Telegram,
    pub clickhouse: Clickhouse,
    pub targets: Vec<Target>,
}

#[derive(Deserialize)]
struct FileConfig {
    pub targets: Vec<Target>,
}

pub struct Telegram {
    pub chat_id: i64,
    pub bot_token: String,
}
pub struct Clickhouse {
    pub url: String,
    pub user: String,
    pub password: String,
}

fn default_interval_seconds() -> u64 {
    60
}

#[derive(Deserialize, Clone)]
pub struct Target {
    pub name: String,
    pub url: String,
    pub timeout_ms: u64,
    #[serde(default = "default_interval_seconds")]
    pub interval_seconds: u64,
}

impl Config {
    pub fn from_path_and_env(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let content = &fs::read_to_string(&path).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Config file not found at path: {}", path.as_ref().display()),
            )
        })?;

        let file_config: FileConfig = toml::from_str(content).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to parse config file: {e}"),
            )
        })?;

        let env = EnvConfig::init_from_env()?;

        Ok(Self {
            telegram: Telegram {
                chat_id: env.telegram_chat_id,
                bot_token: env.telegram_bot_token,
            },
            clickhouse: Clickhouse {
                url: env.clickhouse_url,
                user: env.clickhouse_user,
                password: env.clickhouse_password,
            },
            targets: file_config.targets,
        })
    }
}
