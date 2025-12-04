use super::types::Database;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProviderNotificationConfig {
    pub provider_pubkey: Vec<u8>,
    pub chatwoot_portal_slug: Option<String>,
    pub notify_via: String,
    pub telegram_chat_id: Option<String>,
    pub notify_phone: Option<String>,
}

impl Database {
    /// Get provider notification configuration by pubkey.
    pub async fn get_provider_notification_config(
        &self,
        pubkey: &[u8],
    ) -> Result<Option<ProviderNotificationConfig>> {
        let config = sqlx::query_as!(
            ProviderNotificationConfig,
            r#"SELECT provider_pubkey as "provider_pubkey!", chatwoot_portal_slug, notify_via as "notify_via!", telegram_chat_id, notify_phone
               FROM provider_notification_config
               WHERE provider_pubkey = ?"#,
            pubkey
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(config)
    }

    /// Set provider notification configuration. Creates new entry or updates existing one.
    pub async fn set_provider_notification_config(
        &self,
        pubkey: &[u8],
        config: &ProviderNotificationConfig,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        sqlx::query!(
            r#"INSERT INTO provider_notification_config
               (provider_pubkey, chatwoot_portal_slug, notify_via, telegram_chat_id, notify_phone, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(provider_pubkey) DO UPDATE SET
                   chatwoot_portal_slug = excluded.chatwoot_portal_slug,
                   notify_via = excluded.notify_via,
                   telegram_chat_id = excluded.telegram_chat_id,
                   notify_phone = excluded.notify_phone,
                   updated_at = excluded.updated_at"#,
            pubkey,
            config.chatwoot_portal_slug,
            config.notify_via,
            config.telegram_chat_id,
            config.notify_phone,
            now,
            now
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::database::test_helpers::setup_test_db;

    #[tokio::test]
    async fn test_get_provider_notification_config_not_exists() {
        let db = setup_test_db().await;
        let pubkey = b"nonexistent_provider";

        let result = db.get_provider_notification_config(pubkey).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_set_and_get_provider_notification_config() {
        let db = setup_test_db().await;
        let pubkey = b"test_provider_123";
        let pubkey_slice: &[u8] = pubkey;

        // First, we need to create a provider profile
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

        let config = super::ProviderNotificationConfig {
            provider_pubkey: pubkey.to_vec(),
            chatwoot_portal_slug: Some("test-portal".to_string()),
            notify_via: "telegram".to_string(),
            telegram_chat_id: Some("123456789".to_string()),
            notify_phone: None,
        };

        // Set config
        let set_result = db.set_provider_notification_config(pubkey, &config).await;
        assert!(set_result.is_ok());

        // Get config
        let retrieved = db
            .get_provider_notification_config(pubkey)
            .await
            .unwrap()
            .expect("Config should exist");

        assert_eq!(retrieved.provider_pubkey, pubkey);
        assert_eq!(
            retrieved.chatwoot_portal_slug,
            Some("test-portal".to_string())
        );
        assert_eq!(retrieved.notify_via, "telegram");
        assert_eq!(retrieved.telegram_chat_id, Some("123456789".to_string()));
        assert_eq!(retrieved.notify_phone, None);
    }

    #[tokio::test]
    async fn test_update_existing_notification_config() {
        let db = setup_test_db().await;
        let pubkey = b"test_provider_456";
        let pubkey_slice: &[u8] = pubkey;

        // Create provider profile
        sqlx::query!(
            "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, ?, ?, ?, ?)",
            pubkey_slice,
            "Test Provider 2",
            "v1",
            "v1",
            1700000000i64
        )
        .execute(&db.pool)
        .await
        .unwrap();

        // Initial config
        let initial_config = super::ProviderNotificationConfig {
            provider_pubkey: pubkey.to_vec(),
            chatwoot_portal_slug: Some("initial-portal".to_string()),
            notify_via: "telegram".to_string(),
            telegram_chat_id: Some("111111111".to_string()),
            notify_phone: None,
        };

        db.set_provider_notification_config(pubkey, &initial_config)
            .await
            .unwrap();

        // Update config
        let updated_config = super::ProviderNotificationConfig {
            provider_pubkey: pubkey.to_vec(),
            chatwoot_portal_slug: Some("updated-portal".to_string()),
            notify_via: "sms".to_string(),
            telegram_chat_id: None,
            notify_phone: Some("+1234567890".to_string()),
        };

        db.set_provider_notification_config(pubkey, &updated_config)
            .await
            .unwrap();

        // Verify update
        let retrieved = db
            .get_provider_notification_config(pubkey)
            .await
            .unwrap()
            .expect("Config should exist");

        assert_eq!(
            retrieved.chatwoot_portal_slug,
            Some("updated-portal".to_string())
        );
        assert_eq!(retrieved.notify_via, "sms");
        assert_eq!(retrieved.telegram_chat_id, None);
        assert_eq!(retrieved.notify_phone, Some("+1234567890".to_string()));
    }

    #[tokio::test]
    async fn test_set_config_invalid_notify_via() {
        let db = setup_test_db().await;
        let pubkey = b"test_provider_789";
        let pubkey_slice: &[u8] = pubkey;

        // Create provider profile
        sqlx::query!(
            "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, ?, ?, ?, ?)",
            pubkey_slice,
            "Test Provider 3",
            "v1",
            "v1",
            1700000000i64
        )
        .execute(&db.pool)
        .await
        .unwrap();

        let invalid_config = super::ProviderNotificationConfig {
            provider_pubkey: pubkey.to_vec(),
            chatwoot_portal_slug: Some("test-portal".to_string()),
            notify_via: "invalid_method".to_string(),
            telegram_chat_id: None,
            notify_phone: None,
        };

        // Should fail due to CHECK constraint
        let result = db
            .set_provider_notification_config(pubkey, &invalid_config)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_config_nonexistent_provider() {
        let db = setup_test_db().await;
        let pubkey = b"nonexistent_provider_999";

        let config = super::ProviderNotificationConfig {
            provider_pubkey: pubkey.to_vec(),
            chatwoot_portal_slug: Some("test-portal".to_string()),
            notify_via: "telegram".to_string(),
            telegram_chat_id: Some("123456789".to_string()),
            notify_phone: None,
        };

        // Should fail due to foreign key constraint
        let result = db.set_provider_notification_config(pubkey, &config).await;
        assert!(result.is_err());
    }
}
