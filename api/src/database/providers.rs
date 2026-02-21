use super::types::{Database, LedgerEntryData};
use anyhow::Result;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use borsh::BorshDeserialize;
use dcc_common::CheckInPayload;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

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
        let cutoff_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            - days.max(1) * 24 * 3600 * 1_000_000_000;

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
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

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

    /// Get list of active validators (checked in recently, with or without profiles)
    pub async fn get_active_validators(&self, days: i64) -> Result<Vec<Validator>> {
        let cutoff_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            - days.max(1) * 24 * 3600 * 1_000_000_000;
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let cutoff_24h = now_ns - 24 * 3600 * 1_000_000_000;
        let cutoff_7d = now_ns - 7 * 24 * 3600 * 1_000_000_000;
        let cutoff_30d = now_ns - 30 * 24 * 3600 * 1_000_000_000;

        let validators = sqlx::query_as!(
            Validator,
            r#"SELECT
                lower(encode(r.pubkey, 'hex')) as "pubkey!: String",
                NULLIF(p.name, '') as "name: String",
                NULLIF(p.description, '') as "description: String",
                NULLIF(p.website_url, '') as "website_url: String",
                NULLIF(p.logo_url, '') as "logo_url: String",
                COUNT(DISTINCT c.block_timestamp_ns) as "total_check_ins!: i64",
                COALESCE(SUM(CASE WHEN c.block_timestamp_ns > $1 THEN 1 ELSE 0 END), 0) as "check_ins_24h!: i64",
                COALESCE(SUM(CASE WHEN c.block_timestamp_ns > $2 THEN 1 ELSE 0 END), 0) as "check_ins_7d!: i64",
                COALESCE(SUM(CASE WHEN c.block_timestamp_ns > $3 THEN 1 ELSE 0 END), 0) as "check_ins_30d!: i64",
                MAX(c.block_timestamp_ns) as "last_check_in_ns!: i64",
                r.created_at_ns as "registered_at_ns!: i64"
             FROM provider_registrations r
             INNER JOIN provider_check_ins c ON r.pubkey = c.pubkey
             LEFT JOIN provider_profiles p ON r.pubkey = p.pubkey
             WHERE c.block_timestamp_ns > $4
             GROUP BY r.pubkey, r.created_at_ns, p.name, p.description, p.website_url, p.logo_url
             ORDER BY MAX(c.block_timestamp_ns) DESC"#,
            cutoff_24h,
            cutoff_7d,
            cutoff_30d,
            cutoff_ns
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(validators)
    }

    /// List external providers with offering counts
    pub async fn list_external_providers(&self) -> Result<Vec<ExternalProvider>> {
        let rows = sqlx::query!(
            r#"SELECT
                ep.pubkey,
                ep.name,
                ep.domain,
                ep.website_url,
                ep.logo_url,
                ep.data_source,
                ep.created_at_ns,
                CAST(COUNT(po.id) AS BIGINT) as "offerings_count!: i64"
            FROM external_providers ep
            LEFT JOIN provider_offerings po ON ep.pubkey = po.pubkey AND po.offering_source = 'seeded'
            GROUP BY ep.pubkey, ep.name, ep.domain, ep.website_url, ep.logo_url, ep.data_source, ep.created_at_ns
            ORDER BY ep.name"#
        )
        .fetch_all(&self.pool)
        .await?;

        let providers = rows
            .into_iter()
            .map(|row| ExternalProvider {
                pubkey: hex::encode(&row.pubkey),
                name: row.name,
                domain: row.domain,
                website_url: row.website_url,
                logo_url: row.logo_url,
                data_source: row.data_source,
                offerings_count: row.offerings_count,
                created_at_ns: row.created_at_ns,
            })
            .collect();

        Ok(providers)
    }

    /// Create or update an external provider.
    /// Used by: `api-cli scrape-provider` command
    pub async fn create_or_update_external_provider(
        &self,
        pubkey: &[u8],
        name: &str,
        domain: &str,
        website_url: &str,
        data_source: &str,
    ) -> Result<()> {
        let created_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        sqlx::query!(
            r#"INSERT INTO external_providers (pubkey, name, domain, website_url, data_source, created_at_ns)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT(pubkey) DO UPDATE SET
                   name = excluded.name,
                   domain = excluded.domain,
                   website_url = excluded.website_url,
                   data_source = excluded.data_source"#,
            pubkey,
            name,
            domain,
            website_url,
            data_source,
            created_at_ns
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Provider registrations
    pub(crate) async fn insert_provider_registrations(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            // Store raw Ed25519 public key (32 bytes) and signature
            let timestamp_i64 = entry.block_timestamp_ns as i64;
            sqlx::query!(
                "INSERT INTO provider_registrations (pubkey, signature, created_at_ns) VALUES ($1, $2, $3) ON CONFLICT (pubkey) DO UPDATE SET signature = excluded.signature, created_at_ns = excluded.created_at_ns",
                &entry.key,
                &entry.value,
                timestamp_i64
            )
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Provider check-ins
    pub(crate) async fn insert_provider_check_ins(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let check_in = match CheckInPayload::try_from_slice(&entry.value) {
                Ok(check_in) => check_in,
                Err(e) => {
                    if entry.value.len() == 64 {
                        // Earlier versions of the protocol stored the nonce signature directly
                        CheckInPayload::new(String::new(), entry.value.clone())
                    } else {
                        tracing::error!(
                            "Failed to parse check-in: {}. Payload: {} len {}",
                            e,
                            BASE64.encode(&entry.value),
                            entry.value.len()
                        );
                        continue;
                    }
                }
            };

            let timestamp_i64 = entry.block_timestamp_ns as i64;
            let memo = check_in.memo().to_string();
            let nonce_signature = check_in.nonce_signature();
            sqlx::query!(
                "INSERT INTO provider_check_ins (pubkey, memo, nonce_signature, block_timestamp_ns) VALUES ($1, $2, $3, $4)",
                &entry.key,
                memo,
                nonce_signature,
                timestamp_i64
            )
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    /// Store Chatwoot resources created for a provider during onboarding.
    pub async fn set_provider_chatwoot_resources(
        &self,
        pubkey: &[u8],
        inbox_id: u32,
        team_id: u32,
        portal_slug: &str,
    ) -> Result<()> {
        let inbox_id = inbox_id as i64;
        let team_id = team_id as i64;
        sqlx::query!(
            r#"UPDATE provider_profiles
               SET chatwoot_inbox_id = $1, chatwoot_team_id = $2, chatwoot_portal_slug = $3
               WHERE pubkey = $4"#,
            inbox_id,
            team_id,
            portal_slug,
            pubkey
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get Chatwoot resources for a provider.
    /// Returns (inbox_id, team_id, portal_slug) if set.
    pub async fn get_provider_chatwoot_resources(
        &self,
        pubkey: &[u8],
    ) -> Result<Option<(u32, u32, String)>> {
        let row = sqlx::query!(
            r#"SELECT chatwoot_inbox_id, chatwoot_team_id, chatwoot_portal_slug
               FROM provider_profiles WHERE pubkey = $1"#,
            pubkey
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.and_then(|r| {
            match (
                r.chatwoot_inbox_id,
                r.chatwoot_team_id,
                r.chatwoot_portal_slug,
            ) {
                (Some(inbox), Some(team), Some(slug)) => Some((inbox as u32, team as u32, slug)),
                _ => None,
            }
        }))
    }

    /// Check if provider has auto-accept rentals enabled.
    /// Returns false if provider profile doesn't exist or auto_accept_rentals is not set.
    pub async fn get_provider_auto_accept_rentals(&self, pubkey: &[u8]) -> Result<bool> {
        let row = sqlx::query_scalar!(
            "SELECT auto_accept_rentals FROM provider_profiles WHERE pubkey = $1",
            pubkey
        )
        .fetch_optional(&self.pool)
        .await?;

        // row is Option<bool> - None if no row found, Some(value) if found
        Ok(row.unwrap_or(false))
    }

    /// Set provider auto-accept rentals setting.
    /// Updates the provider_profiles table. Returns error if provider doesn't exist.
    pub async fn set_provider_auto_accept_rentals(
        &self,
        pubkey: &[u8],
        enabled: bool,
    ) -> Result<()> {
        let result = sqlx::query!(
            "UPDATE provider_profiles SET auto_accept_rentals = $1 WHERE pubkey = $2",
            enabled,
            pubkey
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!("Provider profile not found"));
        }

        Ok(())
    }

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
        let now_ns = chrono::Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or(0);
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
        let now_ns = chrono::Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or(0);
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
        let now_ns = chrono::Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or(0);
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

#[cfg(test)]
mod tests;
