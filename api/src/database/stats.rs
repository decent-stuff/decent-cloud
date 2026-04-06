use super::types::Database;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
pub struct PlatformStats {
    #[ts(type = "number")]
    pub total_providers: i64,
    #[ts(type = "number")]
    pub active_providers: i64,
    #[ts(type = "number")]
    pub total_offerings: i64,
    #[ts(type = "number")]
    pub total_contracts: i64,
    #[ts(type = "number")]
    pub total_transfers: i64,
    #[ts(type = "number")]
    pub total_volume_e9s: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, poem_openapi::Object)]
pub struct ReputationInfo {
    #[oai(skip)]
    pub pubkey: Vec<u8>,
    pub total_reputation: i64,
    pub change_count: i64,
}

impl Database {
    /// Get the latest block timestamp from provider check-ins
    pub async fn get_latest_block_timestamp_ns(&self) -> Result<Option<i64>> {
        let result = sqlx::query_scalar!("SELECT MAX(block_timestamp_ns) FROM provider_check_ins")
            .fetch_one(&self.pool)
            .await?;
        Ok(result)
    }

    /// Get platform-wide statistics
    pub async fn get_platform_stats(&self) -> Result<PlatformStats> {
        // Total providers = all who have ever checked in or created a profile
        // Exclude the example provider used for template generation
        let example_provider_hash = Self::example_provider_pubkey();
        let total_providers: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(DISTINCT pubkey) as "count!" FROM (
                SELECT pubkey FROM provider_profiles WHERE pubkey != $1
                UNION
                SELECT pubkey FROM provider_check_ins WHERE pubkey != $2
            ) AS combined"#,
            &example_provider_hash,
            &example_provider_hash
        )
        .fetch_one(&self.pool)
        .await?;

