//! Database functions for Telegram message tracking.
//! Maps Telegram message IDs to Chatwoot conversation IDs for reply handling.

use super::types::Database;
use anyhow::Result;

impl Database {
    /// Store a Telegram message → conversation mapping.
    pub async fn track_telegram_message(
        &self,
        telegram_message_id: i64,
        conversation_id: i64,
        provider_chat_id: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        sqlx::query!(
            r#"INSERT INTO telegram_message_tracking
               (telegram_message_id, conversation_id, provider_chat_id, created_at)
               VALUES ($1, $2, $3, $4)
               ON CONFLICT(telegram_message_id) DO UPDATE SET
                   conversation_id = excluded.conversation_id,
                   provider_chat_id = excluded.provider_chat_id,
                   created_at = excluded.created_at"#,
            telegram_message_id,
            conversation_id,
            provider_chat_id,
            now
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Look up conversation ID by Telegram message ID.
    pub async fn lookup_telegram_conversation(
        &self,
        telegram_message_id: i64,
    ) -> Result<Option<i64>> {
        let result = sqlx::query_scalar!(
            r#"SELECT conversation_id FROM telegram_message_tracking
               WHERE telegram_message_id = $1"#,
            telegram_message_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use crate::database::test_helpers::setup_test_db;

    #[tokio::test]
    async fn test_track_and_lookup_telegram_message() {
        let db = setup_test_db().await;

        // Track a message
        db.track_telegram_message(12345, 100, "chat_123")
            .await
            .unwrap();

        // Lookup should find it
        let conv = db.lookup_telegram_conversation(12345).await.unwrap();
        assert_eq!(conv, Some(100));

        // Unknown message returns None
        let unknown = db.lookup_telegram_conversation(99999).await.unwrap();
        assert_eq!(unknown, None);
    }

    #[tokio::test]
    async fn test_track_telegram_message_upsert() {
        let db = setup_test_db().await;

        // Track initial message
        db.track_telegram_message(111, 200, "chat_a").await.unwrap();
        assert_eq!(
            db.lookup_telegram_conversation(111).await.unwrap(),
            Some(200)
        );

        // Update same message to different conversation
        db.track_telegram_message(111, 300, "chat_b").await.unwrap();
        assert_eq!(
            db.lookup_telegram_conversation(111).await.unwrap(),
            Some(300)
        );
    }
}
