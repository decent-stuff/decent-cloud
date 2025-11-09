use super::types::{Database, LedgerEntryData};
use anyhow::Result;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use borsh::BorshDeserialize;
use dcc_common::{CheckInPayload, UpdateProfilePayload};
use provider_profile::Profile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProviderProfile {
    pub pubkey_hash: Vec<u8>,
    pub name: String,
    pub description: Option<String>,
    pub website_url: Option<String>,
    pub logo_url: Option<String>,
    pub why_choose_us: Option<String>,
    pub api_version: String,
    pub profile_version: String,
    pub updated_at_ns: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProviderCheckIn {
    pub pubkey_hash: Vec<u8>,
    pub memo: String,
    pub block_timestamp_ns: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProviderContact {
    pub contact_type: String,
    pub contact_value: String,
}

impl Database {
    /// Get list of active providers (checked in recently)
    pub async fn get_active_providers(&self, days: i64) -> Result<Vec<ProviderProfile>> {
        let cutoff_ns = (chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            - days.max(1) * 24 * 3600 * 1_000_000_000) as i64;

        let profiles = sqlx::query_as::<_, ProviderProfile>(
            "SELECT DISTINCT p.* FROM provider_profiles p
             INNER JOIN provider_check_ins c ON p.pubkey_hash = c.pubkey_hash
             WHERE c.block_timestamp_ns > ?
             ORDER BY p.name",
        )
        .bind(cutoff_ns)
        .fetch_all(&self.pool)
        .await?;

        Ok(profiles)
    }

    /// Get provider profile by pubkey hash
    pub async fn get_provider_profile(
        &self,
        pubkey_hash: &[u8],
    ) -> Result<Option<ProviderProfile>> {
        let profile = sqlx::query_as::<_, ProviderProfile>(
            "SELECT * FROM provider_profiles WHERE pubkey_hash = ?",
        )
        .bind(pubkey_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(profile)
    }

    /// Get provider contacts
    pub async fn get_provider_contacts(&self, pubkey_hash: &[u8]) -> Result<Vec<ProviderContact>> {
        let contacts = sqlx::query_as::<_, ProviderContact>(
            "SELECT contact_type, contact_value FROM provider_profiles_contacts WHERE provider_pubkey_hash = ?"
        )
        .bind(pubkey_hash)
        .fetch_all(&self.pool)
        .await?;

        Ok(contacts)
    }

    /// Get recent check-ins for a provider
    pub async fn get_provider_check_ins(
        &self,
        pubkey_hash: &[u8],
        limit: i64,
    ) -> Result<Vec<ProviderCheckIn>> {
        let check_ins = sqlx::query_as::<_, ProviderCheckIn>(
            "SELECT pubkey_hash, memo, block_timestamp_ns FROM provider_check_ins
             WHERE pubkey_hash = ? ORDER BY block_timestamp_ns DESC LIMIT ?",
        )
        .bind(pubkey_hash)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(check_ins)
    }

    /// Get all providers with profiles
    pub async fn list_providers(&self, limit: i64, offset: i64) -> Result<Vec<ProviderProfile>> {
        let profiles = sqlx::query_as::<_, ProviderProfile>(
            "SELECT * FROM provider_profiles ORDER BY updated_at_ns DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(profiles)
    }

    /// Count total providers
    pub async fn count_providers(&self) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM provider_profiles")
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0)
    }

    // Provider registrations
    pub(crate) async fn insert_provider_registrations(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            // For now, store raw data since registration is just signature
            sqlx::query(
                "INSERT OR REPLACE INTO provider_registrations (pubkey_hash, pubkey_bytes, signature, created_at_ns) VALUES (?, ?, ?, ?)"
            )
            .bind(&entry.key)
            .bind(&entry.key)
            .bind(&entry.value) // Store signature directly
            .bind(entry.block_timestamp_ns as i64)
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

            sqlx::query(
                "INSERT INTO provider_check_ins (pubkey_hash, memo, nonce_signature, block_timestamp_ns) VALUES (?, ?, ?, ?)"
            )
            .bind(&entry.key)
            .bind(check_in.memo())
            .bind(check_in.nonce_signature())
            .bind(entry.block_timestamp_ns as i64)
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
                    sqlx::query(
                        "INSERT OR REPLACE INTO provider_profiles (pubkey_hash, name, description, website_url, logo_url, why_choose_us, api_version, profile_version, updated_at_ns) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
                    )
                    .bind(&entry.key)
                    .bind(&profile_v0_1_0.metadata.name)
                    .bind(&profile_v0_1_0.spec.description)
                    .bind(&profile_v0_1_0.spec.url)
                    .bind(&profile_v0_1_0.spec.logo_url)
                    .bind(&profile_v0_1_0.spec.why_choose_us)
                    .bind(&profile_v0_1_0.api_version)
                    .bind(&profile_v0_1_0.metadata.version)
                    .bind(entry.block_timestamp_ns as i64)
                    .execute(&mut **tx)
                    .await?;

                    // Insert contact information in normalized table
                    for (contact_type, contact_value) in &profile_v0_1_0.spec.contacts {
                        sqlx::query(
                            "INSERT OR REPLACE INTO provider_profiles_contacts (provider_pubkey_hash, contact_type, contact_value) VALUES (?, ?, ?)"
                        )
                        .bind(&entry.key)
                        .bind(contact_type)
                        .bind(contact_value)
                        .execute(&mut **tx)
                        .await?;
                    }
                }
            }
        }
        Ok(())
    }
}
