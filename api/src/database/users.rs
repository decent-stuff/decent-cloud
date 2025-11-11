use super::types::{Database, LedgerEntryData};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserProfile {
    pub pubkey_hash: Vec<u8>,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub updated_at_ns: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserContact {
    pub contact_type: String,
    pub contact_value: String,
    pub verified: bool,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserSocial {
    pub platform: String,
    pub username: String,
    pub profile_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserPublicKey {
    pub key_type: String,
    pub key_data: String,
    pub key_fingerprint: Option<String>,
    pub label: Option<String>,
}

impl Database {
    // User registrations
    pub(crate) async fn insert_user_registrations(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            // For now, store raw data since registration is just signature
            sqlx::query(
                "INSERT OR REPLACE INTO user_registrations (pubkey_hash, pubkey_bytes, signature, created_at_ns) VALUES (?, ?, ?, ?)"
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

    /// Get user profile by pubkey hash
    pub async fn get_user_profile(&self, pubkey_hash: &[u8]) -> Result<Option<UserProfile>> {
        let profile = sqlx::query_as::<_, UserProfile>(
            "SELECT pubkey_hash, display_name, bio, avatar_url, updated_at_ns FROM user_profiles WHERE pubkey_hash = ?",
        )
        .bind(pubkey_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(profile)
    }

    /// Get user contacts
    pub async fn get_user_contacts(&self, pubkey_hash: &[u8]) -> Result<Vec<UserContact>> {
        let contacts = sqlx::query_as::<_, UserContact>(
            "SELECT contact_type, contact_value, verified FROM user_contacts WHERE user_pubkey_hash = ?",
        )
        .bind(pubkey_hash)
        .fetch_all(&self.pool)
        .await?;

        Ok(contacts)
    }

    /// Get user social accounts
    pub async fn get_user_socials(&self, pubkey_hash: &[u8]) -> Result<Vec<UserSocial>> {
        let socials = sqlx::query_as::<_, UserSocial>(
            "SELECT platform, username, profile_url FROM user_socials WHERE user_pubkey_hash = ?",
        )
        .bind(pubkey_hash)
        .fetch_all(&self.pool)
        .await?;

        Ok(socials)
    }

    /// Get user public keys
    pub async fn get_user_public_keys(&self, pubkey_hash: &[u8]) -> Result<Vec<UserPublicKey>> {
        let keys = sqlx::query_as::<_, UserPublicKey>(
            "SELECT key_type, key_data, key_fingerprint, label FROM user_public_keys WHERE user_pubkey_hash = ?",
        )
        .bind(pubkey_hash)
        .fetch_all(&self.pool)
        .await?;

        Ok(keys)
    }
}