        // Active in the last year
        let cutoff_ns = crate::now_ns()? - 365 * 24 * 3600 * 1_000_000_000;
        let active_providers: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(DISTINCT pubkey) as "count!" FROM provider_check_ins WHERE block_timestamp_ns > $1 AND (pubkey) != $2"#,
            cutoff_ns,
            &example_provider_hash
        )
        .fetch_one(&self.pool)
        .await?;

        let total_offerings: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM provider_offerings WHERE LOWER(visibility) = 'public' AND pubkey != $1"#,
            &example_provider_hash
        )
        .fetch_one(&self.pool)
        .await?;

        let total_contracts: i64 =
            sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM contract_sign_requests"#)
                .fetch_one(&self.pool)
                .await?;

        let total_transfers: i64 =
            sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM token_transfers"#)
                .fetch_one(&self.pool)
                .await?;

        let total_volume: Option<i64> =
            sqlx::query_scalar!(r#"SELECT SUM(amount_e9s)::BIGINT as "sum" FROM token_transfers"#)
                .fetch_one(&self.pool)
                .await?;

        Ok(PlatformStats {
            total_providers,
            active_providers,
            total_offerings,
            total_contracts,
            total_transfers,
            total_volume_e9s: total_volume.unwrap_or(0),
        })
    }

    /// Get reputation for an identity
    pub async fn get_reputation(&self, pubkey: &[u8]) -> Result<Option<ReputationInfo>> {
        let info = sqlx::query_as!(
            ReputationInfo,
            r#"SELECT pubkey, COALESCE(SUM(change_amount), 0)::BIGINT as "total_reputation!: i64", COUNT(*)::BIGINT as "change_count!: i64"
             FROM reputation_changes
             WHERE pubkey = $1
             GROUP BY pubkey"#,
            pubkey
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(info)
    }

    /// Get monthly revenue breakdown for a provider (last 12 months)
    pub async fn get_provider_revenue_by_month(
        &self,
        pubkey: &[u8],
    ) -> Result<Vec<RevenueByMonth>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            month: String,
            revenue_e9s: i64,
            contract_count: i64,
        }

        let rows = sqlx::query_as::<_, Row>(
            r#"SELECT
                to_char(to_timestamp(created_at_ns / 1000000000), 'YYYY-MM') as month,
                COALESCE(SUM(payment_amount_e9s), 0)::BIGINT as revenue_e9s,
                COUNT(*)::BIGINT as contract_count
               FROM contract_sign_requests
               WHERE provider_pubkey = $1
                 AND created_at_ns > (EXTRACT(EPOCH FROM NOW() - INTERVAL '12 months') * 1000000000)::BIGINT
               GROUP BY month
               ORDER BY month ASC"#,
        )
        .bind(pubkey)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| RevenueByMonth {
                month: r.month,
                revenue_e9s: r.revenue_e9s,
                contract_count: r.contract_count,
            })
            .collect())
    }

    /// Get contract stats for a provider
    pub async fn get_provider_stats(&self, pubkey: &[u8]) -> Result<ProviderStats> {
        let total_contracts: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM contract_sign_requests WHERE provider_pubkey = $1"#,
            pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        let pending_contracts: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM contract_sign_requests WHERE provider_pubkey = $1 AND status = 'pending'"#,
            pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        let total_revenue: i64 = sqlx::query_scalar!(
            r#"SELECT COALESCE(SUM(payment_amount_e9s), 0)::BIGINT as "sum!" FROM contract_sign_requests WHERE provider_pubkey = $1"#,
            pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        let offerings_count: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM provider_offerings WHERE pubkey = $1"#,
            pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(ProviderStats {
            total_contracts,
            pending_contracts,
            total_revenue_e9s: total_revenue,
            offerings_count,
        })
    }

    /// Compute reliability score (0.0–100.0) for a provider.
    ///
    /// Formula: uptime_pct * 0.40 + completion_rate_pct * 0.35 + response_rate_pct * 0.25
    ///
    /// - uptime_pct: healthy_checks / total_checks * 100 (default 50 when < 3 checks)
    /// - completion_rate_pct: completed / (total - cancelled_by_tenant) * 100
    /// - response_rate_pct: non_rejected / total * 100
    ///
    /// Returns None when total_checks < 3 AND total_contracts < 3 (insufficient data).
    pub async fn get_provider_reliability_score(&self, pubkey: &[u8]) -> Result<Option<f64>> {
        #[derive(sqlx::FromRow)]
        struct HealthRow {
            total_checks: i64,
            healthy_checks: i64,
        }

        let health = sqlx::query_as::<_, HealthRow>(
            r#"SELECT
                COUNT(*)::BIGINT as total_checks,
                COUNT(*) FILTER (WHERE hc.status = 'healthy')::BIGINT as healthy_checks
               FROM contract_health_checks hc
               JOIN contract_sign_requests csr ON hc.contract_id = csr.contract_id
               WHERE csr.provider_pubkey = $1"#,
        )
        .bind(pubkey)
        .fetch_one(&self.pool)
        .await?;

        #[derive(sqlx::FromRow)]
        struct ContractRow {
            total: i64,
            completed: i64,
            cancelled_by_tenant: i64,
            rejected: i64,
        }

        let contracts = sqlx::query_as::<_, ContractRow>(
            r#"SELECT
                COUNT(*)::BIGINT as total,
                COUNT(*) FILTER (WHERE status = 'completed')::BIGINT as completed,
                COUNT(*) FILTER (WHERE status = 'cancelled')::BIGINT as cancelled_by_tenant,
                COUNT(*) FILTER (WHERE status = 'rejected')::BIGINT as rejected
               FROM contract_sign_requests
               WHERE provider_pubkey = $1"#,
        )
        .bind(pubkey)
        .fetch_one(&self.pool)
        .await?;

        // Require at least 3 health checks OR 3 contracts for a meaningful score
        if health.total_checks < 3 && contracts.total < 3 {
            return Ok(None);
        }

        // Uptime: default 50 (neutral) when fewer than 3 health checks
        let uptime_pct = if health.total_checks >= 3 {
            (health.healthy_checks as f64 / health.total_checks as f64) * 100.0
        } else {
            50.0
        };

        // Completion rate: denominator excludes tenant-cancelled contracts
        let completion_denominator = contracts.total - contracts.cancelled_by_tenant;
        let completion_rate_pct = if completion_denominator > 0 {
            (contracts.completed as f64 / completion_denominator as f64) * 100.0
        } else {
            100.0 // All contracts cancelled by tenant — no negative signal
        };

        // Response rate: non-rejected / total
        let response_rate_pct = if contracts.total > 0 {
            ((contracts.total - contracts.rejected) as f64 / contracts.total as f64) * 100.0
        } else {
            100.0
        };

        let score = uptime_pct * 0.40 + completion_rate_pct * 0.35 + response_rate_pct * 0.25;
        let sla_penalty = self.get_provider_sli_penalty_points(pubkey, 30).await?;
        Ok(Some((score - sla_penalty).clamp(0.0, 100.0)))
    }

    /// Get trust metrics for a provider
    pub async fn get_provider_trust_metrics(&self, pubkey: &[u8]) -> Result<ProviderTrustMetrics> {
        let now_ns = crate::now_ns()?;
        let ns_per_hour: i64 = 3600 * 1_000_000_000;
        let ns_per_day: i64 = 24 * ns_per_hour;
        let cutoff_90d_ns = now_ns - 90 * ns_per_day;
        let cutoff_72h_ns = now_ns - 72 * ns_per_hour;

        // Total contracts for this provider
        let total_contracts: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM contract_sign_requests WHERE provider_pubkey = $1"#,
            pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        // Completed contracts
        let completed_contracts: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM contract_sign_requests WHERE provider_pubkey = $1 AND status = 'completed'"#,
            pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        // Cancelled contracts
        let cancelled_contracts: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM contract_sign_requests WHERE provider_pubkey = $1 AND status = 'cancelled'"#,
            pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        // Rejected contracts
        let rejected_contracts: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM contract_sign_requests WHERE provider_pubkey = $1 AND status = 'rejected'"#,
            pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        // Active contracts value
        let active_contract_value_e9s: i64 = sqlx::query_scalar!(
            r#"SELECT COALESCE(SUM(payment_amount_e9s), 0)::BIGINT as "sum!" FROM contract_sign_requests WHERE provider_pubkey = $1 AND status IN ('active', 'provisioned')"#,
            pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        // Stuck contracts (>72h without progress in early stages)
        let stuck_contracts_value_e9s: i64 = sqlx::query_scalar!(
            r#"SELECT COALESCE(SUM(payment_amount_e9s), 0)::BIGINT as "sum!" FROM contract_sign_requests WHERE provider_pubkey = $1 AND status IN ('requested', 'pending', 'accepted') AND created_at_ns < $2"#,
            pubkey,
            cutoff_72h_ns
        )
        .fetch_one(&self.pool)
        .await?;

        // Repeat customers (users who rented more than once from this provider)
        let repeat_customer_count: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM (
                SELECT requester_pubkey FROM contract_sign_requests
                WHERE provider_pubkey = $1
                GROUP BY requester_pubkey
                HAVING COUNT(*) > 1
            ) AS repeats"#,
            pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        // Last activity timestamp - check multiple sources:
        // 1. Blockchain check-ins
        // 2. Contract activity (status updates as provider)
        // 3. Profile updates
        // 4. Account login (via account_public_keys link)
        let last_active_ns: i64 = sqlx::query_scalar::<_, i64>(
            r#"SELECT COALESCE(MAX(activity_ns), 0) FROM (
                SELECT MAX(block_timestamp_ns) as activity_ns FROM provider_check_ins WHERE pubkey = $1
                UNION ALL
                SELECT MAX(COALESCE(status_updated_at_ns, created_at_ns)) FROM contract_sign_requests WHERE provider_pubkey = $2
                UNION ALL
                SELECT MAX(updated_at_ns) FROM provider_profiles WHERE pubkey = $3
                UNION ALL
                SELECT MAX(a.last_login_at) FROM accounts a
                INNER JOIN account_public_keys apk ON a.id = apk.account_id
                WHERE apk.public_key = $4
            )"#,
        )
        .bind(pubkey)
        .bind(pubkey)
        .bind(pubkey)
        .bind(pubkey)
        .fetch_one(&self.pool)
        .await?;

        // Negative reputation in last 90 days
        let negative_reputation_90d: i64 = sqlx::query_scalar!(
            r#"SELECT COALESCE(SUM(CASE WHEN change_amount < 0 THEN change_amount ELSE 0 END), 0)::BIGINT as "sum!" FROM reputation_changes WHERE pubkey = $1 AND block_timestamp_ns > $2"#,
            pubkey,
            cutoff_90d_ns
        )
        .fetch_one(&self.pool)
        .await?;

        // Average response time (time from created_at to first status change)
        let avg_response_time_ns: Option<f64> = sqlx::query_scalar!(
            r#"SELECT AVG(CAST(h.changed_at_ns - c.created_at_ns AS DOUBLE PRECISION)) as "avg: f64"
               FROM contract_sign_requests c
               INNER JOIN contract_status_history h ON c.contract_id = h.contract_id
               WHERE c.provider_pubkey = $1
               AND h.old_status = 'requested'
               AND h.changed_at_ns IS NOT NULL"#,
            pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        // Time to delivery (median time from created to provisioned)
        // Using average as PostgreSQL's MEDIAN requires window function
        let avg_delivery_time_ns: Option<f64> = sqlx::query_scalar!(
            r#"SELECT AVG(CAST(provisioning_completed_at_ns - created_at_ns AS DOUBLE PRECISION)) as "avg: f64"
               FROM contract_sign_requests
               WHERE provider_pubkey = $1
               AND provisioning_completed_at_ns IS NOT NULL"#,
            pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        // Early cancellations (cancelled within first 10% of duration)
        let early_cancellations: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM contract_sign_requests c
               INNER JOIN contract_status_history h ON c.contract_id = h.contract_id
               WHERE c.provider_pubkey = $1
               AND c.status = 'cancelled'
               AND h.new_status = 'cancelled'
               AND c.duration_hours IS NOT NULL
               AND c.duration_hours > 0
               AND (h.changed_at_ns - c.start_timestamp_ns) < (c.duration_hours * 3600000000000 / 10)"#,
            pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        // Provisioning failures (accepted but never provisioned, older than 72h)
        let provisioning_failures: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM contract_sign_requests WHERE provider_pubkey = $1 AND status = 'accepted' AND provisioning_completed_at_ns IS NULL AND created_at_ns < $2"#,
            pubkey,
            cutoff_72h_ns
        )
        .fetch_one(&self.pool)
        .await?;

        // Accepted contracts count for failure rate
        let accepted_contracts: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM contract_sign_requests WHERE provider_pubkey = $1 AND status IN ('accepted', 'provisioning', 'provisioned', 'active', 'completed')"#,
            pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        // Calculate derived metrics
        let completion_rate_pct = if total_contracts > 0 {
            (completed_contracts as f64 / total_contracts as f64) * 100.0
        } else {
            0.0
        };

        let rejection_rate_pct = if total_contracts > 0 {
            Some((rejected_contracts as f64 / total_contracts as f64) * 100.0)
        } else {
            None
        };

        let early_cancellation_rate_pct = if cancelled_contracts > 0 {
            Some((early_cancellations as f64 / cancelled_contracts as f64) * 100.0)
        } else {
            None
        };

        let provisioning_failure_rate_pct = if accepted_contracts > 0 {
            Some((provisioning_failures as f64 / accepted_contracts as f64) * 100.0)
        } else {
            None
        };

        let avg_response_time_hours = avg_response_time_ns.map(|ns| ns / ns_per_hour as f64);
        let time_to_delivery_hours = avg_delivery_time_ns.map(|ns| ns / ns_per_hour as f64);

        // -1 indicates provider has never checked in
        let days_since_last_checkin = if last_active_ns > 0 {
            (now_ns - last_active_ns) / ns_per_day
        } else {
            -1
        };

        let is_new_provider = completed_contracts < 5;

        // Check if provider has contact info
        let contact_count: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM provider_profiles_contacts WHERE provider_pubkey = $1"#,
            pubkey
        )
        .fetch_one(&self.pool)
        .await?;
        let has_contact_info = contact_count > 0;

        // Fetch feedback stats for trust score integration
        let feedback_stats = self.get_provider_feedback_stats(pubkey).await?;
        let reliability_score = self.get_provider_reliability_score(pubkey).await?;

        // Tier 3 Contextual Metrics

        // Provider tenure classification based on completed contracts
        let provider_tenure = if completed_contracts < 5 {
            "new"
        } else if completed_contracts <= 20 {
            "growing"
        } else {
            "established"
        };

        // Average contract duration ratio (actual vs expected)
        // Calculate for completed and cancelled contracts only
        let avg_contract_duration_ratio: Option<f64> = sqlx::query_scalar!(
            r#"SELECT
                CASE
                    WHEN AVG(duration_hours) > 0 THEN
                        AVG(CAST(
                            CASE
                                WHEN status = 'completed' AND end_timestamp_ns IS NOT NULL AND start_timestamp_ns IS NOT NULL THEN
                                    (end_timestamp_ns - start_timestamp_ns) / 3600000000000.0
                                WHEN status = 'cancelled' AND status_updated_at_ns IS NOT NULL AND start_timestamp_ns IS NOT NULL THEN
                                    (status_updated_at_ns - start_timestamp_ns) / 3600000000000.0
                                ELSE NULL
                            END AS DOUBLE PRECISION
                        )) / AVG(duration_hours)
                    ELSE NULL
                END as "ratio: f64"
            FROM contract_sign_requests
            WHERE provider_pubkey = $1
            AND status IN ('completed', 'cancelled')
            AND duration_hours IS NOT NULL
            AND duration_hours > 0"#,
            pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        // No response rate (% of requests >7 days old still in "requested" status)
        let cutoff_7d_ns = now_ns - 7 * ns_per_day;

        // Count total requests in last 90 days
        let total_requests_90d: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM contract_sign_requests WHERE provider_pubkey = $1 AND created_at_ns > $2"#,
            pubkey,
            cutoff_90d_ns
        )
        .fetch_one(&self.pool)
        .await?;

        // Count requests still in "requested" status and older than 7 days
        let no_response_count: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM contract_sign_requests WHERE provider_pubkey = $1 AND status = 'requested' AND created_at_ns < $2"#,
            pubkey,
            cutoff_7d_ns
        )
        .fetch_one(&self.pool)
        .await?;

        let no_response_rate_pct = if total_requests_90d > 0 {
            Some((no_response_count as f64 / total_requests_90d as f64) * 100.0)
        } else {
            None
        };

        // Abandonment velocity (Tier 2 metric)
        let cutoff_30d_ns = now_ns - 30 * ns_per_day;
        let cutoff_31d_ns = now_ns - 31 * ns_per_day;

        // Recent period: last 30 days
        let recent_cancelled: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM contract_sign_requests WHERE provider_pubkey = $1 AND status = 'cancelled' AND status_updated_at_ns > $2"#,
            pubkey,
            cutoff_30d_ns
        )
        .fetch_one(&self.pool)
        .await?;

        let recent_total: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM contract_sign_requests WHERE provider_pubkey = $1 AND status IN ('completed', 'cancelled') AND status_updated_at_ns > $2"#,
            pubkey,
            cutoff_30d_ns
        )
        .fetch_one(&self.pool)
        .await?;

        // Baseline period: 31-90 days ago
        let baseline_cancelled: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM contract_sign_requests WHERE provider_pubkey = $1 AND status = 'cancelled' AND status_updated_at_ns > $2 AND status_updated_at_ns <= $3"#,
            pubkey,
            cutoff_90d_ns,
            cutoff_31d_ns
        )
        .fetch_one(&self.pool)
        .await?;

        let baseline_total: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM contract_sign_requests WHERE provider_pubkey = $1 AND status IN ('completed', 'cancelled') AND status_updated_at_ns > $2 AND status_updated_at_ns <= $3"#,
            pubkey,
            cutoff_90d_ns,
            cutoff_31d_ns
        )
        .fetch_one(&self.pool)
        .await?;

        let abandonment_velocity = if baseline_total == 0 {
            None
        } else {
            let recent_rate = if recent_total > 0 {
                recent_cancelled as f64 / recent_total as f64
            } else {
                0.0
            };
            let baseline_rate = baseline_cancelled as f64 / baseline_total as f64;

            if baseline_rate == 0.0 {
                Some(recent_rate)
            } else {
                Some(recent_rate / baseline_rate)
            }
        };

        // Calculate trust score and critical flags
        let (trust_score, has_critical_flags, critical_flag_reasons) =
            Self::calculate_trust_score_and_flags(
                early_cancellation_rate_pct,
                provisioning_failure_rate_pct,
                rejection_rate_pct,
                avg_response_time_hours,
                negative_reputation_90d,
                stuck_contracts_value_e9s,
                days_since_last_checkin,
                active_contract_value_e9s > 0,
                repeat_customer_count,
                completion_rate_pct,
                has_contact_info,
                feedback_stats.total_responses,
                feedback_stats.service_match_rate_pct,
                feedback_stats.would_rent_again_rate_pct,
            );

        // Update cached scores in provider_profiles
        sqlx::query!(
            "UPDATE provider_profiles SET trust_score = $1, has_critical_flags = $2, reliability_score = $3 WHERE pubkey = $4",
            trust_score,
            has_critical_flags,
            reliability_score,
            pubkey
        )
        .execute(&self.pool)
        .await?;

        Ok(ProviderTrustMetrics {
            pubkey: hex::encode(pubkey),
            trust_score,
            time_to_delivery_hours,
            completion_rate_pct,
            last_active_ns,
            repeat_customer_count,
            active_contract_value_e9s,
            total_contracts,
            early_cancellation_rate_pct,
            avg_response_time_hours,
            provisioning_failure_rate_pct,
            rejection_rate_pct,
            negative_reputation_90d,
            stuck_contracts_value_e9s,
            days_since_last_checkin,
            is_new_provider,
            has_contact_info,
            has_critical_flags,
            critical_flag_reasons,
            provider_tenure: provider_tenure.to_string(),
            avg_contract_duration_ratio,
            no_response_rate_pct,
            abandonment_velocity,
            feedback_count: feedback_stats.total_responses,
            feedback_service_match_rate_pct: if feedback_stats.total_responses > 0 {
                Some(feedback_stats.service_match_rate_pct)
            } else {
                None
            },
            feedback_would_rent_again_rate_pct: if feedback_stats.total_responses > 0 {
                Some(feedback_stats.would_rent_again_rate_pct)
            } else {
                None
            },
            reliability_score,
        })
    }

    /// Calculate trust score (0-100) and identify critical flags.
    /// Feedback params are only applied when `feedback_count` >= 3 (minimum for statistical relevance).
    #[allow(clippy::too_many_arguments)]
    fn calculate_trust_score_and_flags(
        early_cancellation_rate_pct: Option<f64>,
        provisioning_failure_rate_pct: Option<f64>,
        rejection_rate_pct: Option<f64>,
        avg_response_time_hours: Option<f64>,
        negative_reputation_90d: i64,
        stuck_contracts_value_e9s: i64,
        days_since_last_checkin: i64,
        has_active_contracts: bool,
        repeat_customer_count: i64,
        completion_rate_pct: f64,
        has_contact_info: bool,
        feedback_count: i64,
        feedback_service_match_rate_pct: f64,
        feedback_would_rent_again_rate_pct: f64,
    ) -> (i64, bool, Vec<String>) {
        let mut score: i64 = 100;
        let mut flags: Vec<String> = Vec::new();

        // Penalties
        if let Some(rate) = early_cancellation_rate_pct {
            if rate > 20.0 {
                score -= 25;
                flags.push(format!(
                    "High early cancellation rate: {:.0}% of contracts cancelled quickly",
                    rate
                ));
            }
        }

        if let Some(rate) = provisioning_failure_rate_pct {
            if rate > 15.0 {
                score -= 20;
                flags.push(format!(
                    "Provisioning failures: {:.0}% of accepted contracts never delivered",
                    rate
                ));
            }
        }

        if let Some(rate) = rejection_rate_pct {
            if rate > 30.0 {
                score -= 15;
                flags.push(format!(
                    "High rejection rate: {:.0}% of requests rejected",
                    rate
                ));
            }
        }

        if let Some(hours) = avg_response_time_hours {
            if hours > 48.0 {
                score -= 15;
                flags.push(format!(
                    "Slow response time: average {:.0} hours to respond",
                    hours
                ));
            }
        }

        if negative_reputation_90d < -50 {
            score -= 15;
            flags.push(format!(
                "Negative reputation trend: {} points lost in 90 days",
                negative_reputation_90d
            ));
        }

        // Ghost risk: inactive but has active contracts
        // days_since_last_checkin = -1 means no activity recorded
        if has_active_contracts && (days_since_last_checkin > 7 || days_since_last_checkin == -1) {
            score -= 10;
            if days_since_last_checkin == -1 {
                flags.push(
                    "Ghost risk: no platform activity recorded but has active contracts"
                        .to_string(),
                );
            } else {
                flags.push(format!(
                    "Ghost risk: {} days since last activity with active contracts",
                    days_since_last_checkin
                ));
            }
        }

        // Stuck contracts (convert e9s to approximate dollars: e9s / 1e9)
        let stuck_dollars = stuck_contracts_value_e9s / 1_000_000_000;
        if stuck_dollars > 5000 {
            score -= 10;
            flags.push(format!(
                "Stuck contracts: ~${} in contracts without progress for >72h",
                stuck_dollars
            ));
        }

        // No contact info - users can't reach provider for support
        if !has_contact_info {
            score -= 10;
            flags.push("No contact info: provider has no public contact methods".to_string());
        }

        // Feedback-based scoring (only with >= 3 responses for statistical relevance)
        if feedback_count >= 3 {
            if feedback_would_rent_again_rate_pct < 50.0 {
                score -= 15;
                flags.push(format!(
                    "Low renter satisfaction: only {:.0}% would rent again ({} reviews)",
                    feedback_would_rent_again_rate_pct, feedback_count
                ));
            }
            if feedback_service_match_rate_pct < 50.0 {
                score -= 10;
                flags.push(format!(
                    "Service mismatch: only {:.0}% say service matched description ({} reviews)",
                    feedback_service_match_rate_pct, feedback_count
                ));
            }
        }

        // Bonuses
        if repeat_customer_count > 10 {
            score += 5;
        }

        if completion_rate_pct > 95.0 {
            score += 5;
        }

        if let Some(hours) = avg_response_time_hours {
            if hours < 4.0 {
                score += 5;
            }
        }

        if feedback_count >= 5 && feedback_would_rent_again_rate_pct > 80.0 {
            score += 5;
        }

        // Clamp to 0-100
        score = score.clamp(0, 100);

        let has_critical = !flags.is_empty();

        (score, has_critical, flags)
    }

    /// Search accounts by username, display name, or public key
    pub async fn search_accounts(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<AccountSearchResult>> {
        // Prepare search pattern for LIKE queries
        let search_pattern = format!("%{}%", query.to_lowercase());
        let hex_search_pattern = format!("{}%", query.to_uppercase());

        #[derive(sqlx::FromRow)]
        struct SearchRow {
            username: String,
            display_name: Option<String>,
            pubkey: String,
            reputation_score: i64,
            contract_count: i64,
            offering_count: i64,
        }

        let results = sqlx::query_as::<_, SearchRow>(
            r#"SELECT DISTINCT
                a.username,
                a.display_name,
                encode(apk.public_key, 'hex') as pubkey,
                COALESCE(MAX(rep.total_reputation), 0)::BIGINT as reputation_score,
                COALESCE(SUM(contracts.contract_count), 0)::BIGINT as contract_count,
                COALESCE(MAX(offerings.offering_count), 0)::BIGINT as offering_count
            FROM accounts a
            INNER JOIN account_public_keys apk ON a.id = apk.account_id
            LEFT JOIN (
                SELECT pubkey, SUM(change_amount) as total_reputation
                FROM reputation_changes
                GROUP BY pubkey
            ) rep ON apk.public_key = rep.pubkey
            LEFT JOIN (
                SELECT provider_pubkey as pubkey, COUNT(*) as contract_count
                FROM contract_sign_requests
                GROUP BY provider_pubkey
                UNION ALL
                SELECT requester_pubkey as pubkey, COUNT(*) as contract_count
                FROM contract_sign_requests
                GROUP BY requester_pubkey
            ) contracts ON apk.public_key = contracts.pubkey
            LEFT JOIN (
                SELECT pubkey, COUNT(*) as offering_count
                FROM provider_offerings
                GROUP BY pubkey
            ) offerings ON apk.public_key = offerings.pubkey
            WHERE apk.is_active = TRUE
              AND (
                lower(a.username) LIKE $1
                OR lower(a.display_name) LIKE $2
                OR upper(encode(apk.public_key, 'hex')) LIKE $3
              )
            GROUP BY a.username, a.display_name, apk.public_key
            ORDER BY reputation_score DESC, contract_count DESC, offering_count DESC
            LIMIT $4"#,
        )
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&hex_search_pattern)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(results
            .into_iter()
            .map(|row| AccountSearchResult {
                username: row.username,
                display_name: row.display_name,
                pubkey: row.pubkey,
                reputation_score: row.reputation_score,
                contract_count: row.contract_count,
                offering_count: row.offering_count,
            })
            .collect())
    }
}

