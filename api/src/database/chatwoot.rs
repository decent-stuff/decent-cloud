use super::types::Database;
use anyhow::Result;

impl Database {
    /// Insert a Chatwoot message event for response time tracking.
    pub async fn insert_chatwoot_message_event(
        &self,
        contract_id: &str,
        conversation_id: i64,
        message_id: i64,
        sender_type: &str,
        created_at: i64,
    ) -> Result<()> {
        sqlx::query!(
            r#"INSERT INTO chatwoot_message_events
               (contract_id, chatwoot_conversation_id, chatwoot_message_id, sender_type, created_at)
               VALUES (?, ?, ?, ?, ?)"#,
            contract_id,
            conversation_id,
            message_id,
            sender_type,
            created_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get average response time for a provider (in seconds).
    pub async fn get_provider_avg_response_time(
        &self,
        provider_pubkey: &[u8],
    ) -> Result<Option<f64>> {
        // This query calculates the average time between customer messages and the first provider response
        let result = sqlx::query_scalar!(
            r#"
            SELECT AVG(response_time) as "avg_response_time: f64"
            FROM (
                SELECT
                    MIN(response.created_at - customer_msg.created_at) as response_time
                FROM chatwoot_message_events customer_msg
                JOIN contract_sign_requests c ON hex(c.contract_id) = customer_msg.contract_id
                JOIN chatwoot_message_events response ON
                    response.chatwoot_conversation_id = customer_msg.chatwoot_conversation_id
                    AND response.sender_type = 'provider'
                    AND response.created_at > customer_msg.created_at
                WHERE customer_msg.sender_type = 'customer'
                    AND c.provider_pubkey = ?
                GROUP BY customer_msg.id
            )
            "#,
            provider_pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use crate::database::test_helpers::setup_test_db;

    #[tokio::test]
    async fn test_insert_chatwoot_message_event() {
        let db = setup_test_db().await;

        let result = db
            .insert_chatwoot_message_event("contract123", 1, 100, "customer", 1700000000)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_insert_chatwoot_message_event_duplicate() {
        let db = setup_test_db().await;

        // First insert should succeed
        db.insert_chatwoot_message_event("contract123", 1, 100, "customer", 1700000000)
            .await
            .unwrap();

        // Duplicate message_id should fail (UNIQUE constraint)
        let result = db
            .insert_chatwoot_message_event("contract456", 2, 100, "provider", 1700000001)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_provider_avg_response_time_no_data() {
        let db = setup_test_db().await;

        let result = db.get_provider_avg_response_time(b"nonexistent").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
