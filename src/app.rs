use crate::config::Config;
use std::error::Error;
use std::path::Path;

pub const DEFAULT_CONFIG_PATH: &str = "config.toml";

pub fn load_config(path: impl AsRef<Path>) -> Result<Config, Box<dyn Error>> {
    Config::from_path_and_env(path)
}

pub async fn run(path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    let config = load_config(path)?;
    println!("Config loaded");

    let _clickhouse_client = crate::clickhouse::connect(config.clickhouse).await?;
    println!("ClickHouse connected");

    Ok(())
}

pub async fn run_default() -> Result<(), Box<dyn Error>> {
    run(DEFAULT_CONFIG_PATH).await
}
