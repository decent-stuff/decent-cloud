use super::types::Database;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Account record from database
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Account {
    pub id: Vec<u8>,
    pub username: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Account public key record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AccountPublicKey {
    pub id: Vec<u8>,
    pub account_id: Vec<u8>,
    pub public_key: Vec<u8>,
    pub is_active: i64,
    pub added_at: i64,
    pub disabled_at: Option<i64>,
    pub disabled_by_key_id: Option<Vec<u8>>,
}

/// Full account response with keys
#[derive(Debug, Clone, Serialize, Deserialize, poem_openapi::Object)]
pub struct AccountWithKeys {
    pub id: String,
    pub username: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub public_keys: Vec<PublicKeyInfo>,
}

/// Public key information for API responses
#[derive(Debug, Clone, Serialize, Deserialize, poem_openapi::Object)]
pub struct PublicKeyInfo {
    pub id: String,
    pub public_key: String,
    pub added_at: i64,
    pub is_active: bool,
    #[oai(skip_serializing_if_is_none)]
    pub disabled_at: Option<i64>,
    #[oai(skip_serializing_if_is_none)]
    pub disabled_by_key_id: Option<String>,
}

impl Database {
    /// Create a new account with initial public key
    pub async fn create_account(&self, username: &str, public_key: &[u8]) -> Result<Account> {
        if public_key.len() != 32 {
            bail!("Public key must be 32 bytes");
        }

        // Start transaction
        let mut tx = self.pool.begin().await?;

        // Insert account
        let account_id = uuid::Uuid::new_v4().as_bytes().to_vec();
        sqlx::query("INSERT INTO accounts (id, username) VALUES (?, ?)")
            .bind(&account_id)
            .bind(username)
            .execute(&mut *tx)
            .await?;

        // Insert initial public key
        let key_id = uuid::Uuid::new_v4().as_bytes().to_vec();
        sqlx::query(
            "INSERT INTO account_public_keys (id, account_id, public_key) VALUES (?, ?, ?)",
        )
        .bind(&key_id)
        .bind(&account_id)
        .bind(public_key)
        .execute(&mut *tx)
        .await?;

        // Commit transaction
        tx.commit().await?;

        // Fetch and return the account
        self.get_account(&account_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Account not found after creation"))
    }

    /// Get account by ID
    pub async fn get_account(&self, account_id: &[u8]) -> Result<Option<Account>> {
        let account = sqlx::query_as::<_, Account>(
            "SELECT id, username, created_at, updated_at FROM accounts WHERE id = ?",
        )
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(account)
    }

    /// Get account by username
    pub async fn get_account_by_username(&self, username: &str) -> Result<Option<Account>> {
        let account = sqlx::query_as::<_, Account>(
            "SELECT id, username, created_at, updated_at FROM accounts WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        Ok(account)
    }

    /// Get full account with all public keys
    pub async fn get_account_with_keys(&self, username: &str) -> Result<Option<AccountWithKeys>> {
        let account = self.get_account_by_username(username).await?;

        if let Some(account) = account {
            let keys = self.get_account_keys(&account.id).await?;

            Ok(Some(AccountWithKeys {
                id: hex::encode(&account.id),
                username: account.username,
                created_at: account.created_at,
                updated_at: account.updated_at,
                public_keys: keys
                    .into_iter()
                    .map(|k| PublicKeyInfo {
                        id: hex::encode(&k.id),
                        public_key: hex::encode(&k.public_key),
                        added_at: k.added_at,
                        is_active: k.is_active != 0,
                        disabled_at: k.disabled_at,
                        disabled_by_key_id: k.disabled_by_key_id.map(|id| hex::encode(&id)),
                    })
                    .collect(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Get all public keys for an account
    pub async fn get_account_keys(&self, account_id: &[u8]) -> Result<Vec<AccountPublicKey>> {
        let keys = sqlx::query_as::<_, AccountPublicKey>(
            "SELECT id, account_id, public_key, is_active, added_at, disabled_at, disabled_by_key_id
             FROM account_public_keys
             WHERE account_id = ?
             ORDER BY added_at ASC"
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(keys)
    }

    /// Get active public keys for an account
    pub async fn get_active_account_keys(
        &self,
        account_id: &[u8],
    ) -> Result<Vec<AccountPublicKey>> {
        let keys = sqlx::query_as::<_, AccountPublicKey>(
            "SELECT id, account_id, public_key, is_active, added_at, disabled_at, disabled_by_key_id
             FROM account_public_keys
             WHERE account_id = ? AND is_active = 1
             ORDER BY added_at ASC"
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(keys)
    }

    /// Get account ID by public key
    pub async fn get_account_id_by_public_key(&self, public_key: &[u8]) -> Result<Option<Vec<u8>>> {
        let result: Option<(Vec<u8>,)> = sqlx::query_as(
            "SELECT account_id FROM account_public_keys WHERE public_key = ? AND is_active = 1",
        )
        .bind(public_key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|r| r.0))
    }

    /// Add a new public key to an account
    pub async fn add_account_key(
        &self,
        account_id: &[u8],
        public_key: &[u8],
    ) -> Result<AccountPublicKey> {
        if public_key.len() != 32 {
            bail!("Public key must be 32 bytes");
        }

        // Check max keys limit (10)
        let active_keys = self.get_active_account_keys(account_id).await?;
        if active_keys.len() >= 10 {
            bail!("Maximum 10 keys per account");
        }

        // Insert new key
        let key_id = uuid::Uuid::new_v4().as_bytes().to_vec();
        sqlx::query(
            "INSERT INTO account_public_keys (id, account_id, public_key) VALUES (?, ?, ?)",
        )
        .bind(&key_id)
        .bind(account_id)
        .bind(public_key)
        .execute(&self.pool)
        .await?;

        // Fetch and return the key
        let key = sqlx::query_as::<_, AccountPublicKey>(
            "SELECT id, account_id, public_key, is_active, added_at, disabled_at, disabled_by_key_id
             FROM account_public_keys
             WHERE id = ?"
        )
        .bind(&key_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(key)
    }

    /// Disable (soft delete) a public key
    pub async fn disable_account_key(
        &self,
        key_id: &[u8],
        disabled_by_key_id: &[u8],
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        // Get the key to check account_id
        let key: Option<(Vec<u8>,)> =
            sqlx::query_as("SELECT account_id FROM account_public_keys WHERE id = ?")
                .bind(key_id)
                .fetch_optional(&self.pool)
                .await?;

        let account_id = key.ok_or_else(|| anyhow::anyhow!("Key not found"))?.0;

        // Check that this is not the last active key
        let active_keys = self.get_active_account_keys(&account_id).await?;
        if active_keys.len() <= 1 {
            bail!("Cannot remove the last active key");
        }

        // Disable the key
        sqlx::query(
            "UPDATE account_public_keys
             SET is_active = 0, disabled_at = ?, disabled_by_key_id = ?
             WHERE id = ?",
        )
        .bind(now)
        .bind(disabled_by_key_id)
        .bind(key_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Check if a nonce has been used (for replay prevention)
    pub async fn check_nonce_exists(
        &self,
        nonce: &uuid::Uuid,
        max_age_minutes: i64,
    ) -> Result<bool> {
        let cutoff_time = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            - (max_age_minutes * 60 * 1_000_000_000);
        let nonce_bytes = nonce.as_bytes().to_vec();

        let result: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM signature_audit
             WHERE nonce = ? AND created_at > ?
             LIMIT 1",
        )
        .bind(nonce_bytes)
        .bind(cutoff_time)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.is_some())
    }

    /// Insert signature audit record
    pub async fn insert_signature_audit(
        &self,
        account_id: Option<&[u8]>,
        action: &str,
        payload: &str,
        signature: &[u8],
        public_key: &[u8],
        timestamp: i64,
        nonce: &uuid::Uuid,
    ) -> Result<()> {
        let nonce_bytes = nonce.as_bytes().to_vec();

        sqlx::query(
            "INSERT INTO signature_audit
             (account_id, action, payload, signature, public_key, timestamp, nonce)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(account_id)
        .bind(action)
        .bind(payload)
        .bind(signature)
        .bind(public_key)
        .bind(timestamp)
        .bind(&nonce_bytes)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests;
