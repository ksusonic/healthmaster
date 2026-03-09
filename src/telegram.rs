use crate::config::Telegram as TelegramConfig;
use crate::health_checker::HealthCheckResult;
use teloxide::Bot;
use teloxide::prelude::Requester;
use teloxide::types::{ChatId, ParseMode};

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

        // Escape special characters for MarkdownV2
        let target_escaped = Self::escape_markdown_v2(&result.target);
        let url_escaped = Self::escape_markdown_v2(&result.url);
        let reason_escaped = Self::escape_markdown_v2(&reason);

        format!(
            "🔴 *Health Check Failed: {}*\n\n*URL:* {}\n*Latency:* {}ms\n*Reason:* {}",
            target_escaped, url_escaped, result.latency_ms, reason_escaped
        )
    }

    fn escape_markdown_v2(text: &str) -> String {
        // MarkdownV2 requires escaping: _*[]()~`>#+-=|{}.!
        text.chars()
            .map(|c| match c {
                '_' | '*' | '[' | ']' | '(' | ')' | '~' | '`' | '>' | '#' | '+' | '-' | '='
                | '|' | '{' | '}' | '.' | '!' => {
                    format!("\\{}", c)
                }
                _ => c.to_string(),
            })
            .collect()
    }

    pub async fn send_error(
        &self,
        result: &HealthCheckResult,
    ) -> Result<(), teloxide::RequestError> {
        let mut request = self
            .bot
            .send_message(self.chat_id, Self::build_error_message(result));
        request.parse_mode = Some(ParseMode::MarkdownV2);
        request.await?;
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
