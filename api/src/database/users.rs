use super::types::{Database, LedgerEntryData};
use anyhow::Result;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(skip_serializing_if_is_none)]
pub struct UserProfile {
    #[ts(skip)]
    #[serde(skip_deserializing)]
    #[oai(skip)]
    pub pubkey: Vec<u8>,
    #[oai(skip_serializing_if_is_none)]
    pub display_name: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub bio: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub avatar_url: Option<String>,
    #[ts(type = "number")]
    pub updated_at_ns: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
pub struct UserContact {
    #[ts(type = "number")]
    pub id: i64,
    pub contact_type: String,
    pub contact_value: String,
    pub verified: bool,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(skip_serializing_if_is_none)]
pub struct UserSocial {
    #[ts(type = "number")]
    pub id: i64,
    pub platform: String,
    pub username: String,
    #[oai(skip_serializing_if_is_none)]
    pub profile_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(skip_serializing_if_is_none)]
pub struct UserPublicKey {
    #[ts(type = "number")]
    pub id: i64,
    pub key_type: String,
    pub key_data: String,
    #[oai(skip_serializing_if_is_none)]
    pub key_fingerprint: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub label: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
pub struct UserActivity {
    pub offerings_provided: Vec<crate::database::offerings::Offering>,
    pub rentals_as_requester: Vec<crate::database::contracts::Contract>,
    pub rentals_as_provider: Vec<crate::database::contracts::Contract>,
}

impl Database {
    // User registrations
    pub(crate) async fn insert_user_registrations(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            // Store raw Ed25519 public key (32 bytes) and signature
            sqlx::query(
                "INSERT OR REPLACE INTO user_registrations (pubkey, signature, created_at_ns) VALUES (?, ?, ?)"
            )
            .bind(&entry.key) // Raw Ed25519 public key
            .bind(&entry.value) // Signature
            .bind(entry.block_timestamp_ns as i64)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    /// Ensure user registration exists in database (creates minimal entry if needed)
    async fn ensure_user_registered(&self, pubkey: &[u8]) -> Result<()> {
        let created_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let empty_sig: &[u8] = &[]; // Empty signature for web-created registrations

        sqlx::query!(
            "INSERT OR IGNORE INTO user_registrations (pubkey, signature, created_at_ns)
             VALUES (?, ?, ?)",
            pubkey,
            empty_sig,
            created_at_ns
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get user profile by pubkey
    pub async fn get_user_profile(&self, pubkey: &[u8]) -> Result<Option<UserProfile>> {
        let profile = sqlx::query_as!(
            UserProfile,
            "SELECT pubkey, display_name, bio, avatar_url, updated_at_ns FROM user_profiles WHERE pubkey = ?",
            pubkey
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(profile)
    }

    /// Get user contacts
    pub async fn get_user_contacts(&self, pubkey: &[u8]) -> Result<Vec<UserContact>> {
        let contacts = sqlx::query_as!(
            UserContact,
            r#"SELECT id as "id!", contact_type, contact_value, verified as "verified!" FROM user_contacts WHERE user_pubkey = ?"#,
            pubkey
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(contacts)
    }

    /// Get user social accounts
    pub async fn get_user_socials(&self, pubkey: &[u8]) -> Result<Vec<UserSocial>> {
        let socials = sqlx::query_as::<_, UserSocial>(
            "SELECT id, platform, username, profile_url FROM user_socials WHERE user_pubkey = ?",
        )
        .bind(pubkey)
        .fetch_all(&self.pool)
        .await?;

        Ok(socials)
    }

    /// Get user public keys
    pub async fn get_user_public_keys(&self, pubkey: &[u8]) -> Result<Vec<UserPublicKey>> {
        let keys = sqlx::query_as::<_, UserPublicKey>(
            "SELECT id, key_type, key_data, key_fingerprint, label FROM user_public_keys WHERE user_pubkey = ?",
        )
        .bind(pubkey)
        .fetch_all(&self.pool)
        .await?;

        Ok(keys)
    }

    /// Update or create user profile
    pub async fn upsert_user_profile(
        &self,
        pubkey: &[u8],
        display_name: Option<&str>,
        bio: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<()> {
        self.ensure_user_registered(pubkey).await?;

        let updated_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        sqlx::query(
            "INSERT INTO user_profiles (pubkey, display_name, bio, avatar_url, updated_at_ns)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(pubkey) DO UPDATE SET
                 display_name = COALESCE(excluded.display_name, display_name),
                 bio = COALESCE(excluded.bio, bio),
                 avatar_url = COALESCE(excluded.avatar_url, avatar_url),
                 updated_at_ns = excluded.updated_at_ns",
        )
        .bind(pubkey)
        .bind(display_name)
        .bind(bio)
        .bind(avatar_url)
        .bind(updated_at_ns)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Add user contact
    pub async fn upsert_user_contact(
        &self,
        pubkey: &[u8],
        contact_type: &str,
        contact_value: &str,
        verified: bool,
    ) -> Result<()> {
        self.ensure_user_registered(pubkey).await?;

        let created_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        sqlx::query(
            "INSERT INTO user_contacts (user_pubkey, contact_type, contact_value, verified, created_at_ns)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(pubkey)
        .bind(contact_type)
        .bind(contact_value)
        .bind(verified)
        .bind(created_at_ns)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete user contact by ID
    pub async fn delete_user_contact(&self, pubkey: &[u8], contact_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM user_contacts WHERE user_pubkey = ? AND id = ?")
            .bind(pubkey)
            .bind(contact_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Add user social account
    pub async fn upsert_user_social(
        &self,
        pubkey: &[u8],
        platform: &str,
        username: &str,
        profile_url: Option<&str>,
    ) -> Result<()> {
        self.ensure_user_registered(pubkey).await?;

        let created_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        sqlx::query(
            "INSERT INTO user_socials (user_pubkey, platform, username, profile_url, created_at_ns)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(pubkey)
        .bind(platform)
        .bind(username)
        .bind(profile_url)
        .bind(created_at_ns)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete user social account by ID
    pub async fn delete_user_social(&self, pubkey: &[u8], social_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM user_socials WHERE user_pubkey = ? AND id = ?")
            .bind(pubkey)
            .bind(social_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Add user public key
    pub async fn add_user_public_key(
        &self,
        pubkey: &[u8],
        key_type: &str,
        key_data: &str,
        key_fingerprint: Option<&str>,
        label: Option<&str>,
    ) -> Result<()> {
        self.ensure_user_registered(pubkey).await?;

        let created_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        sqlx::query(
            "INSERT INTO user_public_keys (user_pubkey, key_type, key_data, key_fingerprint, label, created_at_ns)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(pubkey)
        .bind(key_type)
        .bind(key_data)
        .bind(key_fingerprint)
        .bind(label)
        .bind(created_at_ns)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete user public key by ID
    pub async fn delete_user_public_key(&self, pubkey: &[u8], key_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM user_public_keys WHERE user_pubkey = ? AND id = ?")
            .bind(pubkey)
            .bind(key_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get all user activity: offerings provided and rentals (as both requester and provider)
    pub async fn get_user_activity(&self, pubkey: &[u8]) -> Result<UserActivity> {
        let offerings_provided = self.get_provider_offerings(pubkey).await?;
        let rentals_as_requester = self.get_user_contracts(pubkey).await?;
        let rentals_as_provider = self.get_provider_contracts(pubkey).await?;

        Ok(UserActivity {
            offerings_provided,
            rentals_as_requester,
            rentals_as_provider,
        })
    }
}

#[cfg(test)]
mod tests;