#[derive(Debug, Serialize, Deserialize, poem_openapi::Object)]
pub struct ProviderStats {
    pub total_contracts: i64,
    pub pending_contracts: i64,
    pub total_revenue_e9s: i64,
    pub offerings_count: i64,
}

#[derive(Debug, Serialize, Deserialize, poem_openapi::Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
pub struct RevenueByMonth {
    /// Year-month label, e.g. "2024-01"
    pub month: String,
    /// Total revenue in e9s for this month
    #[ts(type = "number")]
    pub revenue_e9s: i64,
    /// Number of contracts this month
    #[ts(type = "number")]
    pub contract_count: i64,
}

/// Provider trust metrics for transparency and red flag detection
#[derive(Debug, Clone, Serialize, Deserialize, poem_openapi::Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(skip_serializing_if_is_none)]
pub struct ProviderTrustMetrics {
    pub pubkey: String,

    // Core metrics
    /// Composite trust score 0-100
    pub trust_score: i64,
    /// Median hours from payment to provisioned service
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    #[ts(type = "number | undefined")]
    pub time_to_delivery_hours: Option<f64>,
    /// Percentage of contracts completed successfully
    pub completion_rate_pct: f64,
    /// Last check-in timestamp in nanoseconds
    #[ts(type = "number")]
    pub last_active_ns: i64,
    /// Number of users who rented more than once
    #[ts(type = "number")]
    pub repeat_customer_count: i64,
    /// Total payment value of active contracts (e9s)
    #[ts(type = "number")]
    pub active_contract_value_e9s: i64,
    /// Total contracts received (track record size)
    #[ts(type = "number")]
    pub total_contracts: i64,

