//! Twilio SMS client for provider notifications.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;

/// Twilio SMS client for sending notifications.
pub struct TwilioClient {
    client: Client,
    account_sid: String,
    auth_token: String,
    from_number: String,
}

impl std::fmt::Debug for TwilioClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TwilioClient")
            .field("account_sid", &self.account_sid)
            .field("from_number", &self.from_number)
            .finish()
    }
}

#[derive(Debug, Deserialize)]
struct TwilioMessageResponse {
    sid: String,
    #[allow(dead_code)]
    status: String,
}

#[derive(Debug, Deserialize)]
struct TwilioErrorResponse {
    message: String,
    #[allow(dead_code)]
    code: Option<i32>,
}

impl TwilioClient {
    /// Creates a new Twilio client from environment variables.
    pub fn from_env() -> Result<Self> {
        let account_sid =
            std::env::var("TWILIO_ACCOUNT_SID").context("TWILIO_ACCOUNT_SID not set")?;
        let auth_token = std::env::var("TWILIO_AUTH_TOKEN").context("TWILIO_AUTH_TOKEN not set")?;
        let from_number =
            std::env::var("TWILIO_PHONE_NUMBER").context("TWILIO_PHONE_NUMBER not set")?;

        Ok(Self {
            client: Client::new(),
            account_sid,
            auth_token,
            from_number,
        })
    }

    /// Check if Twilio is configured.
    pub fn is_configured() -> bool {
        std::env::var("TWILIO_ACCOUNT_SID").is_ok()
            && std::env::var("TWILIO_AUTH_TOKEN").is_ok()
            && std::env::var("TWILIO_PHONE_NUMBER").is_ok()
    }

    /// Send an SMS message.
    pub async fn send_sms(&self, to: &str, message: &str) -> Result<String> {
        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
            self.account_sid
        );

        let resp = self
            .client
            .post(&url)
            .basic_auth(&self.account_sid, Some(&self.auth_token))
            .form(&[("To", to), ("From", &self.from_number), ("Body", message)])
            .send()
            .await
            .context("Failed to send Twilio request")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let error: Result<TwilioErrorResponse, _> = resp.json().await;
            let msg = error
                .map(|e| e.message)
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Twilio API error {}: {}", status, msg);
        }

        let response: TwilioMessageResponse = resp
            .json()
            .await
            .context("Failed to parse Twilio response")?;

        Ok(response.sid)
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

    #[test]
    #[serial]
    fn test_twilio_client_is_configured() {
        let orig_sid = std::env::var("TWILIO_ACCOUNT_SID").ok();
        let orig_token = std::env::var("TWILIO_AUTH_TOKEN").ok();
        let orig_phone = std::env::var("TWILIO_PHONE_NUMBER").ok();

        // Not configured
        std::env::remove_var("TWILIO_ACCOUNT_SID");
        std::env::remove_var("TWILIO_AUTH_TOKEN");
        std::env::remove_var("TWILIO_PHONE_NUMBER");
        assert!(!TwilioClient::is_configured());

        // Partially configured
        std::env::set_var("TWILIO_ACCOUNT_SID", "ACtest");
        assert!(!TwilioClient::is_configured());

        // Fully configured
        std::env::set_var("TWILIO_AUTH_TOKEN", "token");
        std::env::set_var("TWILIO_PHONE_NUMBER", "+15551234567");
        assert!(TwilioClient::is_configured());

        // Restore
        if let Some(v) = orig_sid {
            std::env::set_var("TWILIO_ACCOUNT_SID", v);
        } else {
            std::env::remove_var("TWILIO_ACCOUNT_SID");
        }
        if let Some(v) = orig_token {
            std::env::set_var("TWILIO_AUTH_TOKEN", v);
        } else {
            std::env::remove_var("TWILIO_AUTH_TOKEN");
        }
        if let Some(v) = orig_phone {
            std::env::set_var("TWILIO_PHONE_NUMBER", v);
        } else {
            std::env::remove_var("TWILIO_PHONE_NUMBER");
        }
    }

    #[test]
    #[serial]
    fn test_twilio_client_from_env() {
        let orig_sid = std::env::var("TWILIO_ACCOUNT_SID").ok();
        let orig_token = std::env::var("TWILIO_AUTH_TOKEN").ok();
        let orig_phone = std::env::var("TWILIO_PHONE_NUMBER").ok();

        // Missing vars
        std::env::remove_var("TWILIO_ACCOUNT_SID");
        std::env::remove_var("TWILIO_AUTH_TOKEN");
        std::env::remove_var("TWILIO_PHONE_NUMBER");
        assert!(TwilioClient::from_env().is_err());

        // All set
        std::env::set_var("TWILIO_ACCOUNT_SID", "ACtest123");
        std::env::set_var("TWILIO_AUTH_TOKEN", "secret_token");
        std::env::set_var("TWILIO_PHONE_NUMBER", "+15559876543");
        let client = TwilioClient::from_env().unwrap();
        assert_eq!(client.account_sid, "ACtest123");
        assert_eq!(client.from_number, "+15559876543");

        // Restore
        if let Some(v) = orig_sid {
            std::env::set_var("TWILIO_ACCOUNT_SID", v);
        } else {
            std::env::remove_var("TWILIO_ACCOUNT_SID");
        }
        if let Some(v) = orig_token {
            std::env::set_var("TWILIO_AUTH_TOKEN", v);
        } else {
            std::env::remove_var("TWILIO_AUTH_TOKEN");
        }
        if let Some(v) = orig_phone {
            std::env::set_var("TWILIO_PHONE_NUMBER", v);
        } else {
            std::env::remove_var("TWILIO_PHONE_NUMBER");
        }
    }

    #[test]
    fn test_format_sms_notification() {
        let msg = format_sms_notification("Customer needs help");
        assert!(msg.contains("Customer needs help"));
        assert!(msg.contains("Support alert"));
        assert!(!msg.contains("contract"));
        assert!(msg.len() < 160); // SMS limit
    }
}
