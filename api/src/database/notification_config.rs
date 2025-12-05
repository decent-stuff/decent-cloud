use super::types::Database;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserNotificationConfig {
    pub user_pubkey: Vec<u8>,
    pub chatwoot_portal_slug: Option<String>,
    pub notify_telegram: bool,
    pub notify_email: bool,
    pub notify_sms: bool,
    pub telegram_chat_id: Option<String>,
    pub notify_phone: Option<String>,
    pub notify_email_address: Option<String>,
}

impl Database {
    /// Get user notification configuration by pubkey.
    pub async fn get_user_notification_config(
        &self,
        pubkey: &[u8],
    ) -> Result<Option<UserNotificationConfig>> {
        let config = sqlx::query_as!(
            UserNotificationConfig,
            r#"SELECT user_pubkey as "user_pubkey!", chatwoot_portal_slug,
                      notify_telegram as "notify_telegram!: bool",
                      notify_email as "notify_email!: bool",
                      notify_sms as "notify_sms!: bool",
                      telegram_chat_id, notify_phone, notify_email_address
               FROM user_notification_config
               WHERE user_pubkey = ?"#,
            pubkey
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(config)
    }

    /// Set user notification configuration. Creates new entry or updates existing one.
    pub async fn set_user_notification_config(
        &self,
        pubkey: &[u8],
        config: &UserNotificationConfig,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        sqlx::query!(
            r#"INSERT INTO user_notification_config
               (user_pubkey, chatwoot_portal_slug, notify_telegram, notify_email, notify_sms,
                telegram_chat_id, notify_phone, notify_email_address, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(user_pubkey) DO UPDATE SET
                   chatwoot_portal_slug = excluded.chatwoot_portal_slug,
                   notify_telegram = excluded.notify_telegram,
                   notify_email = excluded.notify_email,
                   notify_sms = excluded.notify_sms,
                   telegram_chat_id = excluded.telegram_chat_id,
                   notify_phone = excluded.notify_phone,
                   notify_email_address = excluded.notify_email_address,
                   updated_at = excluded.updated_at"#,
            pubkey,
            config.chatwoot_portal_slug,
            config.notify_telegram,
            config.notify_email,
            config.notify_sms,
            config.telegram_chat_id,
            config.notify_phone,
            config.notify_email_address,
            now,
            now
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Increment notification usage count for user/channel today.
    pub async fn increment_notification_usage(&self, user_id: &str, channel: &str) -> Result<i64> {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        sqlx::query!(
            r#"INSERT INTO notification_usage (provider_id, channel, date, count)
               VALUES (?, ?, ?, 1)
               ON CONFLICT(provider_id, channel, date) DO UPDATE SET count = count + 1"#,
            user_id,
            channel,
            today
        )
        .execute(&self.pool)
        .await?;

        let row = sqlx::query!(
            r#"SELECT count as "count!" FROM notification_usage
               WHERE provider_id = ? AND channel = ? AND date = ?"#,
            user_id,
            channel,
            today
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.count as i64)
    }

    /// Get notification usage count for user/channel today.
    pub async fn get_notification_usage(&self, user_id: &str, channel: &str) -> Result<i64> {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        let row = sqlx::query!(
            r#"SELECT count as "count!" FROM notification_usage
               WHERE provider_id = ? AND channel = ? AND date = ?"#,
            user_id,
            channel,
            today
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.count as i64).unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use crate::database::test_helpers::setup_test_db;

    #[tokio::test]
    async fn test_get_user_notification_config_not_exists() {
        let db = setup_test_db().await;
        let pubkey = b"nonexistent_user";

        let result = db.get_user_notification_config(pubkey).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_set_and_get_user_notification_config() {
        let db = setup_test_db().await;
        let pubkey = b"test_user_123";

        let config = super::UserNotificationConfig {
            user_pubkey: pubkey.to_vec(),
            chatwoot_portal_slug: Some("test-portal".to_string()),
            notify_telegram: true,
            notify_email: true,
            notify_sms: false,
            telegram_chat_id: Some("123456789".to_string()),
            notify_phone: None,
            notify_email_address: Some("test@example.com".to_string()),
        };

        // Set config
        let set_result = db.set_user_notification_config(pubkey, &config).await;
        assert!(set_result.is_ok());

        // Get config
        let retrieved = db
            .get_user_notification_config(pubkey)
            .await
            .unwrap()
            .expect("Config should exist");

        assert_eq!(retrieved.user_pubkey, pubkey);
        assert_eq!(
            retrieved.chatwoot_portal_slug,
            Some("test-portal".to_string())
        );
        assert!(retrieved.notify_telegram);
        assert!(retrieved.notify_email);
        assert!(!retrieved.notify_sms);
        assert_eq!(retrieved.telegram_chat_id, Some("123456789".to_string()));
        assert_eq!(retrieved.notify_phone, None);
        assert_eq!(
            retrieved.notify_email_address,
            Some("test@example.com".to_string())
        );
    }

    #[tokio::test]
    async fn test_update_existing_notification_config() {
        let db = setup_test_db().await;
        let pubkey = b"test_user_456";

        // Initial config - telegram only
        let initial_config = super::UserNotificationConfig {
            user_pubkey: pubkey.to_vec(),
            chatwoot_portal_slug: Some("initial-portal".to_string()),
            notify_telegram: true,
            notify_email: false,
            notify_sms: false,
            telegram_chat_id: Some("111111111".to_string()),
            notify_phone: None,
            notify_email_address: None,
        };

        db.set_user_notification_config(pubkey, &initial_config)
            .await
            .unwrap();

        // Update config - switch to SMS + email
        let updated_config = super::UserNotificationConfig {
            user_pubkey: pubkey.to_vec(),
            chatwoot_portal_slug: Some("updated-portal".to_string()),
            notify_telegram: false,
            notify_email: true,
            notify_sms: true,
            telegram_chat_id: None,
            notify_phone: Some("+1234567890".to_string()),
            notify_email_address: Some("updated@example.com".to_string()),
        };

        db.set_user_notification_config(pubkey, &updated_config)
            .await
            .unwrap();

        // Verify update
        let retrieved = db
            .get_user_notification_config(pubkey)
            .await
            .unwrap()
            .expect("Config should exist");

        assert_eq!(
            retrieved.chatwoot_portal_slug,
            Some("updated-portal".to_string())
        );
        assert!(!retrieved.notify_telegram);
        assert!(retrieved.notify_email);
        assert!(retrieved.notify_sms);
        assert_eq!(retrieved.telegram_chat_id, None);
        assert_eq!(retrieved.notify_phone, Some("+1234567890".to_string()));
        assert_eq!(
            retrieved.notify_email_address,
            Some("updated@example.com".to_string())
        );
    }

    #[tokio::test]
    async fn test_multi_channel_notifications() {
        let db = setup_test_db().await;
        let pubkey = b"test_user_multi";

        // Enable all channels
        let config = super::UserNotificationConfig {
            user_pubkey: pubkey.to_vec(),
            chatwoot_portal_slug: None,
            notify_telegram: true,
            notify_email: true,
            notify_sms: true,
            telegram_chat_id: Some("999888777".to_string()),
            notify_phone: Some("+1555123456".to_string()),
            notify_email_address: Some("multi@example.com".to_string()),
        };

        db.set_user_notification_config(pubkey, &config)
            .await
            .unwrap();

        let retrieved = db
            .get_user_notification_config(pubkey)
            .await
            .unwrap()
            .expect("Config should exist");

        assert!(retrieved.notify_telegram);
        assert!(retrieved.notify_email);
        assert!(retrieved.notify_sms);
    }
}
