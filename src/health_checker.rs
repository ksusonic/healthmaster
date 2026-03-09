use crate::config::Target;
use chrono::{DateTime, Utc};
use clickhouse::Client;
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize, clickhouse::Row)]
pub struct HealthCheckResult {
    #[serde(with = "clickhouse::serde::chrono::datetime")]
    pub timestamp: DateTime<Utc>,
    pub target: String,
    pub url: String,
    pub status: u16,
    pub latency_ms: u32,
    pub success: u8,
    pub error: String,
}

pub struct HealthChecker {
    client: reqwest::Client,
    clickhouse: Client,
}

impl HealthChecker {
    pub fn new(clickhouse: Client) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()?;
        Ok(Self { client, clickhouse })
    }

    pub async fn check_target(&self, target: &Target) -> HealthCheckResult {
        let start = std::time::Instant::now();
        let timestamp = Utc::now();

        let timeout = Duration::from_millis(target.timeout_ms);
        let request = self.client.get(&target.url).timeout(timeout);

        match request.send().await {
            Ok(response) => {
                let status = response.status().as_u16();
                let mut success = if response.status().is_success() { 1 } else { 0 };
                let mut error = String::new();

                // Drain the body so hyper can return this connection to the pool.
                if let Err(e) = response.bytes().await {
                    success = 0;
                    error = format!("read response body: {e}");
                }

                let latency_ms = start.elapsed().as_millis() as u32;

                HealthCheckResult {
                    timestamp,
                    target: target.name.clone(),
                    url: target.url.clone(),
                    status,
                    latency_ms,
                    success,
                    error,
                }
            }
            Err(e) => {
                let latency_ms = start.elapsed().as_millis() as u32;

                HealthCheckResult {
                    timestamp,
                    target: target.name.clone(),
                    url: target.url.clone(),
                    status: 0,
                    latency_ms,
                    success: 0,
                    error: e.to_string(),
                }
            }
        }
    }

    pub async fn store_result(
        &self,
        result: HealthCheckResult,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut insert = self
            .clickhouse
            .insert::<HealthCheckResult>("health_checks")
            .await?;
        insert.write(&result).await?;
        insert.end().await?;
        Ok(())
    }

    pub async fn run_check_loop(&self, target: Target) {
        let duration = Duration::from_secs(target.interval_seconds.into());
        let mut interval = tokio::time::interval(duration);

        // The first tick completes immediately, so we consume it
        interval.tick().await;

        loop {
            let result = self.check_target(&target).await;

            let success_str = if result.success == 1 { "✓" } else { "✗" };
            println!(
                "{} {} - {} ({}ms) - status: {}",
                success_str,
                result.target,
                result.url,
                result.latency_ms,
                if result.status > 0 {
                    result.status.to_string()
                } else {
                    result.error.clone()
                }
            );

            if let Err(e) = self.store_result(result).await {
                eprintln!("Store result: {}", e);
            }

            // Wait for the next tick
            interval.tick().await;
        }
    }
}
