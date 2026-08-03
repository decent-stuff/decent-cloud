//! SMS provider abstraction. TextBee is the only supported backend.

use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

/// SMS provider trait - implement for each SMS backend.
#[async_trait]
pub trait SmsProvider: Send + Sync {
    /// Send an SMS message. Returns a message ID on success.
    async fn send_sms(&self, to: &str, message: &str) -> Result<String>;
    /// Provider name for logging.
    fn name(&self) -> &'static str;
}

/// Get the configured SMS provider (TextBee), or `None` if unconfigured.
pub fn get_sms_provider() -> Option<Box<dyn SmsProvider>> {
    if !TextBeeClient::is_configured() {
        return None;
    }
    match TextBeeClient::from_env() {
        Ok(c) => Some(Box::new(c) as _),
        Err(e) => {
            tracing::error!("TextBee is configured but failed to initialize: {:#}", e);
            None
        }
    }
}

/// Check if the SMS provider (TextBee) is configured.
pub fn is_sms_configured() -> bool {
    TextBeeClient::is_configured()
}

// ============================================================================
// TextBee Implementation
// ============================================================================

pub struct TextBeeClient {
    client: Client,
    api_url: String,
    device_id: String,
    api_key: String,
}

impl std::fmt::Debug for TextBeeClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextBeeClient")
            .field("api_url", &self.api_url)
            .field("device_id", &self.device_id)
            .finish()
    }
}

#[derive(Debug, Deserialize)]
struct TextBeeResponse {
    #[serde(default)]
    data: Option<TextBeeData>,
}

#[derive(Debug, Deserialize)]
struct TextBeeData {
    #[serde(default)]
    id: Option<String>,
}

impl TextBeeClient {
    pub fn from_env() -> Result<Self> {
        let api_url = std::env::var("TEXTBEE_API_URL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "https://api.textbee.dev".to_string());
        Ok(Self {
            client: crate::http_util::http_client(),
            api_url,
            device_id: std::env::var("TEXTBEE_DEVICE_ID").context("TEXTBEE_DEVICE_ID not set")?,
            api_key: std::env::var("TEXTBEE_API_KEY").context("TEXTBEE_API_KEY not set")?,
        })
    }

    pub fn is_configured() -> bool {
        std::env::var("TEXTBEE_DEVICE_ID").is_ok() && std::env::var("TEXTBEE_API_KEY").is_ok()
    }
}

#[async_trait]
impl SmsProvider for TextBeeClient {
    async fn send_sms(&self, to: &str, message: &str) -> Result<String> {
        let url = format!(
            "{}/api/v1/gateway/devices/{}/send-sms",
            self.api_url, self.device_id
        );

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.api_key)
            .json(&serde_json::json!({
                "recipients": [to],
                "message": message
            }))
            .send()
            .await
            .context("Failed to send TextBee request")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let msg = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("TextBee API error {}: {}", status, msg);
        }

        // Parse response to extract message ID if available
        let response: TextBeeResponse = match resp.json().await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(
                    "TextBee response parsing failed (SMS may still have been sent): {:#}",
                    e
                );
                TextBeeResponse { data: None }
            }
        };

        let msg_id = response
            .data
            .and_then(|d| d.id)
            .unwrap_or_else(|| format!("textbee-{}", chrono::Utc::now().timestamp()));

        Ok(msg_id)
    }

    fn name(&self) -> &'static str {
        "textbee"
    }
}

/// Format a notification message for SMS (shorter than Telegram/email).
pub fn format_sms_notification(summary: &str) -> String {
    format!("Support alert: {}. Check Chatwoot for details.", summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn with_env<F, R>(vars: &[(&str, Option<&str>)], f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let orig: Vec<_> = vars
            .iter()
            .map(|(k, _)| (*k, std::env::var(k).ok()))
            .collect();
        for (k, v) in vars {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
        let result = f();
        for (k, v) in orig {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
        result
    }

    #[test]
    #[serial]
    fn test_textbee_is_configured() {
        with_env(
            &[("TEXTBEE_DEVICE_ID", None), ("TEXTBEE_API_KEY", None)],
            || assert!(!TextBeeClient::is_configured()),
        );

        with_env(
            &[
                ("TEXTBEE_DEVICE_ID", Some("dev123")),
                ("TEXTBEE_API_KEY", Some("key456")),
            ],
            || assert!(TextBeeClient::is_configured()),
        );
    }

    #[test]
    #[serial]
    fn test_textbee_default_api_url() {
        with_env(
            &[
                ("TEXTBEE_DEVICE_ID", Some("dev")),
                ("TEXTBEE_API_KEY", Some("key")),
                ("TEXTBEE_API_URL", None),
            ],
            || {
                let client = TextBeeClient::from_env().unwrap();
                assert_eq!(client.api_url, "https://api.textbee.dev");
            },
        );
    }

    #[test]
    #[serial]
    fn test_textbee_custom_api_url() {
        with_env(
            &[
                ("TEXTBEE_DEVICE_ID", Some("dev")),
                ("TEXTBEE_API_KEY", Some("key")),
                ("TEXTBEE_API_URL", Some("https://my.textbee.local")),
            ],
            || {
                let client = TextBeeClient::from_env().unwrap();
                assert_eq!(client.api_url, "https://my.textbee.local");
            },
        );
    }

    #[test]
    #[serial]
    fn test_textbee_provider_name() {
        with_env(
            &[
                ("TEXTBEE_DEVICE_ID", Some("dev")),
                ("TEXTBEE_API_KEY", Some("key")),
            ],
            || {
                let client = TextBeeClient::from_env().unwrap();
                assert_eq!(client.name(), "textbee");
            },
        );
    }

    #[test]
    #[serial]
    fn test_is_sms_configured() {
        // The boot-time warning fires when is_sms_configured() is false: true
        // only when the TextBee provider is fully configured.
        with_env(
            &[("TEXTBEE_DEVICE_ID", None), ("TEXTBEE_API_KEY", None)],
            || assert!(!is_sms_configured(), "no TextBee keys -> not configured"),
        );

        with_env(
            &[
                ("TEXTBEE_DEVICE_ID", Some("dev123")),
                ("TEXTBEE_API_KEY", Some("key456")),
            ],
            || assert!(is_sms_configured(), "TextBee keys set -> configured"),
        );
    }

    #[test]
    fn test_format_sms_notification() {
        let msg = format_sms_notification("Customer needs help");
        assert!(msg.contains("Customer needs help"));
        assert!(msg.contains("Support alert"));
        assert!(msg.len() < 160);
    }
}
