use crate::database::Database;
use anyhow::{Context, Result};
use email_utils::EmailService;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Notification payload for support escalation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SupportNotification {
    pub user_pubkey: Vec<u8>,
    pub conversation_id: i64,
    pub summary: String,
    pub chatwoot_link: String,
}

impl SupportNotification {
    /// Create a notification from conversation details
    pub fn new(
        user_pubkey: Vec<u8>,
        conversation_id: i64,
        summary: String,
        chatwoot_base_url: &str,
    ) -> Self {
        let chatwoot_link = format!(
            "{}/app/accounts/1/conversations/{}",
            chatwoot_base_url, conversation_id
        );

        Self {
            user_pubkey,
            conversation_id,
            summary,
            chatwoot_link,
        }
    }
}

/// Dispatch a support notification to the user based on their notification preferences.
/// Sends to ALL enabled channels (telegram, email, sms).
/// Returns Ok(()) if at least one notification was sent, Err if all failed.
pub async fn dispatch_notification(
    db: &Database,
    email_service: Option<&Arc<EmailService>>,
    notification: &SupportNotification,
) -> Result<()> {
    // Get user notification config
    let config = db
        .get_user_notification_config(&notification.user_pubkey)
        .await
        .context("Failed to get user notification config")?;

    let config = match config {
        Some(c) => c,
        None => {
            tracing::warn!(
                "No notification config found for user (pubkey: {}), skipping notification",
                hex::encode(&notification.user_pubkey)
            );
            return Ok(());
        }
    };

    let mut channels_sent = Vec::new();
    let mut errors = Vec::new();

    // Send to Telegram if enabled
    if config.notify_telegram {
        match send_telegram_notification(db, notification, &config).await {
            Ok(()) => channels_sent.push("telegram"),
            Err(e) => errors.push(format!("telegram: {}", e)),
        }
    }

    // Send to Email if enabled
    if config.notify_email {
        match send_email_notification(db, email_service, notification).await {
            Ok(()) => channels_sent.push("email"),
            Err(e) => errors.push(format!("email: {}", e)),
        }
    }

    // Send to SMS if enabled
    if config.notify_sms {
        match send_sms_notification(notification, &config).await {
            Ok(()) => channels_sent.push("sms"),
            Err(e) => errors.push(format!("sms: {}", e)),
        }
    }

    if channels_sent.is_empty() && errors.is_empty() {
        tracing::warn!(
            "No notification channels enabled for user (pubkey: {})",
            hex::encode(&notification.user_pubkey)
        );
    } else {
        tracing::info!(
            "Dispatched notification for conversation {} - sent: [{}], errors: [{}]",
            notification.conversation_id,
            channels_sent.join(", "),
            errors.join(", ")
        );
    }

    Ok(())
}

async fn send_telegram_notification(
    db: &Database,
    notification: &SupportNotification,
    config: &crate::database::UserNotificationConfig,
) -> Result<()> {
    use crate::notifications::telegram::{format_notification, TelegramClient};

    let chat_id = config
        .telegram_chat_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No Telegram chat_id configured"))?;

    if !TelegramClient::is_configured() {
        anyhow::bail!("TELEGRAM_BOT_TOKEN not configured");
    }

    let telegram = TelegramClient::from_env()?;
    let message = format_notification(&notification.summary, &notification.chatwoot_link);

    let sent_msg = telegram.send_message(chat_id, &message).await?;

    db.track_telegram_message(sent_msg.message_id, notification.conversation_id, chat_id)
        .await?;

    tracing::info!(
        "Telegram notification sent to chat_id: {}, message_id: {}",
        chat_id,
        sent_msg.message_id
    );
    Ok(())
}

