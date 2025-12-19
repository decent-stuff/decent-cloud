use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Telegram Bot API client for sending notifications and receiving replies.
pub struct TelegramClient {
    client: Client,
    base_url: String,
}

impl std::fmt::Debug for TelegramClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelegramClient")
            .field("base_url", &self.base_url)
            .finish()
    }
}

#[derive(Debug, Serialize)]
struct SendMessageRequest<'a> {
    chat_id: &'a str,
    text: &'a str,
    parse_mode: &'a str,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TelegramMessage {
    pub message_id: i64,
    pub chat: TelegramChat,
}

#[derive(Debug, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
struct SendMessageResponse {
    ok: bool,
    result: Option<TelegramMessage>,
    description: Option<String>,
}

impl TelegramClient {
    /// Creates a new Telegram client from environment variables.
    pub fn from_env() -> Result<Self> {
        let bot_token =
            std::env::var("TELEGRAM_BOT_TOKEN").context("TELEGRAM_BOT_TOKEN not set")?;
        let base_url = format!("https://api.telegram.org/bot{}", bot_token);

        Ok(Self {
            client: Client::new(),
            base_url,
        })
    }

    /// Check if Telegram Bot API is configured.
    pub fn is_configured() -> bool {
        std::env::var("TELEGRAM_BOT_TOKEN").is_ok()
    }

    /// Send a message to a Telegram chat.
    /// Returns the sent message with message_id for reply tracking.
    pub async fn send_message(&self, chat_id: &str, message: &str) -> Result<TelegramMessage> {
        let url = format!("{}/sendMessage", self.base_url);

        let resp = self
            .client
            .post(&url)
            .json(&SendMessageRequest {
                chat_id,
                text: message,
                parse_mode: "Markdown",
            })
            .send()
            .await
            .context("Failed to send Telegram message request")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Telegram API error {}: {}", status, body);
        }

        let response: SendMessageResponse = resp
            .json()
            .await
            .context("Failed to parse Telegram response")?;

        if !response.ok {
            anyhow::bail!(
                "Telegram API returned ok=false: {}",
                response.description.unwrap_or_default()
            );
        }

        response
            .result
            .context("Telegram API response missing result field")
    }
}

// =============================================================================
// Telegram Webhook Types
// =============================================================================

/// Telegram Update from webhook
#[derive(Debug, Deserialize)]
pub struct TelegramUpdate {
    pub update_id: i64,
    pub message: Option<TelegramIncomingMessage>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TelegramIncomingMessage {
    pub message_id: i64,
    pub chat: TelegramChat,
    pub text: Option<String>,
    pub reply_to_message: Option<Box<TelegramReplyToMessage>>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramReplyToMessage {
    pub message_id: i64,
}

/// Format a notification message for Telegram
pub fn format_notification(summary: &str, chatwoot_link: &str) -> String {
    format!(
        "*Customer Support Notification*\n\n\
        {}\n\n\
        [View in Chatwoot]({})",
        summary, chatwoot_link
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telegram_client_is_configured() {
        // Save current env
        let original = std::env::var("TELEGRAM_BOT_TOKEN").ok();

        // Test not configured
        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        assert!(!TelegramClient::is_configured());

        // Test configured
        std::env::set_var("TELEGRAM_BOT_TOKEN", "test_token");
        assert!(TelegramClient::is_configured());

        // Restore env
        if let Some(val) = original {
            std::env::set_var("TELEGRAM_BOT_TOKEN", val);
        } else {
            std::env::remove_var("TELEGRAM_BOT_TOKEN");
        }
    }

    #[test]
    fn test_telegram_client_from_env() {
        // Save current env
        let original = std::env::var("TELEGRAM_BOT_TOKEN").ok();

        // Test missing token
        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        assert!(TelegramClient::from_env().is_err());

        // Test with token
        std::env::set_var("TELEGRAM_BOT_TOKEN", "123456:ABC-DEF");
        let client = TelegramClient::from_env().unwrap();
        assert_eq!(
            client.base_url,
            "https://api.telegram.org/bot123456:ABC-DEF"
        );

        // Restore env
        if let Some(val) = original {
            std::env::set_var("TELEGRAM_BOT_TOKEN", val);
        } else {
            std::env::remove_var("TELEGRAM_BOT_TOKEN");
        }
    }

    #[test]
    fn test_send_message_request_serialization() {
        let req = SendMessageRequest {
            chat_id: "123456",
            text: "Hello, world!",
            parse_mode: "Markdown",
        };

        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["chat_id"], "123456");
        assert_eq!(json["text"], "Hello, world!");
        assert_eq!(json["parse_mode"], "Markdown");
    }

    #[test]
    fn test_telegram_message_deserialization() {
        let json = r#"{
            "message_id": 42,
            "chat": {
                "id": 123456,
                "type": "private"
            }
        }"#;

        let msg: TelegramMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.message_id, 42);
        assert_eq!(msg.chat.id, 123456);
    }

    #[test]
    fn test_send_message_response_deserialization_success() {
        let json = r#"{
            "ok": true,
            "result": {
                "message_id": 42,
                "chat": {
                    "id": 123456,
                    "type": "private"
                }
            }
        }"#;

        let resp: SendMessageResponse = serde_json::from_str(json).unwrap();
        assert!(resp.ok);
        assert!(resp.result.is_some());
        assert_eq!(resp.result.unwrap().message_id, 42);
    }

    #[test]
    fn test_send_message_response_deserialization_error() {
        let json = r#"{
            "ok": false,
            "description": "Bad Request: chat not found"
        }"#;

        let resp: SendMessageResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.ok);
        assert!(resp.result.is_none());
        assert_eq!(
            resp.description,
            Some("Bad Request: chat not found".to_string())
        );
    }

    #[test]
    fn test_telegram_update_deserialization() {
        let json = r#"{
            "update_id": 123,
            "message": {
                "message_id": 456,
                "chat": {
                    "id": 789,
                    "type": "private"
                },
                "text": "Hello bot"
            }
        }"#;

        let update: TelegramUpdate = serde_json::from_str(json).unwrap();
        assert_eq!(update.update_id, 123);
        assert!(update.message.is_some());

        let msg = update.message.unwrap();
        assert_eq!(msg.message_id, 456);
        assert_eq!(msg.chat.id, 789);
        assert_eq!(msg.text, Some("Hello bot".to_string()));
        assert!(msg.reply_to_message.is_none());
    }

    #[test]
    fn test_telegram_update_with_reply_deserialization() {
        let json = r#"{
            "update_id": 124,
            "message": {
                "message_id": 457,
                "chat": {
                    "id": 789,
                    "type": "private"
                },
                "text": "This is a reply",
                "reply_to_message": {
                    "message_id": 456
                }
            }
        }"#;

        let update: TelegramUpdate = serde_json::from_str(json).unwrap();
        let msg = update.message.unwrap();
        assert!(msg.reply_to_message.is_some());
        assert_eq!(msg.reply_to_message.unwrap().message_id, 456);
    }

    #[test]
    fn test_format_notification() {
        let message = format_notification(
            "Customer needs help with billing",
            "https://support.example.com/conversations/42",
        );

        assert!(message.contains("*Customer Support Notification*"));
        assert!(message.contains("Customer needs help with billing"));
        assert!(message.contains("https://support.example.com/conversations/42"));
        assert!(!message.contains("Contract"));
    }
}
