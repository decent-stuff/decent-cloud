use super::types::Database;
use anyhow::Result;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

mod auto_accept;
mod external;
mod sla;

#[cfg(test)]
mod tests;

/// Per-offering auto-accept rule for a provider.
///
/// When `auto_accept_rentals` is true, rules filter which offering+duration combos are auto-accepted.
/// If no rule exists for an offering, all requests for that offering are auto-accepted.
/// If a rule exists and is enabled, only requests within [min_duration_hours, max_duration_hours] match.
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AutoAcceptRule {
    #[ts(type = "number")]
    pub id: i64,
    pub offering_id: String,
    #[ts(type = "number | null")]
    pub min_duration_hours: Option<i64>,
    #[ts(type = "number | null")]
    pub max_duration_hours: Option<i64>,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(skip_serializing_if_is_none)]
pub struct ProviderProfile {
    #[ts(skip)]
    #[serde(skip_deserializing)]
    #[oai(skip)]
    pub pubkey: Vec<u8>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub website_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub logo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub why_choose_us: Option<String>,
    pub api_version: String,
    pub profile_version: String,
    #[ts(type = "number")]
    pub updated_at_ns: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub support_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub support_hours: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub support_channels: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub regions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub payment_methods: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub refund_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub sla_guarantee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub unique_selling_points: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub common_issues: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    #[ts(type = "number | null")]
    pub onboarding_completed_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
#[allow(dead_code)]
pub struct ProviderCheckIn {
    pub pubkey: Vec<u8>,
    pub memo: String,
    pub block_timestamp_ns: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(
    export,
    export_to = "../../website/src/lib/types/generated/",
    rename_all = "camelCase"
)]
#[serde(rename_all = "camelCase")]
#[oai(rename_all = "camelCase")]
pub struct ProviderContact {
    #[ts(type = "number")]
    pub id: i64,
    pub contact_type: String,
    pub contact_value: String,
}

#[derive(Debug, Serialize, Deserialize, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(skip_serializing_if_is_none)]
pub struct ProviderOnboarding {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub support_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub support_hours: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub support_channels: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub regions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub payment_methods: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub refund_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub sla_guarantee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub unique_selling_points: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub common_issues: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    #[ts(type = "number | null")]
    pub onboarding_completed_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(skip_serializing_if_is_none)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct Validator {
    pub pubkey: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub website_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub logo_url: Option<String>,
    #[ts(type = "number")]
    pub total_check_ins: i64,
    #[ts(type = "number")]
    pub check_ins_24h: i64,
    #[ts(type = "number")]
    pub check_ins_7d: i64,
    #[ts(type = "number")]
    pub check_ins_30d: i64,
    #[ts(type = "number")]
    pub last_check_in_ns: i64,
    #[ts(type = "number")]
    pub registered_at_ns: i64,
}

#[derive(Debug, Serialize, Deserialize, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(skip_serializing_if_is_none)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ExternalProvider {
    pub pubkey: String,
    pub name: String,
    pub domain: String,
    pub website_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub logo_url: Option<String>,
    pub data_source: String,
    #[ts(type = "number")]
    pub offerings_count: i64,
    #[ts(type = "number")]
    pub created_at_ns: i64,
}

/// A recently joined provider with offering count and days-since-join.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
pub struct NewProvider {
    pub pubkey: String,
    pub name: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub trust_score: Option<i64>,
    pub offerings_count: i64,
    pub joined_days_ago: i64,
}

/// Uptime SLA configuration for a provider (from provider_sla_config).
#[derive(Debug, Serialize, Deserialize, poem_openapi::Object, ts_rs::TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct SlaUptimeConfig {
    /// Minimum acceptable uptime percentage (0–100)
    pub uptime_threshold_percent: i32,
    /// Rolling window (in hours) over which uptime is measured
    pub sla_alert_window_hours: i32,
}

