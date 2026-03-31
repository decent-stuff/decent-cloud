use super::*;
use crate::database::types::Database;
use anyhow::Result;

impl Database {
    /// Get provider's uptime SLA configuration (threshold_percent and alert_window_hours).
    /// Returns None if no config row exists for this provider.
    pub async fn get_provider_sla_uptime_config(
        &self,
        provider_pubkey: &[u8],
    ) -> Result<Option<SlaUptimeConfig>> {
        let row = sqlx::query!(
            r#"SELECT uptime_threshold_percent, sla_alert_window_hours
               FROM provider_sla_config WHERE provider_pubkey = $1"#,
            provider_pubkey
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| SlaUptimeConfig {
            uptime_threshold_percent: r.uptime_threshold_percent,
            sla_alert_window_hours: r.sla_alert_window_hours,
        }))
    }

    /// Upsert provider's uptime SLA configuration.
    pub async fn upsert_provider_sla_uptime_config(
        &self,
        provider_pubkey: &[u8],
        uptime_threshold_percent: i32,
        sla_alert_window_hours: i32,
    ) -> Result<()> {
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        sqlx::query!(
            r#"INSERT INTO provider_sla_config
                   (provider_pubkey, response_time_seconds, created_at, updated_at,
                    uptime_threshold_percent, sla_alert_window_hours)
               VALUES ($1, 14400, $2, $2, $3, $4)
               ON CONFLICT (provider_pubkey) DO UPDATE
                   SET uptime_threshold_percent = EXCLUDED.uptime_threshold_percent,
                       sla_alert_window_hours   = EXCLUDED.sla_alert_window_hours,
                       updated_at               = EXCLUDED.updated_at"#,
            provider_pubkey,
            now_ns,
            uptime_threshold_percent,
            sla_alert_window_hours,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Return contracts whose uptime % is below their provider's configured threshold
    /// over the last `window_hours` hours, where no alert has been sent in the last hour.
    ///
    /// Only considers contracts with status 'active' or 'provisioned'.
    pub async fn get_contracts_with_sla_breach(
        &self,
        window_hours: i32,
        threshold_percent: i32,
    ) -> Result<Vec<SlaBreachInfo>> {
        let window_ns = (window_hours as i64) * 3600 * 1_000_000_000_i64;
        let one_hour_ns = 3600 * 1_000_000_000_i64;
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let window_start = now_ns - window_ns;
        let alert_cutoff = now_ns - one_hour_ns;

        let rows = sqlx::query!(
            r#"SELECT
                   encode(c.contract_id, 'hex') AS "contract_id!",
                   encode(c.provider_pubkey, 'hex') AS "provider_pubkey!",
                   COUNT(h.id)                  AS "total_checks!: i64",
                   SUM(CASE WHEN h.status = 'healthy' THEN 1 ELSE 0 END) AS "healthy_checks!: i64"
               FROM contract_sign_requests c
               JOIN contract_health_checks h ON h.contract_id = c.contract_id
               WHERE c.status IN ('active', 'provisioned')
                 AND h.checked_at >= $1
               GROUP BY c.contract_id, c.provider_pubkey
               HAVING COUNT(h.id) > 0
                 AND (SUM(CASE WHEN h.status = 'healthy' THEN 1 ELSE 0 END) * 100 / COUNT(h.id)) < $2
                 AND NOT EXISTS (
                     SELECT 1 FROM sla_breach_alerts a
                     WHERE a.contract_id = c.contract_id
                       AND a.alert_sent_at > $3
                 )"#,
            window_start,
            threshold_percent as i64,
            alert_cutoff,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let uptime_percent = if r.total_checks > 0 {
                    ((r.healthy_checks * 100) / r.total_checks) as i32
                } else {
                    0
                };
                SlaBreachInfo {
                    contract_id: r.contract_id,
                    provider_pubkey: r.provider_pubkey,
                    uptime_percent,
                    threshold_percent,
                }
            })
            .collect())
    }

    /// Record that an SLA breach alert was sent for a contract.
    pub async fn upsert_sla_breach_alert(
        &self,
        contract_id: &[u8],
        provider_pubkey: &[u8],
        uptime_percent: i32,
        threshold_percent: i32,
    ) -> Result<()> {
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        sqlx::query!(
            r#"INSERT INTO sla_breach_alerts
                   (contract_id, provider_pubkey, uptime_percent, threshold_percent, alert_sent_at)
               VALUES ($1, $2, $3, $4, $5)
               ON CONFLICT (contract_id) DO UPDATE
                   SET provider_pubkey  = EXCLUDED.provider_pubkey,
                       uptime_percent   = EXCLUDED.uptime_percent,
                       threshold_percent = EXCLUDED.threshold_percent,
                       alert_sent_at    = EXCLUDED.alert_sent_at"#,
            contract_id,
            provider_pubkey,
            uptime_percent,
            threshold_percent,
            now_ns,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all distinct provider pubkeys that have an SLA uptime config row.
    pub async fn get_providers_with_sla_config(&self) -> Result<Vec<ProviderSlaRow>> {
        let rows = sqlx::query!(
            r#"SELECT provider_pubkey, uptime_threshold_percent, sla_alert_window_hours
               FROM provider_sla_config"#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ProviderSlaRow {
                provider_pubkey: r.provider_pubkey,
                uptime_threshold_percent: r.uptime_threshold_percent,
                sla_alert_window_hours: r.sla_alert_window_hours,
            })
            .collect())
    }
}