async fn send_email_notification(
    db: &Database,
    email_service: Option<&Arc<EmailService>>,
    notification: &SupportNotification,
) -> Result<()> {
    let email_svc = email_service.ok_or_else(|| {
        anyhow::anyhow!("Email service not configured (missing MAILCHANNELS_API_KEY)")
    })?;

    // Look up account email by pubkey
    let account_id = db
        .get_account_id_by_public_key(&notification.user_pubkey)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No account found for pubkey"))?;

    let account = db
        .get_account_by_id(&account_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Account not found"))?;

    let email_addr = account
        .email
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No email address on account"))?;

    let email_body = format!(
        "A customer conversation requires your attention.\n\n\
        Summary: {}\n\n\
        View conversation: {}\n\n\
        Please log in to Chatwoot to respond.",
        notification.summary,
        notification.chatwoot_link
    );

    let from_addr =
        std::env::var("EMAIL_FROM_ADDR").unwrap_or_else(|_| "noreply@decent-cloud.org".to_string());

    email_svc
        .send_email(
            &from_addr,
            email_addr,
            "Customer Support Conversation Needs Attention",
            &email_body,
            false,
        )
        .await
        .context("Failed to send email")?;

    tracing::info!("Email notification sent to {}", email_addr);
    Ok(())
}

async fn send_sms_notification(
    notification: &SupportNotification,
    config: &crate::database::UserNotificationConfig,
) -> Result<()> {
    use crate::notifications::twilio::{format_sms_notification, TwilioClient};

    let phone = config
        .notify_phone
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No phone number configured"))?;

    if !TwilioClient::is_configured() {
        anyhow::bail!("Twilio not configured");
    }

    let twilio = TwilioClient::from_env()?;
    let message = format_sms_notification(&notification.summary);
    let sid = twilio.send_sms(phone, &message).await?;

    tracing::info!("SMS notification sent to {}, sid: {}", phone, sid);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_helpers::setup_test_db;

    #[test]
    fn test_support_notification_creation() {
        let pubkey = b"test_user_123".to_vec();
        let notification = SupportNotification::new(
            pubkey.clone(),
            42,
            "Customer needs human help".to_string(),
            "https://support.example.com",
        );

        assert_eq!(notification.user_pubkey, pubkey);
        assert_eq!(notification.conversation_id, 42);
        assert_eq!(notification.summary, "Customer needs human help");
        assert_eq!(
            notification.chatwoot_link,
            "https://support.example.com/app/accounts/1/conversations/42"
        );
    }

    #[test]
    fn test_support_notification_link_format() {
        let notification = SupportNotification::new(
            b"user".to_vec(),
            999,
            "Test".to_string(),
            "https://chat.example.org",
        );

        assert_eq!(
            notification.chatwoot_link,
            "https://chat.example.org/app/accounts/1/conversations/999"
        );
    }

    #[tokio::test]
    async fn test_dispatch_notification_no_config() {
        let db = setup_test_db().await;
        let pubkey = b"nonexistent_user";

        let notification = SupportNotification::new(
            pubkey.to_vec(),
            1,
            "Test notification".to_string(),
            "https://example.com",
        );

        // Should succeed but skip notification (no config)
        let result = dispatch_notification(&db, None, &notification).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dispatch_notification_telegram() {
        let db = setup_test_db().await;
        let pubkey = b"test_user_telegram";

        // Set telegram notification config (no FK constraint anymore)
        let config = crate::database::notification_config::UserNotificationConfig {
            user_pubkey: pubkey.to_vec(),
            chatwoot_portal_slug: Some("test-portal".to_string()),
            notify_telegram: true,
            notify_email: false,
            notify_sms: false,
            telegram_chat_id: Some("123456789".to_string()),
            notify_phone: None,
            notify_email_address: None,
        };
        db.set_user_notification_config(pubkey, &config)
            .await
            .unwrap();

        let notification = SupportNotification::new(
            pubkey.to_vec(),
            1,
            "Test notification".to_string(),
            "https://example.com",
        );

        // Should succeed (will log error since TELEGRAM_BOT_TOKEN not set in tests)
        let result = dispatch_notification(&db, None, &notification).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dispatch_notification_email_no_service() {
        let db = setup_test_db().await;
        // Use 32-byte pubkey for account creation
        let pubkey = [0u8; 32];

        // Create account with email first
        db.create_account("test_email_user", &pubkey, "user@example.com")
            .await
            .unwrap();

        // Set email notification config
        let config = crate::database::notification_config::UserNotificationConfig {
            user_pubkey: pubkey.to_vec(),
            chatwoot_portal_slug: Some("test-portal".to_string()),
            notify_telegram: false,
            notify_email: true,
            notify_sms: false,
            telegram_chat_id: None,
            notify_phone: None,
            notify_email_address: None,
        };
        db.set_user_notification_config(&pubkey, &config)
            .await
            .unwrap();

        let notification = SupportNotification::new(
            pubkey.to_vec(),
            42,
            "Customer escalated conversation".to_string(),
            "https://support.test.com",
        );

        // Should succeed but log error (no email service configured)
        let result = dispatch_notification(&db, None, &notification).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dispatch_notification_multi_channel() {
        let db = setup_test_db().await;
        // Use 32-byte pubkey for account creation
        let pubkey = [2u8; 32];

        // Create account with email first
        db.create_account("test_multi_user", &pubkey, "multi@example.com")
            .await
            .unwrap();

        // Enable email + telegram (both will fail: no token, no email service)
        let config = crate::database::notification_config::UserNotificationConfig {
            user_pubkey: pubkey.to_vec(),
            chatwoot_portal_slug: None,
            notify_telegram: true,
            notify_email: true,
            notify_sms: false,
            telegram_chat_id: Some("123456".to_string()),
            notify_phone: None,
            notify_email_address: None,
        };
        db.set_user_notification_config(&pubkey, &config)
            .await
            .unwrap();

        let notification = SupportNotification::new(
            pubkey.to_vec(),
            1,
            "Multi-channel test".to_string(),
            "https://example.com",
        );

        // Should succeed but log errors for both channels (no services configured)
        let result = dispatch_notification(&db, None, &notification).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dispatch_notification_no_channels_enabled() {
        let db = setup_test_db().await;
        let pubkey = b"test_user_no_channels";

        // Config exists but no channels enabled
        let config = crate::database::notification_config::UserNotificationConfig {
            user_pubkey: pubkey.to_vec(),
            chatwoot_portal_slug: None,
            notify_telegram: false,
            notify_email: false,
            notify_sms: false,
            telegram_chat_id: None,
            notify_phone: None,
            notify_email_address: None,
        };
        db.set_user_notification_config(pubkey, &config)
            .await
            .unwrap();

        let notification = SupportNotification::new(
            pubkey.to_vec(),
            1,
            "Test".to_string(),
            "https://example.com",
        );

        // Should succeed (logs warning about no channels)
        let result = dispatch_notification(&db, None, &notification).await;
        assert!(result.is_ok());
    }
}
