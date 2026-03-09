use crate::config::Config;
use crate::health_checker::HealthChecker;
use crate::telegram::Telegram;
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

    let telegram = Telegram::from_config(&config.telegram);
    let health_checker = HealthChecker::new(clickhouse_client, telegram)
        .map_err(|e| format!("Initialize HTTP client: {e}"))?;
    let health_checker = std::sync::Arc::new(health_checker);

    println!("Starting health checks...");

    let mut join_set = tokio::task::JoinSet::new();

    for target in config.targets {
        println!("Spawning task: {:?}", target.name);

        let checker = health_checker.clone();
        let target_name = target.name.clone();
        join_set.spawn(async move {
            checker.run_check_loop(target).await;
            target_name
        });
    }

    // Wait for the first task termination; any finished task is treated as unexpected.
    match join_set.join_next().await {
        Some(result) => match result {
            Ok(target_name) => {
                eprintln!(
                    "Health check task for '{}' exited unexpectedly",
                    target_name
                );
                Err(format!("Health check task for '{}' stopped running", target_name).into())
            }
            Err(e) if e.is_panic() => {
                eprintln!("Health check task panicked: {:?}", e);
                Err(format!("Health check task panicked: {:?}", e).into())
            }
            Err(e) => {
                eprintln!("Health check task failed: {:?}", e);
                Err(format!("Health check task failed: {:?}", e).into())
            }
        },
        None => Ok(()),
    }
}

pub async fn run_default() -> Result<(), Box<dyn Error>> {
    run(DEFAULT_CONFIG_PATH).await
}
