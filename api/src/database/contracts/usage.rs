use super::*;
use crate::database::types::Database;
use anyhow::Result;

impl Database {
    // === Contract Usage Tracking ===

    /// Record a usage event for a contract
    pub async fn record_usage_event(
        &self,
        contract_id: &[u8],
        event_type: &str,
        units_delta: Option<f64>,
        heartbeat_at: Option<i64>,
        source: Option<&str>,
        metadata: Option<&str>,
    ) -> Result<i64> {
        let result = sqlx::query!(
            r#"INSERT INTO contract_usage_events (contract_id, event_type, units_delta, heartbeat_at, source, metadata)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING id as "id!: i64""#,
            contract_id,
            event_type,
            units_delta,
            heartbeat_at,
            source,
            metadata
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.id)
    }

    /// Get current billing period usage for a contract
    pub async fn get_current_usage(&self, contract_id: &[u8]) -> Result<Option<ContractUsage>> {
        let now = chrono::Utc::now().timestamp();
        let usage = sqlx::query_as::<_, ContractUsage>(
            r#"SELECT
                cu.id,
                lower(encode(cu.contract_id, 'hex')) as contract_id,
                cu.billing_period_start,
                cu.billing_period_end,
                cu.units_used,
                cu.units_included,
                cu.overage_units,
                cu.estimated_charge_cents,
                cu.reported_to_stripe,
                cu.stripe_usage_record_id,
                cu.created_at,
                cu.updated_at,
                COALESCE(po.billing_unit, 'hour') as billing_unit
            FROM contract_usage cu
            JOIN contract_sign_requests csr ON cu.contract_id = csr.contract_id
            LEFT JOIN provider_offerings po ON csr.offering_id = po.offering_id
            WHERE cu.contract_id = $1 AND cu.billing_period_start <= $2 AND cu.billing_period_end > $3
            ORDER BY cu.billing_period_start DESC
            LIMIT 1"#,
        )
        .bind(contract_id)
        .bind(now)
        .bind(now)
        .fetch_optional(&self.pool)
        .await?;

        Ok(usage)
    }

    /// Update usage from heartbeat events for a contract
    /// Calculates units_used based on heartbeat intervals
    pub async fn update_usage_from_heartbeats(
        &self,
        contract_id: &[u8],
        usage_id: i64,
        billing_unit: &str,
    ) -> Result<f64> {
        // Get the billing period
        let usage = sqlx::query!(
            r#"SELECT billing_period_start as "billing_period_start!: i64",
                      billing_period_end as "billing_period_end!: i64",
                      units_included
               FROM contract_usage WHERE id = $1"#,
            usage_id
        )
        .fetch_one(&self.pool)
        .await?;

        // Get heartbeat events in this billing period
        let heartbeats = sqlx::query!(
            r#"SELECT heartbeat_at as "heartbeat_at!: i64"
               FROM contract_usage_events
               WHERE contract_id = $1
                 AND event_type = 'heartbeat'
                 AND heartbeat_at >= $2
                 AND heartbeat_at < $3
               ORDER BY heartbeat_at ASC"#,
            contract_id,
            usage.billing_period_start,
            usage.billing_period_end
        )
        .fetch_all(&self.pool)
        .await?;

        // Calculate total units based on billing_unit
        let units_per_second = match billing_unit {
            "minute" => 1.0 / 60.0,
            "hour" => 1.0 / 3600.0,
            "day" => 1.0 / 86400.0,
            "month" => 1.0 / (30.0 * 86400.0),
            _ => 1.0 / 3600.0, // Default to hourly
        };

        let mut total_units = 0.0;
        let mut prev_ts: Option<i64> = None;

        for hb in &heartbeats {
            if let Some(prev) = prev_ts {
                let interval_seconds = (hb.heartbeat_at - prev) as f64;
                // Cap interval at 10 minutes (600 seconds) - if no heartbeat for longer, assume offline
                let capped_interval = interval_seconds.min(600.0);
                total_units += capped_interval * units_per_second;
            }
            prev_ts = Some(hb.heartbeat_at);
        }

        // Calculate overage
        let overage = if let Some(included) = usage.units_included {
            (total_units - included).max(0.0)
        } else {
            0.0
        };

        // Update the usage record
        let now_ns = crate::now_ns()?;
        sqlx::query!(
            "UPDATE contract_usage SET units_used = $1, overage_units = $2, updated_at = $3 WHERE id = $4",
            total_units,
            overage,
            now_ns,
            usage_id
        )
        .execute(&self.pool)
        .await?;

        Ok(total_units)
    }

