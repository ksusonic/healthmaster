use crate::config::Telegram as TelegramConfig;
use crate::health_checker::HealthCheckResult;
use teloxide::Bot;
use teloxide::prelude::Requester;
use teloxide::types::ChatId;

pub struct Telegram {
    bot: Bot,
    chat_id: ChatId,
}

impl Telegram {
    pub fn from_config(config: &TelegramConfig) -> Self {
        Self {
            bot: Bot::new(config.bot_token.clone()),
            chat_id: ChatId(config.chat_id),
        }
    }

    fn build_error_message(result: &HealthCheckResult) -> String {
        let reason = if result.status != 0 {
            format!("HTTP status {}", result.status)
        } else {
            result.error.clone()
        };

        format!(
            "Health check failed\nTarget: {}\nURL: {}\nLatency: {}ms\nReason: {}",
            result.target, result.url, result.latency_ms, reason
        )
    }

    pub async fn send_error(
        &self,
        result: &HealthCheckResult,
    ) -> Result<(), teloxide::RequestError> {
        self.bot
            .send_message(self.chat_id, Self::build_error_message(result))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Telegram;
    use crate::health_checker::HealthCheckResult;

    #[test]
    fn build_error_message_uses_http_status_for_unsuccessful_response() {
        let message = Telegram::build_error_message(&HealthCheckResult {
            timestamp: chrono::Utc::now(),
            target: "svc".to_string(),
            url: "https://svc.local/health".to_string(),
            status: 503,
            latency_ms: 150,
            success: false,
            error: String::new(),
        });

        assert!(message.contains("HTTP status 503"));
    }

    #[test]
    fn build_error_message_uses_transport_error_when_status_missing() {
        let message = Telegram::build_error_message(&HealthCheckResult {
            timestamp: chrono::Utc::now(),
            target: "svc".to_string(),
            url: "https://svc.local/health".to_string(),
            status: 0,
            latency_ms: 10,
            success: false,
            error: "connection reset".to_string(),
        });

        assert!(message.contains("connection reset"));
    }
}