    // Red flag metrics
    /// Percentage of contracts cancelled within first 10% of duration
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    #[ts(type = "number | undefined")]
    pub early_cancellation_rate_pct: Option<f64>,
    /// Average hours to first response after contract request
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    #[ts(type = "number | undefined")]
    pub avg_response_time_hours: Option<f64>,
    /// Percentage of accepted contracts never provisioned
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    #[ts(type = "number | undefined")]
    pub provisioning_failure_rate_pct: Option<f64>,
    /// Percentage of contract requests rejected
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    #[ts(type = "number | undefined")]
    pub rejection_rate_pct: Option<f64>,
    /// Sum of negative reputation changes in last 90 days
    #[ts(type = "number")]
    pub negative_reputation_90d: i64,
    /// Total value of contracts stuck >72h without progress
    #[ts(type = "number")]
    pub stuck_contracts_value_e9s: i64,
    /// Days since last provider check-in
    #[ts(type = "number")]
    pub days_since_last_checkin: i64,

    // Flags
    /// True if provider has <5 completed contracts
    pub is_new_provider: bool,
    /// True if provider has at least one contact method set
    pub has_contact_info: bool,
    /// True if any critical threshold exceeded
    pub has_critical_flags: bool,
    /// Human-readable list of exceeded thresholds
    pub critical_flag_reasons: Vec<String>,