    /// Mark usage as reported to Stripe
    pub async fn mark_usage_reported(
        &self,
        usage_id: i64,
        stripe_usage_record_id: &str,
    ) -> Result<()> {
        let now_ns = crate::now_ns()?;
        sqlx::query!(
            "UPDATE contract_usage SET reported_to_stripe = TRUE, stripe_usage_record_id = $1, updated_at = $2 WHERE id = $3",
            stripe_usage_record_id,
            now_ns,
            usage_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get unreported usage records that are past their billing period end
    pub async fn get_unreported_usage(&self) -> Result<Vec<ContractUsage>> {
        let now = chrono::Utc::now().timestamp();
        let usage = sqlx::query_as::<_, ContractUsage>(
            r#"SELECT
                cu.id,
                lower(encode(cu.contract_id, 'hex')) as contract_id,
                cu.billing_period_start,
                cu.billing_period_end,
                cu.units_used,
                cu.units_included,
                cu.overage_units,
                cu.estimated_charge_cents,
                cu.reported_to_stripe,
                cu.stripe_usage_record_id,
                cu.created_at,
                cu.updated_at,
                COALESCE(po.billing_unit, 'hour') as billing_unit
            FROM contract_usage cu
            JOIN contract_sign_requests csr ON cu.contract_id = csr.contract_id
            LEFT JOIN provider_offerings po ON csr.offering_id = po.offering_id
            WHERE cu.reported_to_stripe = FALSE AND cu.billing_period_end <= $1
            ORDER BY cu.billing_period_end ASC"#,
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await?;

        Ok(usage)
    }

    // === Contract Health Checks ===

    /// Record a health check for a contract
    ///
    /// Called by dc-agent to report the health status of a provisioned service.
    /// Returns the ID of the created health check record.
    pub async fn record_health_check(
        &self,
        contract_id: &[u8],
        checked_at: i64,
        status: &str,
        latency_ms: Option<i32>,
        details: Option<&str>,
    ) -> Result<i64> {
        // Validate status
        if !matches!(status, "healthy" | "unhealthy" | "unknown") {
            return Err(anyhow::anyhow!(
                "Invalid health status '{}'. Must be one of: healthy, unhealthy, unknown",
                status
            ));
        }

        let result = sqlx::query!(
            r#"INSERT INTO contract_health_checks (contract_id, checked_at, status, latency_ms, details)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id as "id!: i64""#,
            contract_id,
            checked_at,
            status,
            latency_ms,
            details
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.id)
    }

    /// Get recent health checks for a contract
    ///
    /// Returns health checks ordered by checked_at descending (most recent first).
    pub async fn get_recent_health_checks(
        &self,
        contract_id: &[u8],
        limit: i64,
    ) -> Result<Vec<ContractHealthCheck>> {
        let checks = sqlx::query_as!(
            ContractHealthCheck,
            r#"SELECT
                id as "id!: i64",
                lower(encode(contract_id, 'hex')) as "contract_id!: String",
                checked_at as "checked_at!: i64",
                status as "status!: String",
                latency_ms,
                details,
                created_at as "created_at!: i64"
            FROM contract_health_checks
            WHERE contract_id = $1
            ORDER BY checked_at DESC
            LIMIT $2"#,
            contract_id,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(checks)
    }

    /// Get provider health summary with uptime calculation
    ///
    /// Aggregates health check data across all contracts for a provider
    /// within the specified time window (default: last 30 days).
    ///
    /// # Arguments
    /// * `provider_pubkey` - Provider's public key
    /// * `days` - Number of days to look back (default: 30)
    ///
    /// # Returns
    /// Health summary with uptime percentage and metrics
    pub async fn get_provider_health_summary(
        &self,
        provider_pubkey: &[u8],
        days: Option<i64>,
    ) -> Result<ProviderHealthSummary> {
        let days = days.unwrap_or(30);
        let now_ns = crate::now_ns()?;
        let period_start_ns = now_ns - (days * 24 * 60 * 60 * 1_000_000_000);

        // Aggregate health checks for all contracts belonging to this provider
        let stats = sqlx::query!(
            r#"SELECT
                COUNT(*) as "total_checks!: i64",
                COALESCE(SUM(CASE WHEN hc.status = 'healthy' THEN 1 ELSE 0 END), 0) as "healthy_checks!: i64",
                COALESCE(SUM(CASE WHEN hc.status = 'unhealthy' THEN 1 ELSE 0 END), 0) as "unhealthy_checks!: i64",
                COALESCE(SUM(CASE WHEN hc.status = 'unknown' THEN 1 ELSE 0 END), 0) as "unknown_checks!: i64",
                AVG(hc.latency_ms)::DOUBLE PRECISION as "avg_latency_ms: f64",
                COUNT(DISTINCT hc.contract_id) as "contracts_monitored!: i64"
            FROM contract_health_checks hc
            JOIN contract_sign_requests csr ON hc.contract_id = csr.contract_id
            WHERE csr.provider_pubkey = $1
            AND hc.checked_at >= $2"#,
            provider_pubkey,
            period_start_ns
        )
        .fetch_one(&self.pool)
        .await?;

        // Calculate uptime percentage
        // If no checks, default to 0% (no data means we can't claim uptime)
        let uptime_percent = if stats.total_checks > 0 {
            (stats.healthy_checks as f64 / stats.total_checks as f64) * 100.0
        } else {
            0.0
        };

        Ok(ProviderHealthSummary {
            total_checks: stats.total_checks,
            healthy_checks: stats.healthy_checks,
            unhealthy_checks: stats.unhealthy_checks,
            unknown_checks: stats.unknown_checks,
            uptime_percent,
            avg_latency_ms: stats.avg_latency_ms,
            contracts_monitored: stats.contracts_monitored,
            period_start_ns,
            period_end_ns: now_ns,
        })
    }

    /// Get health summary for a single contract
    ///
    /// Aggregates all health check data for one contract (all-time).
    pub async fn get_contract_health_summary(
        &self,
        contract_id: &[u8],
    ) -> Result<ContractHealthSummary> {
        let stats = sqlx::query!(
            r#"SELECT
                COUNT(*) as "total_checks!: i64",
                COALESCE(SUM(CASE WHEN status = 'healthy' THEN 1 ELSE 0 END), 0) as "healthy_checks!: i64",
                COALESCE(SUM(CASE WHEN status = 'unhealthy' THEN 1 ELSE 0 END), 0) as "unhealthy_checks!: i64",
                COALESCE(SUM(CASE WHEN status = 'unknown' THEN 1 ELSE 0 END), 0) as "unknown_checks!: i64",
                AVG(latency_ms)::DOUBLE PRECISION as "avg_latency_ms: f64",
                MAX(checked_at) as "last_checked_at: i64"
            FROM contract_health_checks
            WHERE contract_id = $1"#,
            contract_id
        )
        .fetch_one(&self.pool)
        .await?;

        let uptime_percent = if stats.total_checks > 0 {
            (stats.healthy_checks as f64 / stats.total_checks as f64) * 100.0
        } else {
            0.0
        };

        Ok(ContractHealthSummary {
            total_checks: stats.total_checks,
            healthy_checks: stats.healthy_checks,
            unhealthy_checks: stats.unhealthy_checks,
            unknown_checks: stats.unknown_checks,
            uptime_percent,
            avg_latency_ms: stats.avg_latency_ms,
            last_checked_at: stats.last_checked_at,
        })
    }
}