/// Info about a contract that has breached its uptime SLA.
#[derive(Debug, Serialize, Deserialize)]
pub struct SlaBreachInfo {
    /// Hex-encoded contract ID
    pub contract_id: String,
    /// Hex-encoded provider pubkey
    pub provider_pubkey: String,
    /// Measured uptime percentage
    pub uptime_percent: i32,
    /// Threshold that was breached
    pub threshold_percent: i32,
}

/// Raw SLA config row (includes provider_pubkey as bytes for background service use).
#[derive(Debug)]
pub struct ProviderSlaRow {
    pub provider_pubkey: Vec<u8>,
    pub uptime_threshold_percent: i32,
    pub sla_alert_window_hours: i32,
}

impl Database {
    /// Get list of active providers (checked in recently)
    pub async fn get_active_providers(&self, days: i64) -> Result<Vec<ProviderProfile>> {
        let cutoff_ns = crate::now_ns()? - days.max(1) * 24 * 3600 * 1_000_000_000;

        let profiles = sqlx::query_as!(
            ProviderProfile,
            r#"SELECT DISTINCT p.pubkey, p.name, p.description, p.website_url, p.logo_url, p.why_choose_us, p.api_version, p.profile_version, p.updated_at_ns, p.support_email, p.support_hours, p.support_channels, p.regions, p.payment_methods, p.refund_policy, p.sla_guarantee, p.unique_selling_points, p.common_issues, p.onboarding_completed_at FROM provider_profiles p
             INNER JOIN provider_check_ins c ON p.pubkey = c.pubkey
             WHERE c.block_timestamp_ns > $1
             ORDER BY p.name"#,
            cutoff_ns
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(profiles)
    }

    /// Get provider profile by pubkey
    pub async fn get_provider_profile(&self, pubkey: &[u8]) -> Result<Option<ProviderProfile>> {
        let profile = sqlx::query_as!(
            ProviderProfile,
            "SELECT pubkey, name, description, website_url, logo_url, why_choose_us, api_version, profile_version, updated_at_ns, support_email, support_hours, support_channels, regions, payment_methods, refund_policy, sla_guarantee, unique_selling_points, common_issues, onboarding_completed_at FROM provider_profiles WHERE pubkey = $1",
            pubkey
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(profile)
    }

    /// Get provider contacts
    pub async fn get_provider_contacts(&self, pubkey: &[u8]) -> Result<Vec<ProviderContact>> {
        let contacts = sqlx::query_as!(
            ProviderContact,
            r#"SELECT id as "id!", contact_type, contact_value FROM provider_profiles_contacts WHERE provider_pubkey = $1"#,
            pubkey
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(contacts)
    }

    /// Add provider contact
    pub async fn add_provider_contact(
        &self,
        provider_pubkey: &[u8],
        contact_type: &str,
        contact_value: &str,
    ) -> Result<()> {
        sqlx::query!(
            "INSERT INTO provider_profiles_contacts (provider_pubkey, contact_type, contact_value) VALUES ($1, $2, $3)",
            provider_pubkey,
            contact_type,
            contact_value
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete provider contact by ID
    pub async fn delete_provider_contact(
        &self,
        provider_pubkey: &[u8],
        contact_id: i64,
    ) -> Result<()> {
        sqlx::query!(
            "DELETE FROM provider_profiles_contacts WHERE provider_pubkey = $1 AND id = $2",
            provider_pubkey,
            contact_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get recent check-ins for a provider
    #[allow(dead_code)]
    pub async fn get_provider_check_ins(
        &self,
        pubkey: &[u8],
        limit: i64,
    ) -> Result<Vec<ProviderCheckIn>> {
        let check_ins = sqlx::query_as!(
            ProviderCheckIn,
            r#"SELECT pubkey, memo, block_timestamp_ns FROM provider_check_ins
             WHERE pubkey = $1 ORDER BY block_timestamp_ns DESC LIMIT $2"#,
            pubkey,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(check_ins)
    }

    /// Get all providers with profiles
    pub async fn list_providers(&self, limit: i64, offset: i64) -> Result<Vec<ProviderProfile>> {
        let profiles = sqlx::query_as!(
            ProviderProfile,
            "SELECT pubkey, name, description, website_url, logo_url, why_choose_us, api_version, profile_version, updated_at_ns, support_email, support_hours, support_channels, regions, payment_methods, refund_policy, sla_guarantee, unique_selling_points, common_issues, onboarding_completed_at FROM provider_profiles ORDER BY updated_at_ns DESC LIMIT $1 OFFSET $2",
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(profiles)
    }

    /// Count total providers
    #[allow(dead_code)]
    pub async fn count_providers(&self) -> Result<i64> {
        let count: i64 =
            sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM provider_profiles"#)
                .fetch_one(&self.pool)
                .await?;

        Ok(count)
    }

    /// Get provider onboarding data
    pub async fn get_provider_onboarding(
        &self,
        pubkey: &[u8],
    ) -> Result<Option<ProviderOnboarding>> {
        let onboarding = sqlx::query_as!(
            ProviderOnboarding,
            "SELECT support_email, support_hours, support_channels, regions, payment_methods, refund_policy, sla_guarantee, unique_selling_points, common_issues, onboarding_completed_at FROM provider_profiles WHERE pubkey = $1",
            pubkey
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(onboarding)
    }

    /// Update or create provider onboarding data (upsert).
    /// If the provider profile doesn't exist, creates one with the given name.
    /// Also ensures account_id is set (creates account if needed).
    pub async fn update_provider_onboarding(
        &self,
        pubkey: &[u8],
        data: &ProviderOnboarding,
        provider_name: &str,
    ) -> Result<()> {
        let now_ns = crate::now_ns()?;

        // Ensure account exists for this pubkey
        let account_id = self.ensure_account_for_pubkey(pubkey).await?;

        sqlx::query!(
            r#"INSERT INTO provider_profiles (
                   pubkey, name, api_version, profile_version, updated_at_ns,
                   support_email, support_hours, support_channels, regions,
                   payment_methods, refund_policy, sla_guarantee,
                   unique_selling_points, common_issues, onboarding_completed_at,
                   account_id
               ) VALUES ($1, $2, 'v1', '1.0', $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
               ON CONFLICT(pubkey) DO UPDATE SET
                   support_email = excluded.support_email,
                   support_hours = excluded.support_hours,
                   support_channels = excluded.support_channels,
                   regions = excluded.regions,
                   payment_methods = excluded.payment_methods,
                   refund_policy = excluded.refund_policy,
                   sla_guarantee = excluded.sla_guarantee,
                   unique_selling_points = excluded.unique_selling_points,
                   common_issues = excluded.common_issues,
                   onboarding_completed_at = excluded.onboarding_completed_at,
                   updated_at_ns = excluded.updated_at_ns,
                   account_id = COALESCE(provider_profiles.account_id, excluded.account_id)"#,
            pubkey,
            provider_name,
            now_ns,
            data.support_email,
            data.support_hours,
            data.support_channels,
            data.regions,
            data.payment_methods,
            data.refund_policy,
            data.sla_guarantee,
            data.unique_selling_points,
            data.common_issues,
            now_ns,
            account_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get recently joined providers (joined within 90 days) that have at least one public offering.
    /// `limit` is capped at 10.
    pub async fn get_new_providers(&self, limit: i64) -> Result<Vec<NewProvider>> {
        let limit = limit.min(10);
        let rows = sqlx::query_as::<_, NewProvider>(
            r#"SELECT
                lower(encode(pp.pubkey, 'hex')) AS pubkey,
                pp.name,
                pp.description,
                pp.logo_url,
                pp.trust_score,
                COUNT(po.id)::BIGINT AS offerings_count,
                EXTRACT(DAY FROM NOW() - pp.created_at)::BIGINT AS joined_days_ago
            FROM provider_profiles pp
            LEFT JOIN provider_offerings po ON po.pubkey = pp.pubkey
                AND po.is_draft = false
                AND po.visibility = 'public'
            WHERE pp.created_at >= NOW() - INTERVAL '90 days'
              AND pp.has_critical_flags = false
            GROUP BY pp.pubkey, pp.name, pp.description, pp.logo_url, pp.trust_score, pp.created_at
            HAVING COUNT(po.id) > 0
            ORDER BY pp.created_at DESC
            LIMIT $1"#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