    // Tier 3 Contextual Info Metrics
    /// Provider tenure classification: "new" (<5), "growing" (5-20), "established" (>20)
    pub provider_tenure: String,
    /// Ratio of actual contract duration to expected (avg_actual/avg_expected)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    #[ts(type = "number | undefined")]
    pub avg_contract_duration_ratio: Option<f64>,
    /// Percentage of requests that received no response (>7 days old, still "requested")
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    #[ts(type = "number | undefined")]
    pub no_response_rate_pct: Option<f64>,
    /// Abandonment velocity: ratio of recent (30d) to baseline (31-90d) cancellation rates.
    /// >1.5 = concerning, >2.0 = critical. None if insufficient baseline data.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    #[ts(type = "number | undefined")]
    pub abandonment_velocity: Option<f64>,

    // Renter feedback metrics
    /// Number of feedback responses received
    #[ts(type = "number")]
    pub feedback_count: i64,
    /// Percentage of renters who said service matched description (0-100). None if no feedback.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    #[ts(type = "number | undefined")]
    pub feedback_service_match_rate_pct: Option<f64>,
    /// Percentage of renters who would rent again (0-100). None if no feedback.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    #[ts(type = "number | undefined")]
    pub feedback_would_rent_again_rate_pct: Option<f64>,

