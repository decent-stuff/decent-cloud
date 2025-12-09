use super::types::Database;
use anyhow::Result;

/// SLA breach info returned by breach detection query.
#[derive(Debug)]
pub struct SlaBreach {
    pub message_id: i64,
    pub contract_id: String,
    #[allow(dead_code)]
    pub conversation_id: i64,
    pub provider_pubkey: Vec<u8>,
    pub created_at: i64,
}

/// Response time distribution across time buckets.
#[derive(Debug, Clone, Default)]
pub struct ResponseTimeDistribution {
    pub within_1h_pct: f64,
    pub within_4h_pct: f64,
    pub within_12h_pct: f64,
    pub within_24h_pct: f64,
    pub within_72h_pct: f64,
    pub total_responses: i64,
}

/// Provider response time metrics.
#[derive(Debug, Clone)]
pub struct ProviderResponseMetrics {
    pub avg_response_seconds: Option<f64>,
    pub sla_compliance_percent: f64,
    pub breach_count_30d: i64,
    pub total_inquiries_30d: i64,
    pub distribution: ResponseTimeDistribution,
}

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

    /// Find customer messages that have breached SLA (no provider response within threshold).
    /// Default SLA is 4 hours (14400 seconds) if not configured.
    pub async fn get_sla_breaches(&self) -> Result<Vec<SlaBreach>> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let rows = sqlx::query!(
            r#"
            SELECT
                cm.id as "id!",
                cm.contract_id as "contract_id!",
                cm.chatwoot_conversation_id as "conversation_id!",
                c.provider_pubkey as "provider_pubkey!",
                cm.created_at as "created_at!"
            FROM chatwoot_message_events cm
            JOIN contract_sign_requests c ON hex(c.contract_id) = cm.contract_id
            LEFT JOIN provider_sla_config sla ON sla.provider_pubkey = c.provider_pubkey
            WHERE cm.sender_type = 'customer'
              AND cm.sla_breached = 0
              AND cm.sla_alert_sent = 0
              AND NOT EXISTS (
                  SELECT 1 FROM chatwoot_message_events response
                  WHERE response.chatwoot_conversation_id = cm.chatwoot_conversation_id
                    AND response.sender_type = 'provider'
                    AND response.created_at > cm.created_at
              )
              AND (? - cm.created_at) > COALESCE(sla.response_time_seconds, 14400)
            "#,
            now
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| SlaBreach {
                message_id: r.id,
                contract_id: r.contract_id,
                conversation_id: r.conversation_id,
                provider_pubkey: r.provider_pubkey,
                created_at: r.created_at,
            })
            .collect())
    }

    /// Mark a message event as SLA breached.
    pub async fn mark_sla_breached(&self, message_id: i64) -> Result<()> {
        sqlx::query!(
            "UPDATE chatwoot_message_events SET sla_breached = 1 WHERE id = ?",
            message_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Mark that SLA alert was sent for a message event.
    pub async fn mark_sla_alert_sent(&self, message_id: i64) -> Result<()> {
        sqlx::query!(
            "UPDATE chatwoot_message_events SET sla_alert_sent = 1 WHERE id = ?",
            message_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get response time distribution for a provider.
    pub async fn get_response_time_distribution(
        &self,
        provider_pubkey: &[u8],
    ) -> Result<ResponseTimeDistribution> {
        // Time thresholds in seconds
        const ONE_HOUR: i64 = 3600;
        const FOUR_HOURS: i64 = 14400;
        const TWELVE_HOURS: i64 = 43200;
        const TWENTY_FOUR_HOURS: i64 = 86400;
        const SEVENTY_TWO_HOURS: i64 = 259200;

        let stats = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as "total!: i64",
                COALESCE(SUM(CASE WHEN response_time <= ? THEN 1 ELSE 0 END), 0) as "within_1h!: i64",
                COALESCE(SUM(CASE WHEN response_time <= ? THEN 1 ELSE 0 END), 0) as "within_4h!: i64",
                COALESCE(SUM(CASE WHEN response_time <= ? THEN 1 ELSE 0 END), 0) as "within_12h!: i64",
                COALESCE(SUM(CASE WHEN response_time <= ? THEN 1 ELSE 0 END), 0) as "within_24h!: i64",
                COALESCE(SUM(CASE WHEN response_time <= ? THEN 1 ELSE 0 END), 0) as "within_72h!: i64"
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
            ONE_HOUR,
            FOUR_HOURS,
            TWELVE_HOURS,
            TWENTY_FOUR_HOURS,
            SEVENTY_TWO_HOURS,
            provider_pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        let total = stats.total;
        if total == 0 {
            return Ok(ResponseTimeDistribution::default());
        }

        let pct = |count: i64| (count as f64 / total as f64) * 100.0;

        Ok(ResponseTimeDistribution {
            within_1h_pct: pct(stats.within_1h),
            within_4h_pct: pct(stats.within_4h),
            within_12h_pct: pct(stats.within_12h),
            within_24h_pct: pct(stats.within_24h),
            within_72h_pct: pct(stats.within_72h),
            total_responses: total,
        })
    }

    /// Get response time metrics for a provider (last 30 days).
    pub async fn get_provider_response_metrics(
        &self,
        provider_pubkey: &[u8],
    ) -> Result<ProviderResponseMetrics> {
        let thirty_days_ago = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            - (30 * 24 * 60 * 60);

        // Get average response time (all time)
        let avg = self.get_provider_avg_response_time(provider_pubkey).await?;

        // Get response time distribution
        let distribution = self.get_response_time_distribution(provider_pubkey).await?;

        // Get breach and total counts for last 30 days
        let stats = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as "total!: i64",
                COALESCE(SUM(CASE WHEN cm.sla_breached = 1 THEN 1 ELSE 0 END), 0) as "breached!: i64"
            FROM chatwoot_message_events cm
            JOIN contract_sign_requests c ON hex(c.contract_id) = cm.contract_id
            WHERE cm.sender_type = 'customer'
              AND c.provider_pubkey = ?
              AND cm.created_at >= ?
            "#,
            provider_pubkey,
            thirty_days_ago
        )
        .fetch_one(&self.pool)
        .await?;

        let total: i64 = stats.total;
        let breached: i64 = stats.breached;
        let compliance = if total > 0 {
            ((total - breached) as f64 / total as f64) * 100.0
        } else {
            100.0
        };

        Ok(ProviderResponseMetrics {
            avg_response_seconds: avg,
            sla_compliance_percent: compliance,
            breach_count_30d: breached,
            total_inquiries_30d: total,
            distribution,
        })
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

    #[tokio::test]
    async fn test_mark_sla_breached() {
        let db = setup_test_db().await;

        // Insert a message event
        db.insert_chatwoot_message_event("contract123", 1, 200, "customer", 1700000000)
            .await
            .unwrap();

        // Mark as breached
        db.mark_sla_breached(1).await.unwrap();

        // Verify (by trying to mark alert sent on same record)
        db.mark_sla_alert_sent(1).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_provider_response_metrics_no_data() {
        let db = setup_test_db().await;

        let metrics = db
            .get_provider_response_metrics(b"nonexistent")
            .await
            .unwrap();
        assert!(metrics.avg_response_seconds.is_none());
        assert_eq!(metrics.sla_compliance_percent, 100.0);
        assert_eq!(metrics.breach_count_30d, 0);
        assert_eq!(metrics.total_inquiries_30d, 0);
        assert_eq!(metrics.distribution.total_responses, 0);
    }

    #[tokio::test]
    async fn test_get_response_time_distribution_no_data() {
        let db = setup_test_db().await;

        let dist = db
            .get_response_time_distribution(b"nonexistent")
            .await
            .unwrap();
        assert_eq!(dist.total_responses, 0);
        assert_eq!(dist.within_1h_pct, 0.0);
    }
}
