//! Test notification functionality for verifying channel configuration.

use crate::database::email::EmailType;
use crate::database::Database;
use crate::notifications::telegram::TelegramClient;
use crate::notifications::twilio::TwilioClient;
use anyhow::{bail, Context, Result};

/// Send a test notification to a specific channel.
pub async fn send_test_notification(db: &Database, pubkey: &[u8], channel: &str) -> Result<String> {
    let config = db
        .get_user_notification_config(pubkey)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No notification config found"))?;

    match channel {
        "telegram" => send_test_telegram(&config.telegram_chat_id).await,
        "email" => send_test_email(db, pubkey).await,
        "sms" => send_test_sms(&config.notify_phone).await,
        _ => bail!("Invalid channel: {}. Use telegram, email, or sms", channel),
    }
}

async fn send_test_telegram(chat_id: &Option<String>) -> Result<String> {
    let chat_id = chat_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No Telegram chat ID configured"))?;

    if !TelegramClient::is_configured() {
        bail!("Telegram not configured on server (TELEGRAM_BOT_TOKEN missing)");
    }

    let telegram = TelegramClient::from_env()?;
    let msg = telegram
        .send_message(chat_id, "This is a test notification from DecentCloud.")
        .await
        .context("Failed to send Telegram message")?;

    Ok(format!(
        "Telegram test sent (message_id: {})",
        msg.message_id
    ))
}

async fn send_test_email(db: &Database, pubkey: &[u8]) -> Result<String> {
    let account_id = db
        .get_account_id_by_public_key(pubkey)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No account found for this key"))?;

    let account = db
        .get_account_by_id(&account_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Account not found"))?;

    let email = account
        .email
        .ok_or_else(|| anyhow::anyhow!("No email address on account"))?;

    let from =
        std::env::var("EMAIL_FROM_ADDR").unwrap_or_else(|_| "noreply@decent-cloud.org".into());

    db.queue_email(
        &email,
        &from,
        "DecentCloud Test Notification",
        "This is a test notification from DecentCloud.\n\nIf you received this, your email notifications are working correctly.",
        false,
        EmailType::General,
    )
    .await?;

    Ok(format!("Email test queued for {}", email))
}

async fn send_test_sms(phone: &Option<String>) -> Result<String> {
    let phone = phone
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No phone number configured"))?;

    if !TwilioClient::is_configured() {
        bail!("SMS not configured on server (Twilio credentials missing)");
    }

    let twilio = TwilioClient::from_env()?;
    let sid = twilio
        .send_sms(phone, "This is a test notification from DecentCloud.")
        .await
        .context("Failed to send SMS")?;

    Ok(format!("SMS test sent to {} (sid: {})", phone, sid))
}

/// Send a test escalation notification to all enabled channels.
/// Returns detailed results for each channel.
pub async fn send_test_escalation(db: &Database, pubkey: &[u8]) -> Result<String> {
    let config = db
        .get_user_notification_config(pubkey)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No notification config found"))?;

    let mut results = Vec::new();

    if config.notify_telegram {
        match send_test_telegram(&config.telegram_chat_id).await {
            Ok(msg) => results.push(format!("Telegram: {}", msg)),
            Err(e) => results.push(format!("Telegram: FAILED - {}", e)),
        }
    }

    if config.notify_email {
        match send_test_email(db, pubkey).await {
            Ok(msg) => results.push(format!("Email: {}", msg)),
            Err(e) => results.push(format!("Email: FAILED - {}", e)),
        }
    }

    if config.notify_sms {
        match send_test_sms(&config.notify_phone).await {
            Ok(msg) => results.push(format!("SMS: {}", msg)),
            Err(e) => results.push(format!("SMS: FAILED - {}", e)),
        }
    }

    if results.is_empty() {
        bail!("No notification channels enabled");
    }

    Ok(results.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_helpers::setup_test_db;

    #[tokio::test]
    async fn test_send_test_notification_no_config() {
        let db = setup_test_db().await;
        let result = send_test_notification(&db, b"nonexistent", "telegram").await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No notification config"));
    }

    #[tokio::test]
    async fn test_send_test_notification_invalid_channel() {
        let db = setup_test_db().await;
        let pubkey = b"test_user";
        let config = crate::database::notification_config::UserNotificationConfig {
            user_pubkey: pubkey.to_vec(),
            chatwoot_portal_slug: None,
            notify_telegram: true,
            notify_email: false,
            notify_sms: false,
            telegram_chat_id: Some("123".into()),
            notify_phone: None,
            notify_email_address: None,
        };
        db.set_user_notification_config(pubkey, &config)
            .await
            .unwrap();

        let result = send_test_notification(&db, pubkey, "invalid").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid channel"));
    }

    #[tokio::test]
    async fn test_send_test_telegram_no_chat_id() {
        let result = send_test_telegram(&None).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No Telegram chat ID"));
    }

    #[tokio::test]
    async fn test_send_test_sms_no_phone() {
        let result = send_test_sms(&None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No phone number"));
    }
}