    /// Composite reliability score 0-100: uptime 40% + completion rate 35% + response rate 25%.
    /// None if insufficient data (< 3 health checks AND < 3 contracts total).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    #[ts(type = "number | undefined")]
    pub reliability_score: Option<f64>,
}

/// Account search result with reputation and activity stats
#[derive(Debug, Serialize, Deserialize, poem_openapi::Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
pub struct AccountSearchResult {
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub display_name: Option<String>,
    pub pubkey: String,
    #[ts(type = "number")]
    pub reputation_score: i64,
    #[ts(type = "number")]
    pub contract_count: i64,
    #[ts(type = "number")]
    pub offering_count: i64,
}

/// Per-offering conversion stats: views vs rentals for a provider's offerings
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, poem_openapi::Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
#[oai(rename_all = "camelCase")]
pub struct OfferingConversionStats {
    /// The offering_id text identifier
    pub offering_id: String,
    /// Human-readable offering name
    pub offer_name: String,
    /// Product type (e.g. "vps", "dedicated", "gpu")
    pub product_type: String,
    /// Views in the last 7 days
    #[ts(type = "number")]
    pub views_7d: i64,
    /// Views in the last 30 days
    #[ts(type = "number")]
    pub views_30d: i64,
    /// Rentals (contract requests) in the last 7 days
    #[ts(type = "number")]
    pub rentals_7d: i64,
    /// Rentals (contract requests) in the last 30 days
    #[ts(type = "number")]
    pub rentals_30d: i64,
    /// Conversion rate: rentals_30d / views_30d * 100, or 0.0 if no views
    pub conversion_rate_30d: f64,
    /// Revenue from rentals in the last 30 days (in e9s)
    #[ts(type = "number")]
    pub revenue_30d_e9s: i64,
}

impl Database {
    /// Get per-offering conversion stats (views → rentals) for a provider
    pub async fn get_offering_conversion_stats(
        &self,
        provider_pubkey: &[u8],
    ) -> Result<Vec<OfferingConversionStats>> {
        let cutoff_7d_ms: i64 = (chrono::Utc::now().timestamp_millis()) - 7 * 24 * 3600 * 1000;
        let cutoff_30d_ms: i64 = (chrono::Utc::now().timestamp_millis()) - 30 * 24 * 3600 * 1000;
        let cutoff_7d_ns: i64 = cutoff_7d_ms * 1_000_000;
        let cutoff_30d_ns: i64 = cutoff_30d_ms * 1_000_000;

        let rows = sqlx::query_as::<_, OfferingConversionStats>(
            r#"SELECT
                o.offering_id,
                o.offer_name,
                o.product_type,
                COALESCE(ov7.views, 0)::BIGINT AS views_7d,
                COALESCE(ov30.views, 0)::BIGINT AS views_30d,
                COALESCE(c7.cnt, 0)::BIGINT AS rentals_7d,
                COALESCE(c30.cnt, 0)::BIGINT AS rentals_30d,
                CASE WHEN COALESCE(ov30.views, 0) > 0
                     THEN COALESCE(c30.cnt, 0)::DOUBLE PRECISION / ov30.views::DOUBLE PRECISION * 100.0
                     ELSE 0.0 END AS conversion_rate_30d,
                COALESCE(c30.revenue, 0)::BIGINT AS revenue_30d_e9s
            FROM provider_offerings o
            LEFT JOIN (
                SELECT offering_id, COUNT(*) AS views
                FROM offering_views
                WHERE viewed_at >= $2
                GROUP BY offering_id
            ) ov7 ON ov7.offering_id = o.id
            LEFT JOIN (
                SELECT offering_id, COUNT(*) AS views
                FROM offering_views
                WHERE viewed_at >= $3
                GROUP BY offering_id
            ) ov30 ON ov30.offering_id = o.id
            LEFT JOIN (
                SELECT offering_id, COUNT(*) AS cnt
                FROM contract_sign_requests
                WHERE created_at_ns >= $4
                  AND provider_pubkey = $1
                GROUP BY offering_id
            ) c7 ON c7.offering_id = o.offering_id
            LEFT JOIN (
                SELECT offering_id, COUNT(*) AS cnt, COALESCE(SUM(payment_amount_e9s), 0) AS revenue
                FROM contract_sign_requests
                WHERE created_at_ns >= $5
                  AND provider_pubkey = $1
                GROUP BY offering_id
            ) c30 ON c30.offering_id = o.offering_id
            WHERE o.pubkey = $1
            ORDER BY rentals_30d DESC, views_30d DESC"#,
        )
        .bind(provider_pubkey)
        .bind(cutoff_7d_ms)
        .bind(cutoff_30d_ms)
        .bind(cutoff_7d_ns)
        .bind(cutoff_30d_ns)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}

/// Per-offering satisfaction stats derived from tenant boolean feedback
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, poem_openapi::Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
#[oai(rename_all = "camelCase")]
pub struct OfferingSatisfactionStats {
    /// The offering_id text identifier
    pub offering_id: String,
    /// Human-readable offering name
    pub offer_name: String,
    /// Total feedback responses for this offering
    #[ts(type = "number")]
    pub total_feedback: i64,
    /// Count of "service matched description" = yes
    #[ts(type = "number")]
    pub service_matched_yes: i64,
    /// Count of "would rent again" = yes
    #[ts(type = "number")]
    pub would_rent_again_yes: i64,
    /// Composite satisfaction rate: average of both boolean metrics, 0.0–100.0
    pub satisfaction_rate_pct: f64,
}

impl Database {
    /// Get per-offering satisfaction stats (tenant feedback) for a provider
    pub async fn get_offering_satisfaction_stats(
        &self,
        provider_pubkey: &[u8],
    ) -> Result<Vec<OfferingSatisfactionStats>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            offering_id: String,
            offer_name: String,
            total_feedback: i64,
            service_matched_yes: i64,
            would_rent_again_yes: i64,
        }

