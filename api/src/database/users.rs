use super::types::{Database, LedgerEntryData};
use anyhow::Result;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Contact information for an account (account_contacts table)
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(
    export,
    export_to = "../../website/src/lib/types/generated/",
    rename_all = "camelCase"
)]
#[serde(rename_all = "camelCase")]
#[oai(rename_all = "camelCase")]
pub struct AccountContact {
    #[ts(type = "number")]
    pub id: i64,
    pub contact_type: String,
    pub contact_value: String,
    pub verified: bool,
}

/// Social media account for an account (account_socials table)
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(
    export,
    export_to = "../../website/src/lib/types/generated/",
    rename_all = "camelCase"
)]
#[serde(rename_all = "camelCase")]
#[oai(rename_all = "camelCase")]
pub struct AccountSocial {
    #[ts(type = "number")]
    pub id: i64,
    pub platform: String,
    pub username: String,
    #[oai(skip_serializing_if_is_none)]
    pub profile_url: Option<String>,
}

/// External public key (SSH/GPG) for an account (account_external_keys table)
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(
    export,
    export_to = "../../website/src/lib/types/generated/",
    rename_all = "camelCase"
)]
#[serde(rename_all = "camelCase")]
#[oai(rename_all = "camelCase")]
pub struct AccountExternalKey {
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
    // User registrations (blockchain-based, keyed by pubkey)
    pub(crate) async fn insert_user_registrations(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let pubkey = entry.key.clone();
            let signature = entry.value.clone();
            let created_at_ns = entry.block_timestamp_ns as i64;
            sqlx::query!(
                "INSERT OR REPLACE INTO user_registrations (pubkey, signature, created_at_ns) VALUES (?, ?, ?)",
                pubkey,
                signature,
                created_at_ns
            )
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    /// Get user activity by pubkey (blockchain-based)
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

    // ===== ACCOUNT-BASED METHODS =====

    /// Get account contacts by account ID
    pub async fn get_account_contacts(&self, account_id: &[u8]) -> Result<Vec<AccountContact>> {
        let contacts = sqlx::query_as!(
            AccountContact,
            r#"SELECT id as "id!", contact_type, contact_value, verified as "verified!"
               FROM account_contacts
               WHERE account_id = ?"#,
            account_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(contacts)
    }

    /// Add account contact
    pub async fn add_account_contact(
        &self,
        account_id: &[u8],
        contact_type: &str,
        contact_value: &str,
        verified: bool,
    ) -> Result<()> {
        let created_at = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        sqlx::query!(
            "INSERT INTO account_contacts (account_id, contact_type, contact_value, verified, created_at)
             VALUES (?, ?, ?, ?, ?)",
            account_id,
            contact_type,
            contact_value,
            verified,
            created_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete account contact by ID
    pub async fn delete_account_contact(&self, account_id: &[u8], contact_id: i64) -> Result<()> {
        sqlx::query!(
            "DELETE FROM account_contacts WHERE account_id = ? AND id = ?",
            account_id,
            contact_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get account socials by account ID
    pub async fn get_account_socials(&self, account_id: &[u8]) -> Result<Vec<AccountSocial>> {
        let socials = sqlx::query_as!(
            AccountSocial,
            r#"SELECT id as "id!", platform, username, profile_url
               FROM account_socials
               WHERE account_id = ?"#,
            account_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(socials)
    }

    /// Add account social
    pub async fn add_account_social(
        &self,
        account_id: &[u8],
        platform: &str,
        username: &str,
        profile_url: Option<&str>,
    ) -> Result<()> {
        let created_at = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        sqlx::query!(
            "INSERT INTO account_socials (account_id, platform, username, profile_url, created_at)
             VALUES (?, ?, ?, ?, ?)",
            account_id,
            platform,
            username,
            profile_url,
            created_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete account social by ID
    pub async fn delete_account_social(&self, account_id: &[u8], social_id: i64) -> Result<()> {
        sqlx::query!(
            "DELETE FROM account_socials WHERE account_id = ? AND id = ?",
            account_id,
            social_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get account external keys by account ID
    pub async fn get_account_external_keys(
        &self,
        account_id: &[u8],
    ) -> Result<Vec<AccountExternalKey>> {
        let keys = sqlx::query_as!(
            AccountExternalKey,
            r#"SELECT id as "id!", key_type, key_data, key_fingerprint, label
               FROM account_external_keys
               WHERE account_id = ?"#,
            account_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(keys)
    }

    /// Add account external key
    pub async fn add_account_external_key(
        &self,
        account_id: &[u8],
        key_type: &str,
        key_data: &str,
        key_fingerprint: Option<&str>,
        label: Option<&str>,
    ) -> Result<()> {
        let created_at = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        sqlx::query!(
            "INSERT INTO account_external_keys (account_id, key_type, key_data, key_fingerprint, label, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
            account_id,
            key_type,
            key_data,
            key_fingerprint,
            label,
            created_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete account external key by ID
    pub async fn delete_account_external_key(&self, account_id: &[u8], key_id: i64) -> Result<()> {
        sqlx::query!(
            "DELETE FROM account_external_keys WHERE account_id = ? AND id = ?",
            account_id,
            key_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests;
