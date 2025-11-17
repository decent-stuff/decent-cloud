use super::types::{Database, LedgerEntryData};
use anyhow::Result;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use borsh::BorshDeserialize;
use dcc_common::{CheckInPayload, UpdateProfilePayload};
use poem_openapi::Object;
use provider_profile::Profile;
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

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(skip_serializing_if_is_none)]
pub struct Validator {
    #[ts(skip)]
    #[serde(skip_deserializing)]
    #[oai(skip)]
    pub pubkey: Vec<u8>,
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

impl Database {
    /// Get list of active providers (checked in recently)
    pub async fn get_active_providers(&self, days: i64) -> Result<Vec<ProviderProfile>> {
        let cutoff_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            - days.max(1) * 24 * 3600 * 1_000_000_000;

        let profiles = sqlx::query_as!(
            ProviderProfile,
            r#"SELECT DISTINCT p.pubkey, p.name, p.description, p.website_url, p.logo_url, p.why_choose_us, p.api_version, p.profile_version, p.updated_at_ns FROM provider_profiles p
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
            "SELECT pubkey, name, description, website_url, logo_url, why_choose_us, api_version, profile_version, updated_at_ns FROM provider_profiles WHERE pubkey = ?",
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
            "SELECT pubkey, name, description, website_url, logo_url, why_choose_us, api_version, profile_version, updated_at_ns FROM provider_profiles ORDER BY updated_at_ns DESC LIMIT ? OFFSET ?",
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
                r.pubkey,
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
            let nonce_signature = check_in.nonce_signature().clone();
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

    // Provider profiles
    pub(crate) async fn insert_provider_profiles(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let profile_payload = UpdateProfilePayload::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse profile payload: {}", e))?;
            let profile = profile_payload
                .deserialize_update_profile()
                .map_err(|e| anyhow::anyhow!("Failed to deserialize profile: {}", e))?;

            // Extract structured fields from profile based on ProfileV0_1_0 structure
            match profile {
                Profile::V0_1_0(profile_v0_1_0) => {
                    // Insert main profile record
                    let timestamp_i64 = entry.block_timestamp_ns as i64;
                    sqlx::query!(
                        "INSERT OR REPLACE INTO provider_profiles (pubkey, name, description, website_url, logo_url, why_choose_us, api_version, profile_version, updated_at_ns) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                        entry.key,
                        profile_v0_1_0.metadata.name,
                        profile_v0_1_0.spec.description,
                        profile_v0_1_0.spec.url,
                        profile_v0_1_0.spec.logo_url,
                        profile_v0_1_0.spec.why_choose_us,
                        profile_v0_1_0.api_version,
                        profile_v0_1_0.metadata.version,
                        timestamp_i64
                    )
                    .execute(&mut **tx)
                    .await?;

                    // Insert contact information in normalized table
                    for (contact_type, contact_value) in &profile_v0_1_0.spec.contacts {
                        sqlx::query!(
                            "INSERT OR REPLACE INTO provider_profiles_contacts (provider_pubkey, contact_type, contact_value) VALUES (?, ?, ?)",
                            entry.key,
                            contact_type,
                            contact_value
                        )
                        .execute(&mut **tx)
                        .await?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
