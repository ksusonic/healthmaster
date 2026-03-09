use crate::config::Config;
use crate::health_checker::HealthChecker;
use std::error::Error;
use std::path::Path;

pub const DEFAULT_CONFIG_PATH: &str = "config.toml";

pub fn load_config(path: impl AsRef<Path>) -> Result<Config, Box<dyn Error>> {
    Config::from_path_and_env(path)
}

pub async fn run(path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    let config = load_config(path)?;
    println!("Config loaded with {} targets", config.targets.len());

    let clickhouse_client = crate::clickhouse::connect(config.clickhouse).await?;
    println!("ClickHouse connected");

    let health_checker = HealthChecker::new(clickhouse_client);
    let health_checker = std::sync::Arc::new(health_checker);

    println!("Starting health checks...");

    let mut handles = vec![];

    for target in config.targets {
        let checker = health_checker.clone();
        let handle = tokio::spawn(async move {
            checker.run_check_loop(target).await;
        });
        handles.push(handle);
    }

    // Wait for all tasks (they run forever)
    for handle in handles {
        let _ = handle.await;
    }

    Ok(())
}

pub async fn run_default() -> Result<(), Box<dyn Error>> {
    run(DEFAULT_CONFIG_PATH).await
}
