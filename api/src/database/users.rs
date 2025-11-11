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

    /// Update or create user profile
    pub async fn upsert_user_profile(
        &self,
        pubkey_hash: &[u8],
        display_name: Option<&str>,
        bio: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<()> {
        let updated_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        sqlx::query(
            "INSERT INTO user_profiles (pubkey_hash, display_name, bio, avatar_url, updated_at_ns)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(pubkey_hash) DO UPDATE SET
                 display_name = COALESCE(excluded.display_name, display_name),
                 bio = COALESCE(excluded.bio, bio),
                 avatar_url = COALESCE(excluded.avatar_url, avatar_url),
                 updated_at_ns = excluded.updated_at_ns",
        )
        .bind(pubkey_hash)
        .bind(display_name)
        .bind(bio)
        .bind(avatar_url)
        .bind(updated_at_ns)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Add or update user contact
    pub async fn upsert_user_contact(
        &self,
        pubkey_hash: &[u8],
        contact_type: &str,
        contact_value: &str,
        verified: bool,
    ) -> Result<()> {
        let created_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        sqlx::query(
            "INSERT INTO user_contacts (user_pubkey_hash, contact_type, contact_value, verified, created_at_ns)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(user_pubkey_hash, contact_type) DO UPDATE SET
                 contact_value = excluded.contact_value,
                 verified = excluded.verified",
        )
        .bind(pubkey_hash)
        .bind(contact_type)
        .bind(contact_value)
        .bind(verified)
        .bind(created_at_ns)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete user contact
    pub async fn delete_user_contact(&self, pubkey_hash: &[u8], contact_type: &str) -> Result<()> {
        sqlx::query("DELETE FROM user_contacts WHERE user_pubkey_hash = ? AND contact_type = ?")
            .bind(pubkey_hash)
            .bind(contact_type)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Add or update user social account
    pub async fn upsert_user_social(
        &self,
        pubkey_hash: &[u8],
        platform: &str,
        username: &str,
        profile_url: Option<&str>,
    ) -> Result<()> {
        let created_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        sqlx::query(
            "INSERT INTO user_socials (user_pubkey_hash, platform, username, profile_url, created_at_ns)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(user_pubkey_hash, platform) DO UPDATE SET
                 username = excluded.username,
                 profile_url = excluded.profile_url",
        )
        .bind(pubkey_hash)
        .bind(platform)
        .bind(username)
        .bind(profile_url)
        .bind(created_at_ns)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete user social account
    pub async fn delete_user_social(&self, pubkey_hash: &[u8], platform: &str) -> Result<()> {
        sqlx::query("DELETE FROM user_socials WHERE user_pubkey_hash = ? AND platform = ?")
            .bind(pubkey_hash)
            .bind(platform)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Add user public key
    pub async fn add_user_public_key(
        &self,
        pubkey_hash: &[u8],
        key_type: &str,
        key_data: &str,
        key_fingerprint: Option<&str>,
        label: Option<&str>,
    ) -> Result<()> {
        let created_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        sqlx::query(
            "INSERT INTO user_public_keys (user_pubkey_hash, key_type, key_data, key_fingerprint, label, created_at_ns)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(pubkey_hash)
        .bind(key_type)
        .bind(key_data)
        .bind(key_fingerprint)
        .bind(label)
        .bind(created_at_ns)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete user public key by fingerprint
    pub async fn delete_user_public_key(
        &self,
        pubkey_hash: &[u8],
        key_fingerprint: &str,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM user_public_keys WHERE user_pubkey_hash = ? AND key_fingerprint = ?",
        )
        .bind(pubkey_hash)
        .bind(key_fingerprint)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
