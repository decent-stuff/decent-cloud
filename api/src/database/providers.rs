use anyhow::Result;
use borsh::BorshDeserialize;
use dcc_common::{CheckInPayload, UpdateProfilePayload};
use provider_profile::Profile;
use super::types::{Database, LedgerEntryData};

impl Database {
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
            let check_in = CheckInPayload::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse check-in: {}", e))?;

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
