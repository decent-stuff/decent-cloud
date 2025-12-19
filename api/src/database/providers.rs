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
    #[oai(skip_serializing_if_is_none)]
    pub description: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub website_url: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub logo_url: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub why_choose_us: Option<String>,
    pub api_version: String,
    pub profile_version: String,
    #[ts(type = "number")]
    pub updated_at_ns: i64,
    #[oai(skip_serializing_if_is_none)]
    pub support_email: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub support_hours: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub support_channels: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub regions: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub payment_methods: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub refund_policy: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub sla_guarantee: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub unique_selling_points: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub common_issues: Option<String>,
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

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Object)]
pub struct ProviderContact {
    pub contact_type: String,
    pub contact_value: String,
}

#[derive(Debug, Serialize, Deserialize, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(skip_serializing_if_is_none)]
pub struct ProviderOnboarding {
    #[oai(skip_serializing_if_is_none)]
    pub support_email: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub support_hours: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub support_channels: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub regions: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub payment_methods: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub refund_policy: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub sla_guarantee: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub unique_selling_points: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub common_issues: Option<String>,
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
    #[oai(skip_serializing_if_is_none)]
    pub name: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub description: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub website_url: Option<String>,
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
    #[oai(skip_serializing_if_is_none)]
    pub logo_url: Option<String>,
    pub data_source: String,
    #[ts(type = "number")]
    pub offerings_count: i64,
    #[ts(type = "number")]
    pub created_at_ns: i64,
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
             WHERE c.block_timestamp_ns > ?
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
            "SELECT pubkey, name, description, website_url, logo_url, why_choose_us, api_version, profile_version, updated_at_ns, support_email, support_hours, support_channels, regions, payment_methods, refund_policy, sla_guarantee, unique_selling_points, common_issues, onboarding_completed_at FROM provider_profiles WHERE pubkey = ?",
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
            "SELECT contact_type, contact_value FROM provider_profiles_contacts WHERE provider_pubkey = ?",
            pubkey
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(contacts)
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
             WHERE pubkey = ? ORDER BY block_timestamp_ns DESC LIMIT ?"#,
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
            "SELECT pubkey, name, description, website_url, logo_url, why_choose_us, api_version, profile_version, updated_at_ns, support_email, support_hours, support_channels, regions, payment_methods, refund_policy, sla_guarantee, unique_selling_points, common_issues, onboarding_completed_at FROM provider_profiles ORDER BY updated_at_ns DESC LIMIT ? OFFSET ?",
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
        let count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM provider_profiles")
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
            "SELECT support_email, support_hours, support_channels, regions, payment_methods, refund_policy, sla_guarantee, unique_selling_points, common_issues, onboarding_completed_at FROM provider_profiles WHERE pubkey = ?",
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
               ) VALUES (?, ?, 'v1', '1.0', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
                   account_id = COALESCE(account_id, excluded.account_id)"#,
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
                lower(hex(r.pubkey)) as "pubkey!: String",
                NULLIF(p.name, '') as "name?: String",
                NULLIF(p.description, '') as "description?: String",
                NULLIF(p.website_url, '') as "website_url?: String",
                NULLIF(p.logo_url, '') as "logo_url?: String",
                COUNT(DISTINCT c.block_timestamp_ns) as "total_check_ins!: i64",
                COALESCE(SUM(CASE WHEN c.block_timestamp_ns > ? THEN 1 ELSE 0 END), 0) as "check_ins_24h!: i64",
                COALESCE(SUM(CASE WHEN c.block_timestamp_ns > ? THEN 1 ELSE 0 END), 0) as "check_ins_7d!: i64",
                COALESCE(SUM(CASE WHEN c.block_timestamp_ns > ? THEN 1 ELSE 0 END), 0) as "check_ins_30d!: i64",
                MAX(c.block_timestamp_ns) as "last_check_in_ns!: i64",
                r.created_at_ns as "registered_at_ns!: i64"
             FROM provider_registrations r
             INNER JOIN provider_check_ins c ON r.pubkey = c.pubkey
             LEFT JOIN provider_profiles p ON r.pubkey = p.pubkey
             WHERE c.block_timestamp_ns > ?
             GROUP BY r.pubkey
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
                CAST(COUNT(po.id) AS INTEGER) as "offerings_count!: i64"
            FROM external_providers ep
            LEFT JOIN provider_offerings po ON ep.pubkey = po.pubkey AND po.offering_source = 'seeded'
            GROUP BY ep.pubkey
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

    /// Create or update an external provider (used by api-cli scraper)
    #[allow(dead_code)]
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
               VALUES (?, ?, ?, ?, ?, ?)
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
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            // Store raw Ed25519 public key (32 bytes) and signature
            let timestamp_i64 = entry.block_timestamp_ns as i64;
            sqlx::query!(
                "INSERT OR REPLACE INTO provider_registrations (pubkey, signature, created_at_ns) VALUES (?, ?, ?)",
                entry.key,
                entry.value,
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
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
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
                "INSERT INTO provider_check_ins (pubkey, memo, nonce_signature, block_timestamp_ns) VALUES (?, ?, ?, ?)",
                entry.key,
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
               SET chatwoot_inbox_id = ?, chatwoot_team_id = ?, chatwoot_portal_slug = ?
               WHERE pubkey = ?"#,
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
               FROM provider_profiles WHERE pubkey = ?"#,
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
            "SELECT auto_accept_rentals FROM provider_profiles WHERE pubkey = ?",
            pubkey
        )
        .fetch_optional(&self.pool)
        .await?;

        // row is Option<i64> - None if no row found, Some(value) if found
        Ok(row.unwrap_or(0) != 0)
    }

    /// Set provider auto-accept rentals setting.
    /// Updates the provider_profiles table. Returns error if provider doesn't exist.
    pub async fn set_provider_auto_accept_rentals(
        &self,
        pubkey: &[u8],
        enabled: bool,
    ) -> Result<()> {
        let value = if enabled { 1 } else { 0 };
        let result = sqlx::query!(
            "UPDATE provider_profiles SET auto_accept_rentals = ? WHERE pubkey = ?",
            value,
            pubkey
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!("Provider profile not found"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
