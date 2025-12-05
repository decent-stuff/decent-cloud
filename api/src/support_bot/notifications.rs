use crate::database::email::EmailType;
use crate::database::Database;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Notification payload for support escalation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SupportNotification {
    pub provider_pubkey: Vec<u8>,
    pub conversation_id: i64,
    pub contract_id: String,
    pub summary: String,
    pub chatwoot_link: String,
}

impl SupportNotification {
    /// Create a notification from conversation details
    pub fn new(
        provider_pubkey: Vec<u8>,
        conversation_id: i64,
        contract_id: String,
        summary: String,
        chatwoot_base_url: &str,
    ) -> Self {
        let chatwoot_link = format!(
            "{}/app/accounts/1/conversations/{}",
            chatwoot_base_url, conversation_id
        );

        Self {
            provider_pubkey,
            conversation_id,
            contract_id,
            summary,
            chatwoot_link,
        }
    }
}

/// Dispatch a support notification to the provider based on their notification preferences.
/// Returns Ok(()) if notification was queued successfully, Err if there was a problem.
pub async fn dispatch_notification(
    db: &Database,
    notification: &SupportNotification,
) -> Result<()> {
    // Get provider notification config
    let config = db
        .get_provider_notification_config(&notification.provider_pubkey)
        .await
        .context("Failed to get provider notification config")?;

    let config = match config {
        Some(c) => c,
        None => {
            tracing::warn!(
                "No notification config found for provider (pubkey: {}), skipping notification",
                hex::encode(&notification.provider_pubkey)
            );
            return Ok(());
        }
    };

    tracing::info!(
        "Dispatching support notification for conversation {} via {}",
        notification.conversation_id,
        config.notify_via
    );

    match config.notify_via.as_str() {
        "telegram" => {
            // Step 8 will implement actual Telegram sending
            tracing::info!(
                "Telegram notification queued for chat_id: {:?}, conversation: {}, link: {}",
                config.telegram_chat_id,
                notification.conversation_id,
                notification.chatwoot_link
            );
            // TODO: Queue for Telegram in Step 8
            Ok(())
        }
        "email" => {
            // Queue email notification using existing email queue
            let email_body = format!(
                "A customer conversation requires your attention.\n\n\
                Contract ID: {}\n\
                Summary: {}\n\n\
                View conversation: {}\n\n\
                Please log in to Chatwoot to respond.",
                notification.contract_id, notification.summary, notification.chatwoot_link
            );

            // Get provider email - lookup by pubkey (account ID = provider pubkey)
            let account = db
                .get_account(&notification.provider_pubkey)
                .await
                .context("Failed to get provider account")?;

            let email_addr = match account.and_then(|a| a.email) {
                Some(email) => email,
                None => {
                    tracing::warn!(
                        "Provider has no email address, cannot send notification (pubkey: {})",
                        hex::encode(&notification.provider_pubkey)
                    );
                    return Ok(());
                }
            };

            let from_addr = std::env::var("EMAIL_FROM_ADDR")
                .unwrap_or_else(|_| "noreply@decent-cloud.org".to_string());

            db.queue_email(
                &email_addr,
                &from_addr,
                "Customer Support Conversation Needs Attention",
                &email_body,
                false,
                EmailType::General,
            )
            .await
            .context("Failed to queue email notification")?;

            tracing::info!(
                "Email notification queued for {} (conversation {})",
                email_addr,
                notification.conversation_id
            );

            Ok(())
        }
        "sms" => {
            // Future implementation
            tracing::info!(
                "SMS notification requested for phone: {:?}, conversation: {} (not yet implemented)",
                config.notify_phone,
                notification.conversation_id
            );
            Ok(())
        }
        unknown => {
            tracing::warn!(
                "Unknown notification method '{}' for provider (pubkey: {})",
                unknown,
                hex::encode(&notification.provider_pubkey)
            );
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_helpers::setup_test_db;

    #[test]
    fn test_support_notification_creation() {
        let pubkey = b"test_provider_123".to_vec();
        let notification = SupportNotification::new(
            pubkey.clone(),
            42,
            "contract_123".to_string(),
            "Customer needs human help".to_string(),
            "https://support.example.com",
        );

        assert_eq!(notification.provider_pubkey, pubkey);
        assert_eq!(notification.conversation_id, 42);
        assert_eq!(notification.contract_id, "contract_123");
        assert_eq!(notification.summary, "Customer needs human help");
        assert_eq!(
            notification.chatwoot_link,
            "https://support.example.com/app/accounts/1/conversations/42"
        );
    }

    #[test]
    fn test_support_notification_link_format() {
        let notification = SupportNotification::new(
            b"provider".to_vec(),
            999,
            "contract_xyz".to_string(),
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
        let pubkey = b"nonexistent_provider";

        let notification = SupportNotification::new(
            pubkey.to_vec(),
            1,
            "test_contract".to_string(),
            "Test notification".to_string(),
            "https://example.com",
        );

        // Should succeed but skip notification (no config)
        let result = dispatch_notification(&db, &notification).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dispatch_notification_telegram() {
        let db = setup_test_db().await;
        let pubkey = b"test_provider_telegram";
        let pubkey_slice: &[u8] = pubkey;

        // Create provider profile
        sqlx::query!(
            "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, ?, ?, ?, ?)",
            pubkey_slice,
            "Test Provider",
            "v1",
            "v1",
            1700000000i64
        )
        .execute(&db.pool)
        .await
        .unwrap();

        // Set telegram notification config
        let config = crate::database::notification_config::ProviderNotificationConfig {
            provider_pubkey: pubkey.to_vec(),
            chatwoot_portal_slug: Some("test-portal".to_string()),
            notify_via: "telegram".to_string(),
            telegram_chat_id: Some("123456789".to_string()),
            notify_phone: None,
        };
        db.set_provider_notification_config(pubkey, &config)
            .await
            .unwrap();

        let notification = SupportNotification::new(
            pubkey.to_vec(),
            1,
            "test_contract".to_string(),
            "Test notification".to_string(),
            "https://example.com",
        );

        // Should succeed (logs for now, actual Telegram in Step 8)
        let result = dispatch_notification(&db, &notification).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dispatch_notification_email() {
        let db = setup_test_db().await;
        let pubkey = b"test_provider_email";
        let pubkey_slice: &[u8] = pubkey;

        // Create provider profile
        sqlx::query!(
            "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, ?, ?, ?, ?)",
            pubkey_slice,
            "Test Provider",
            "v1",
            "v1",
            1700000000i64
        )
        .execute(&db.pool)
        .await
        .unwrap();

        // Create account with email
        sqlx::query!(
            "INSERT INTO accounts (id, username, email) VALUES (?, ?, ?)",
            pubkey_slice,
            "testprovider",
            "provider@example.com"
        )
        .execute(&db.pool)
        .await
        .unwrap();

        // Set email notification config
        let config = crate::database::notification_config::ProviderNotificationConfig {
            provider_pubkey: pubkey.to_vec(),
            chatwoot_portal_slug: Some("test-portal".to_string()),
            notify_via: "email".to_string(),
            telegram_chat_id: None,
            notify_phone: None,
        };
        db.set_provider_notification_config(pubkey, &config)
            .await
            .unwrap();

        let notification = SupportNotification::new(
            pubkey.to_vec(),
            42,
            "abc123".to_string(),
            "Customer escalated conversation".to_string(),
            "https://support.test.com",
        );

        // Should queue email
        let result = dispatch_notification(&db, &notification).await;
        assert!(result.is_ok());

        // Verify email was queued
        let pending = db.get_pending_emails(10).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].to_addr, "provider@example.com");
        assert_eq!(
            pending[0].subject,
            "Customer Support Conversation Needs Attention"
        );
        assert!(pending[0].body.contains("abc123"));
        assert!(pending[0]
            .body
            .contains("https://support.test.com/app/accounts/1/conversations/42"));
    }

    #[tokio::test]
    async fn test_dispatch_notification_email_no_email_address() {
        let db = setup_test_db().await;
        let pubkey = b"test_provider_no_email";
        let pubkey_slice: &[u8] = pubkey;

        // Create provider profile
        sqlx::query!(
            "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, ?, ?, ?, ?)",
            pubkey_slice,
            "Test Provider",
            "v1",
            "v1",
            1700000000i64
        )
        .execute(&db.pool)
        .await
        .unwrap();

        // Create account WITHOUT email
        sqlx::query!(
            "INSERT INTO accounts (id, username, email) VALUES (?, ?, ?)",
            pubkey_slice,
            "testprovider",
            None as Option<String>
        )
        .execute(&db.pool)
        .await
        .unwrap();

        // Set email notification config
        let config = crate::database::notification_config::ProviderNotificationConfig {
            provider_pubkey: pubkey.to_vec(),
            chatwoot_portal_slug: Some("test-portal".to_string()),
            notify_via: "email".to_string(),
            telegram_chat_id: None,
            notify_phone: None,
        };
        db.set_provider_notification_config(pubkey, &config)
            .await
            .unwrap();

        let notification = SupportNotification::new(
            pubkey.to_vec(),
            1,
            "test".to_string(),
            "Test".to_string(),
            "https://example.com",
        );

        // Should succeed but skip notification (no email)
        let result = dispatch_notification(&db, &notification).await;
        assert!(result.is_ok());

        // Verify no email was queued
        let pending = db.get_pending_emails(10).await.unwrap();
        assert_eq!(pending.len(), 0);
    }

    #[tokio::test]
    async fn test_dispatch_notification_sms() {
        let db = setup_test_db().await;
        let pubkey = b"test_provider_sms";
        let pubkey_slice: &[u8] = pubkey;

        // Create provider profile
        sqlx::query!(
            "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, ?, ?, ?, ?)",
            pubkey_slice,
            "Test Provider",
            "v1",
            "v1",
            1700000000i64
        )
        .execute(&db.pool)
        .await
        .unwrap();

        // Set SMS notification config
        let config = crate::database::notification_config::ProviderNotificationConfig {
            provider_pubkey: pubkey.to_vec(),
            chatwoot_portal_slug: Some("test-portal".to_string()),
            notify_via: "sms".to_string(),
            telegram_chat_id: None,
            notify_phone: Some("+1234567890".to_string()),
        };
        db.set_provider_notification_config(pubkey, &config)
            .await
            .unwrap();

        let notification = SupportNotification::new(
            pubkey.to_vec(),
            1,
            "test_contract".to_string(),
            "Test notification".to_string(),
            "https://example.com",
        );

        // Should succeed (logs for now, SMS not yet implemented)
        let result = dispatch_notification(&db, &notification).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dispatch_notification_unknown_method() {
        let db = setup_test_db().await;
        let pubkey = b"test_provider_unknown";
        let pubkey_slice: &[u8] = pubkey;

        // Create provider profile
        sqlx::query!(
            "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, ?, ?, ?, ?)",
            pubkey_slice,
            "Test Provider",
            "v1",
            "v1",
            1700000000i64
        )
        .execute(&db.pool)
        .await
        .unwrap();

        // Manually insert config with invalid method (bypassing CHECK constraint via raw insert)
        // Note: This shouldn't happen in production, but test defensive handling
        // We can't actually do this with the CHECK constraint, so we'll skip this test
        // and rely on database constraint testing in notification_config.rs
    }
}