        let rows = sqlx::query_as::<_, Row>(
            r#"SELECT
                o.offering_id,
                o.offer_name,
                COUNT(cf.id)::BIGINT AS total_feedback,
                COUNT(cf.id) FILTER (WHERE cf.service_matched_description = true)::BIGINT AS service_matched_yes,
                COUNT(cf.id) FILTER (WHERE cf.would_rent_again = true)::BIGINT AS would_rent_again_yes
            FROM provider_offerings o
            LEFT JOIN contract_sign_requests csr ON csr.offering_id = o.offering_id
                AND csr.provider_pubkey = $1
            LEFT JOIN contract_feedback cf ON cf.contract_id = csr.contract_id
            WHERE o.pubkey = $1
            GROUP BY o.offering_id, o.offer_name
            ORDER BY total_feedback DESC, o.offer_name ASC"#,
        )
        .bind(provider_pubkey)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let satisfaction_rate_pct = if r.total_feedback > 0 {
                    ((r.service_matched_yes + r.would_rent_again_yes) as f64
                        / (r.total_feedback * 2) as f64)
                        * 100.0
                } else {
                    0.0
                };
                OfferingSatisfactionStats {
                    offering_id: r.offering_id,
                    offer_name: r.offer_name,
                    total_feedback: r.total_feedback,
                    service_matched_yes: r.service_matched_yes,
                    would_rent_again_yes: r.would_rent_again_yes,
                    satisfaction_rate_pct,
                }
            })
            .collect())
    }
}

/// Contract feedback from renters (structured Y/N survey)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, poem_openapi::Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
pub struct ContractFeedback {
    /// Contract ID (hex-encoded)
    pub contract_id: String,
    /// Provider public key (hex-encoded)
    pub provider_pubkey: String,
    /// Did the service match its description?
    pub service_matched_description: bool,
    /// Would the renter rent from this provider again?
    pub would_rent_again: bool,
    /// When feedback was submitted (nanoseconds)
    #[ts(type = "number")]
    pub created_at_ns: i64,
}

/// Aggregated feedback stats for a provider
#[derive(Debug, Clone, Serialize, Deserialize, poem_openapi::Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
pub struct ProviderFeedbackStats {
    /// Provider public key (hex-encoded)
    pub provider_pubkey: String,
    /// Total feedback responses received
    #[ts(type = "number")]
    pub total_responses: i64,
    /// Count of "service matched description" = yes
    #[ts(type = "number")]
    pub service_matched_yes: i64,
    /// Count of "service matched description" = no
    #[ts(type = "number")]
    pub service_matched_no: i64,
    /// Count of "would rent again" = yes
    #[ts(type = "number")]
    pub would_rent_again_yes: i64,
    /// Count of "would rent again" = no
    #[ts(type = "number")]
    pub would_rent_again_no: i64,
    /// Percentage of "service matched" = yes (0-100)
    pub service_match_rate_pct: f64,
    /// Percentage of "would rent again" = yes (0-100)
    pub would_rent_again_rate_pct: f64,
}

/// Individual contract feedback entry for a provider's own review
#[derive(Debug, Clone, Serialize, Deserialize, poem_openapi::Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
pub struct ProviderContractFeedback {
    /// Contract ID (hex-encoded)
    pub contract_id: String,
    /// Provider public key (hex-encoded)
    pub provider_pubkey: String,
    /// Did the service match its description?
    pub service_matched_description: bool,
    /// Would the renter rent from this provider again?
    pub would_rent_again: bool,
    /// When feedback was submitted (nanoseconds)
    #[ts(type = "number")]
    pub created_at_ns: i64,
    /// When the contract was created (nanoseconds), if available
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    #[ts(type = "number | undefined")]
    pub contract_created_at_ns: Option<i64>,
}

/// Input for submitting contract feedback
#[derive(Debug, Clone, Deserialize, poem_openapi::Object)]
pub struct SubmitFeedbackInput {
    /// Did the service match its description?
    pub service_matched_description: bool,
    /// Would you rent from this provider again?
    pub would_rent_again: bool,
}

