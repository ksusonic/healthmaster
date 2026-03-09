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

#[derive(Deserialize)]
pub struct Target {
    pub name: String,
    pub url: String,
    pub timeout_ms: u64,
    pub interval_seconds: u32,
    #[serde(default = "default_retry")]
    pub retry: usize,
}

fn default_retry() -> usize {
    3
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
                format!(
                    "Failed to parse config file at {}: {e}",
                    path.as_ref().display()
                ),
            )
        })?;

        validate_targets(&file_config.targets)?;

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

fn validate_targets(targets: &[Target]) -> Result<(), std::io::Error> {
    for (idx, target) in targets.iter().enumerate() {
        if target.interval_seconds == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Invalid target config at index {idx} ('{}'): interval_seconds must be greater than 0",
                    target.name
                ),
            ));
        }
        if target.retry == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Invalid target config at index {idx} ('{}'): retry must be at least 1",
                    target.name
                ),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Config;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn write_temp_file(content: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        path.push(format!("healthmaster-config-test-{now}.toml"));

        fs::write(&path, content).expect("temporary config should be writable");
        path
    }

    fn set_required_env_vars() {
        unsafe {
            std::env::set_var("TELEGRAM_CHAT_ID", "42");
            std::env::set_var("TELEGRAM_BOT_TOKEN", "token");
            std::env::set_var("CLICKHOUSE_URL", "http://localhost:8123");
            std::env::set_var("CLICKHOUSE_USER", "default");
            std::env::set_var("CLICKHOUSE_PASSWORD", "default");
        }
    }

    #[test]
    fn from_path_and_env_returns_not_found_for_missing_file() {
        let path = PathBuf::from("/tmp/healthmaster-does-not-exist.toml");
        let error = match Config::from_path_and_env(&path) {
            Ok(_) => panic!("missing file should fail"),
            Err(e) => e,
        };
        assert!(error.to_string().contains("Config file not found at path"));
    }

    #[test]
    fn from_path_and_env_returns_invalid_data_for_bad_toml() {
        let path = write_temp_file("[[targets]\nname =");
        let error = match Config::from_path_and_env(&path) {
            Ok(_) => panic!("invalid toml should fail"),
            Err(e) => e,
        };
        assert!(error.to_string().contains("Failed to parse config file at"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn from_path_and_env_fails_when_required_env_var_is_missing() {
        let _guard = env_lock().lock().expect("env lock should not be poisoned");

        let path = write_temp_file(
            "[[targets]]\nname = \"example\"\nurl = \"https://example.com\"\ntimeout_ms = 1000\ninterval_seconds = 30\n",
        );

        unsafe {
            std::env::set_var("TELEGRAM_BOT_TOKEN", "token");
            std::env::set_var("CLICKHOUSE_URL", "http://localhost:8123");
            std::env::set_var("CLICKHOUSE_USER", "default");
            std::env::set_var("CLICKHOUSE_PASSWORD", "default");
            std::env::remove_var("TELEGRAM_CHAT_ID");
        }

        let error = match Config::from_path_and_env(&path) {
            Ok(_) => panic!("missing env should fail"),
            Err(e) => e,
        };
        assert!(error.to_string().contains("TELEGRAM_CHAT_ID"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn from_path_and_env_fails_when_target_interval_is_zero() {
        let _guard = env_lock().lock().expect("env lock should not be poisoned");
        set_required_env_vars();

        let path = write_temp_file(
            "[[targets]]\nname = \"example\"\nurl = \"https://example.com\"\ntimeout_ms = 1000\ninterval_seconds = 0\nretry = 3\n",
        );

        let error = match Config::from_path_and_env(&path) {
            Ok(_) => panic!("zero interval should fail"),
            Err(e) => e,
        };
        assert!(
            error
                .to_string()
                .contains("interval_seconds must be greater than 0")
        );

        let _ = fs::remove_file(path);
    }
}
