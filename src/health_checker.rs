use crate::config::Target;
use crate::telegram::Telegram;
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
    pub success: bool,
    pub error: String,
}

pub struct HealthChecker {
    client: reqwest::Client,
    clickhouse: Client,
    telegram: Telegram,
}

impl HealthChecker {
    pub fn new(clickhouse: Client, telegram: Telegram) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()?;
        Ok(Self {
            client,
            clickhouse,
            telegram,
        })
    }

    pub async fn check_target(&self, target: &Target) -> HealthCheckResult {
        let start = std::time::Instant::now();
        let timestamp = Utc::now();

        let timeout = Duration::from_millis(target.timeout_ms);

        // Retry logic: attempt the request up to target.retry times
        // with a short 50ms delay between attempts to avoid long waits
        let mut last_error = None;
        let mut attempts = 0;

        for attempt in 0..target.retry {
            attempts = attempt + 1;
            let request = self.client.get(&target.url).timeout(timeout);

            match request.send().await {
                Ok(response) => {
                    let status = response.status().as_u16();
                    let mut success = response.status().is_success();
                    let mut error = String::new();

                    // Drain the body so hyper can return this connection to the pool.
                    if let Err(e) = response.bytes().await {
                        success = false;
                        error = format!("read response body: {e}");
                    }

                    let latency_ms = start.elapsed().as_millis() as u32;

                    return HealthCheckResult {
                        timestamp,
                        target: target.name.clone(),
                        url: target.url.clone(),
                        status,
                        latency_ms,
                        success,
                        error,
                    };
                }
                Err(e) => {
                    last_error = Some(e);

                    // If this isn't the last attempt, wait briefly before retrying
                    if attempt + 1 < target.retry {
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                }
            }
        }

        // All retries exhausted
        let latency_ms = start.elapsed().as_millis() as u32;
        let error_msg = match last_error {
            Some(e) => format!("{} (after {} attempts)", e, attempts),
            None => format!("Unknown error (after {} attempts)", attempts),
        };

        HealthCheckResult {
            timestamp,
            target: target.name.clone(),
            url: target.url.clone(),
            status: 0,
            latency_ms,
            success: false,
            error: error_msg,
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
        if target.interval_seconds == 0 {
            eprintln!(
                "Target '{}' has interval_seconds=0; skipping to avoid panic",
                target.name
            );
            return;
        }

        let duration = Duration::from_secs(target.interval_seconds.into());
        let mut interval = tokio::time::interval(duration);

        // The first tick completes immediately, so we consume it
        interval.tick().await;

        loop {
            let result = self.check_target(&target).await;

            let success_str = if result.success { "ok" } else { "err" };
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

            if !result.success
                && let Err(e) = self.telegram.send_error(&result).await
            {
                eprintln!("Telegram notification failed: {}", e);
            }

            if let Err(e) = self.store_result(result).await {
                eprintln!("Store result: {}", e);
            }

            // Wait for the next tick
            interval.tick().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HealthChecker;
    use crate::config::{Target, Telegram as TelegramConfig};
    use crate::telegram::Telegram;
    use clickhouse::Client;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::time::{Duration, sleep, timeout};

    async fn spawn_http_server(status_code: u16, body: &'static str, delay_ms: u64) -> String {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test listener should bind");
        let addr = listener
            .local_addr()
            .expect("listener should have local address");

        tokio::spawn(async move {
            let (mut stream, _) = listener
                .accept()
                .await
                .expect("server should accept one connection");
            let mut request_buf = [0_u8; 1024];
            let _ = stream.read(&mut request_buf).await;

            if delay_ms > 0 {
                sleep(Duration::from_millis(delay_ms)).await;
            }

            let response = format!(
                "HTTP/1.1 {} TEST\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                status_code,
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes()).await;
        });

        format!("http://{}", addr)
    }

    fn make_target(url: String, timeout_ms: u64) -> Target {
        Target {
            name: "example".to_string(),
            url,
            timeout_ms,
            interval_seconds: 60,
            retry: 3,
        }
    }

    fn make_telegram_config() -> TelegramConfig {
        TelegramConfig {
            chat_id: 42,
            bot_token: "test-token".to_string(),
        }
    }

    fn make_checker() -> HealthChecker {
        let telegram = Telegram::from_config(&make_telegram_config());
        HealthChecker::new(Client::default(), telegram).expect("checker should initialize")
    }

    #[tokio::test]
    async fn check_target_returns_success_for_2xx_response() {
        let url = spawn_http_server(200, "ok", 0).await;
        let checker = make_checker();

        let result = checker.check_target(&make_target(url, 1000)).await;

        assert_eq!(result.status, 200);
        assert_eq!(result.success, true);
        assert!(result.error.is_empty());
    }

    #[tokio::test]
    async fn check_target_marks_non_2xx_as_unsuccessful() {
        let url = spawn_http_server(503, "unavailable", 0).await;
        let checker = make_checker();

        let result = checker.check_target(&make_target(url, 1000)).await;

        assert_eq!(result.status, 503);
        assert_eq!(result.success, false);
        assert!(result.error.is_empty());
    }

    #[tokio::test]
    async fn check_target_returns_error_when_request_fails() {
        // Port 9 is typically closed locally, giving a deterministic connect error.
        let url = "http://127.0.0.1:9".to_string();
        let checker = make_checker();

        let result = checker.check_target(&make_target(url, 100)).await;

        assert_eq!(result.status, 0);
        assert_eq!(result.success, false);
        assert!(!result.error.is_empty());
    }

    #[tokio::test]
    async fn run_check_loop_returns_immediately_for_zero_interval() {
        let checker = make_checker();
        let target = Target {
            name: "invalid".to_string(),
            url: "https://example.com".to_string(),
            timeout_ms: 1000,
            interval_seconds: 0,
            retry: 3,
        };

        let result = timeout(Duration::from_millis(50), checker.run_check_loop(target)).await;
        assert!(result.is_ok(), "zero interval should not block or panic");
    }

    #[tokio::test]
    async fn check_target_retries_on_failure_but_completes_quickly() {
        // Port 9 is typically closed, so this will fail quickly
        let url = "http://127.0.0.1:9".to_string();
        let checker = make_checker();
        let target = Target {
            name: "example".to_string(),
            url,
            timeout_ms: 100, // Short timeout
            interval_seconds: 60,
            retry: 3, // 3 retries
        };

        let start = std::time::Instant::now();
        let result = checker.check_target(&target).await;
        let elapsed = start.elapsed();

        // Should fail after retries
        assert_eq!(result.success, false);
        assert!(result.error.contains("after 3 attempts"));

        // Should complete in less than 500ms (100ms timeout * 3 attempts + 50ms * 2 delays = ~400ms max)
        assert!(
            elapsed < Duration::from_millis(500),
            "Retries took too long: {:?}",
            elapsed
        );
    }
}