impl Database {
    /// Submit feedback for a completed contract.
    /// Only the contract requester can submit feedback, and only once per contract.
    /// Contract must be in a terminal state (completed/cancelled/terminated).
    pub async fn submit_contract_feedback(
        &self,
        contract_id: &[u8],
        requester_pubkey: &[u8],
        input: &SubmitFeedbackInput,
    ) -> Result<ContractFeedback> {
        // Verify contract exists and requester is authorized
        #[derive(sqlx::FromRow)]
        struct ContractRow {
            requester_pubkey: Vec<u8>,
            provider_pubkey: Vec<u8>,
            status: Option<String>,
        }

        let contract = sqlx::query_as::<_, ContractRow>(
            r#"SELECT requester_pubkey, provider_pubkey, status FROM contract_sign_requests WHERE contract_id = $1"#,
        )
        .bind(contract_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Contract not found"))?;

        // Verify the user is the contract requester
        if contract.requester_pubkey != requester_pubkey {
            anyhow::bail!("Only the contract requester can submit feedback");
        }

        // Verify contract is in a terminal state (feedback makes sense only for finished contracts)
        let terminal_states = ["completed", "cancelled", "terminated", "expired"];
        let status = contract.status.as_deref().unwrap_or("unknown");
        if !terminal_states.contains(&status) {
            anyhow::bail!(
                "Cannot submit feedback for contract in '{}' status. Contract must be completed, cancelled, terminated, or expired.",
                status
            );
        }

        let now_ns = chrono::Utc::now()
            .timestamp_nanos_opt()
            .ok_or_else(|| anyhow::anyhow!("Failed to get current timestamp"))?;

        // Insert feedback (UNIQUE constraint on contract_id prevents duplicates)
        sqlx::query(
            r#"INSERT INTO contract_feedback (contract_id, requester_pubkey, provider_pubkey, service_matched_description, would_rent_again, created_at_ns)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
        )
        .bind(contract_id)
        .bind(requester_pubkey)
        .bind(&contract.provider_pubkey)
        .bind(input.service_matched_description)
        .bind(input.would_rent_again)
        .bind(now_ns)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
                anyhow::anyhow!("Feedback already submitted for this contract")
            } else {
                anyhow::anyhow!("Failed to submit feedback: {}", e)
            }
        })?;

        Ok(ContractFeedback {
            contract_id: hex::encode(contract_id),
            provider_pubkey: hex::encode(&contract.provider_pubkey),
            service_matched_description: input.service_matched_description,
            would_rent_again: input.would_rent_again,
            created_at_ns: now_ns,
        })
    }

    /// Get feedback for a specific contract (if any)
    pub async fn get_contract_feedback(
        &self,
        contract_id: &[u8],
    ) -> Result<Option<ContractFeedback>> {
        #[derive(sqlx::FromRow)]
        struct FeedbackRow {
            contract_id: Vec<u8>,
            provider_pubkey: Vec<u8>,
            service_matched_description: bool,
            would_rent_again: bool,
            created_at_ns: i64,
        }

        let row = sqlx::query_as::<_, FeedbackRow>(
            r#"SELECT contract_id, provider_pubkey, service_matched_description, would_rent_again, created_at_ns
               FROM contract_feedback WHERE contract_id = $1"#,
        )
        .bind(contract_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| ContractFeedback {
            contract_id: hex::encode(r.contract_id),
            provider_pubkey: hex::encode(r.provider_pubkey),
            service_matched_description: r.service_matched_description,
            would_rent_again: r.would_rent_again,
            created_at_ns: r.created_at_ns,
        }))
    }

    /// Get aggregated feedback stats for a provider
    pub async fn get_provider_feedback_stats(
        &self,
        provider_pubkey: &[u8],
    ) -> Result<ProviderFeedbackStats> {
        #[derive(sqlx::FromRow)]
        struct StatsRow {
            total_responses: i64,
            service_matched_yes: i64,
            would_rent_again_yes: i64,
        }

        let stats = sqlx::query_as::<_, StatsRow>(
            r#"SELECT
                COUNT(*)::BIGINT as total_responses,
                COUNT(*) FILTER (WHERE service_matched_description = true)::BIGINT as service_matched_yes,
                COUNT(*) FILTER (WHERE would_rent_again = true)::BIGINT as would_rent_again_yes
               FROM contract_feedback WHERE provider_pubkey = $1"#,
        )
        .bind(provider_pubkey)
        .fetch_one(&self.pool)
        .await?;

        let service_matched_no = stats.total_responses - stats.service_matched_yes;
        let would_rent_again_no = stats.total_responses - stats.would_rent_again_yes;

        let service_match_rate_pct = if stats.total_responses > 0 {
            (stats.service_matched_yes as f64 / stats.total_responses as f64) * 100.0
        } else {
            0.0
        };

        let would_rent_again_rate_pct = if stats.total_responses > 0 {
            (stats.would_rent_again_yes as f64 / stats.total_responses as f64) * 100.0
        } else {
            0.0
        };

        Ok(ProviderFeedbackStats {
            provider_pubkey: hex::encode(provider_pubkey),
            total_responses: stats.total_responses,
            service_matched_yes: stats.service_matched_yes,
            service_matched_no,
            would_rent_again_yes: stats.would_rent_again_yes,
            would_rent_again_no,
            service_match_rate_pct,
            would_rent_again_rate_pct,
        })
    }

    /// Get all individual feedback entries for a provider's contracts, ordered newest first
    pub async fn get_provider_all_feedback(
        &self,
        provider_pubkey: &[u8],
    ) -> Result<Vec<ProviderContractFeedback>> {
        #[derive(sqlx::FromRow)]
        struct FeedbackRow {
            contract_id: Vec<u8>,
            provider_pubkey: Vec<u8>,
            service_matched_description: bool,
            would_rent_again: bool,
            created_at_ns: i64,
            contract_created_at_ns: Option<i64>,
        }

        let rows = sqlx::query_as::<_, FeedbackRow>(
            r#"SELECT cf.contract_id, cf.provider_pubkey, cf.service_matched_description, cf.would_rent_again, cf.created_at_ns,
                      csr.created_at_ns as contract_created_at_ns
               FROM contract_feedback cf
               LEFT JOIN contract_sign_requests csr ON csr.contract_id = cf.contract_id
               WHERE cf.provider_pubkey = $1
               ORDER BY cf.created_at_ns DESC"#,
        )
        .bind(provider_pubkey)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ProviderContractFeedback {
                contract_id: hex::encode(r.contract_id),
                provider_pubkey: hex::encode(r.provider_pubkey),
                service_matched_description: r.service_matched_description,
                would_rent_again: r.would_rent_again,
                created_at_ns: r.created_at_ns,
                contract_created_at_ns: r.contract_created_at_ns,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests;
