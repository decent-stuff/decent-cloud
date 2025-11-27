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
    // Auth provider ('seed_phrase' or 'google_oauth')
    pub auth_provider: String,
    // Email for account linking (nullable for backward compatibility)
    pub email: Option<String>,
    // Profile fields (nullable)
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub profile_updated_at: Option<i64>,
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
    pub device_name: Option<String>,
}

/// Full account response with keys
#[derive(Debug, Clone, Serialize, Deserialize, poem_openapi::Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AccountWithKeys {
    pub id: String,
    pub username: String,
    pub created_at: i64,
    pub updated_at: i64,
    // Profile fields (optional)
    #[oai(skip_serializing_if_is_none)]
    pub display_name: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub bio: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub avatar_url: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub profile_updated_at: Option<i64>,
    pub public_keys: Vec<PublicKeyInfo>,
}

/// Public key information for API responses
#[derive(Debug, Clone, Serialize, Deserialize, poem_openapi::Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct PublicKeyInfo {
    pub id: String,
    pub public_key: String,
    pub added_at: i64,
    pub is_active: bool,
    #[oai(skip_serializing_if_is_none)]
    pub device_name: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub disabled_at: Option<i64>,
    #[oai(skip_serializing_if_is_none)]
    pub disabled_by_key_id: Option<String>,
}

/// Account profile for API responses
#[derive(Debug, Clone, Serialize, Deserialize, poem_openapi::Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AccountProfile {
    pub id: String,
    pub username: String,
    pub created_at: i64,
    pub updated_at: i64,
    #[oai(skip_serializing_if_is_none)]
    pub display_name: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub bio: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub avatar_url: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub profile_updated_at: Option<i64>,
}

impl From<Account> for AccountProfile {
    fn from(account: Account) -> Self {
        Self {
            id: hex::encode(&account.id),
            username: account.username,
            created_at: account.created_at,
            updated_at: account.updated_at,
            display_name: account.display_name,
            bio: account.bio,
            avatar_url: account.avatar_url,
            profile_updated_at: account.profile_updated_at,
        }
    }
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
            "SELECT id, username, created_at, updated_at, auth_provider, email, display_name, bio, avatar_url, profile_updated_at
             FROM accounts WHERE id = ?",
        )
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(account)
    }

    /// Get account by username
    pub async fn get_account_by_username(&self, username: &str) -> Result<Option<Account>> {
        let account = sqlx::query_as::<_, Account>(
            "SELECT id, username, created_at, updated_at, auth_provider, email, display_name, bio, avatar_url, profile_updated_at
             FROM accounts WHERE username = ?",
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
                display_name: account.display_name,
                bio: account.bio,
                avatar_url: account.avatar_url,
                profile_updated_at: account.profile_updated_at,
                public_keys: keys
                    .into_iter()
                    .map(|k| PublicKeyInfo {
                        id: hex::encode(&k.id),
                        public_key: hex::encode(&k.public_key),
                        added_at: k.added_at,
                        is_active: k.is_active != 0,
                        device_name: k.device_name,
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
            "SELECT id, account_id, public_key, is_active, added_at, disabled_at, disabled_by_key_id, device_name
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
            "SELECT id, account_id, public_key, is_active, added_at, disabled_at, disabled_by_key_id, device_name
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
            "SELECT id, account_id, public_key, is_active, added_at, disabled_at, disabled_by_key_id, device_name
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
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_signature_audit(
        &self,
        account_id: Option<&[u8]>,
        action: &str,
        payload: &str,
        signature: &[u8],
        public_key: &[u8],
        timestamp: i64,
        nonce: &uuid::Uuid,
        is_admin_action: bool,
    ) -> Result<()> {
        let nonce_bytes = nonce.as_bytes().to_vec();
        let is_admin = if is_admin_action { 1 } else { 0 };

        sqlx::query(
            "INSERT INTO signature_audit
             (account_id, action, payload, signature, public_key, timestamp, nonce, is_admin_action)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(account_id)
        .bind(action)
        .bind(payload)
        .bind(signature)
        .bind(public_key)
        .bind(timestamp)
        .bind(&nonce_bytes)
        .bind(is_admin)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Clean up old signature audit records (older than retention_days)
    /// Should be run periodically (e.g., daily) to maintain database hygiene
    pub async fn cleanup_signature_audit(&self, retention_days: i64) -> Result<u64> {
        let cutoff_time = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            - (retention_days * 24 * 60 * 60 * 1_000_000_000);

        let result = sqlx::query("DELETE FROM signature_audit WHERE created_at < ?")
            .bind(cutoff_time)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Get account with keys by public key (for login without username)
    pub async fn get_account_with_keys_by_public_key(
        &self,
        public_key: &[u8],
    ) -> Result<Option<AccountWithKeys>> {
        let account_id = match self.get_account_id_by_public_key(public_key).await? {
            Some(id) => id,
            None => return Ok(None),
        };

        let account = match self.get_account(&account_id).await? {
            Some(acc) => acc,
            None => return Ok(None),
        };

        let keys = self.get_account_keys(&account_id).await?;

        Ok(Some(AccountWithKeys {
            id: hex::encode(&account.id),
            username: account.username,
            created_at: account.created_at,
            updated_at: account.updated_at,
            display_name: account.display_name,
            bio: account.bio,
            avatar_url: account.avatar_url,
            profile_updated_at: account.profile_updated_at,
            public_keys: keys
                .into_iter()
                .map(|k| PublicKeyInfo {
                    id: hex::encode(&k.id),
                    public_key: hex::encode(&k.public_key),
                    added_at: k.added_at,
                    is_active: k.is_active != 0,
                    device_name: k.device_name,
                    disabled_at: k.disabled_at,
                    disabled_by_key_id: k.disabled_by_key_id.map(|id| hex::encode(&id)),
                })
                .collect(),
        }))
    }

    /// Update device name for a public key
    pub async fn update_device_name(
        &self,
        key_id: &[u8],
        device_name: Option<&str>,
    ) -> Result<AccountPublicKey> {
        sqlx::query("UPDATE account_public_keys SET device_name = ? WHERE id = ?")
            .bind(device_name)
            .bind(key_id)
            .execute(&self.pool)
            .await?;

        let key = sqlx::query_as::<_, AccountPublicKey>(
            "SELECT id, account_id, public_key, is_active, added_at, disabled_at, disabled_by_key_id, device_name
             FROM account_public_keys
             WHERE id = ?",
        )
        .bind(key_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Key not found"))?;

        Ok(key)
    }

    /// Update account profile fields
    pub async fn update_account_profile(
        &self,
        account_id: &[u8],
        display_name: Option<&str>,
        bio: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<Account> {
        let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        sqlx::query(
            "UPDATE accounts
             SET display_name = ?, bio = ?, avatar_url = ?, profile_updated_at = ?
             WHERE id = ?",
        )
        .bind(display_name)
        .bind(bio)
        .bind(avatar_url)
        .bind(now)
        .bind(account_id)
        .execute(&self.pool)
        .await?;

        self.get_account(account_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Account not found after profile update"))
    }

    /// Create OAuth account link
    pub async fn create_oauth_account(
        &self,
        account_id: &[u8],
        provider: &str,
        external_id: &str,
        email: Option<&str>,
    ) -> Result<OAuthAccount> {
        let oauth_id = uuid::Uuid::new_v4().as_bytes().to_vec();

        sqlx::query(
            "INSERT INTO oauth_accounts (id, account_id, provider, external_id, email) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&oauth_id)
        .bind(account_id)
        .bind(provider)
        .bind(external_id)
        .bind(email)
        .execute(&self.pool)
        .await?;

        self.get_oauth_account(&oauth_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("OAuth account not found after creation"))
    }

    /// Get OAuth account by ID
    pub async fn get_oauth_account(&self, oauth_id: &[u8]) -> Result<Option<OAuthAccount>> {
        let oauth_account = sqlx::query_as::<_, OAuthAccount>(
            "SELECT id, account_id, provider, external_id, email, created_at
             FROM oauth_accounts WHERE id = ?",
        )
        .bind(oauth_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(oauth_account)
    }

    /// Get OAuth account by provider and external ID
    pub async fn get_oauth_account_by_provider_and_external_id(
        &self,
        provider: &str,
        external_id: &str,
    ) -> Result<Option<OAuthAccount>> {
        let oauth_account = sqlx::query_as::<_, OAuthAccount>(
            "SELECT id, account_id, provider, external_id, email, created_at
             FROM oauth_accounts WHERE provider = ? AND external_id = ?",
        )
        .bind(provider)
        .bind(external_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(oauth_account)
    }

    /// Get account by ID
    pub async fn get_account_by_id(&self, account_id: &[u8]) -> Result<Option<Account>> {
        self.get_account(account_id).await
    }

    /// Get account by email
    pub async fn get_account_by_email(&self, email: &str) -> Result<Option<Account>> {
        let account = sqlx::query_as::<_, Account>(
            "SELECT id, username, created_at, updated_at, auth_provider, email, display_name, bio, avatar_url, profile_updated_at
             FROM accounts WHERE email = ?",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(account)
    }

    /// Create account with email and link to OAuth provider
    pub async fn create_oauth_linked_account(
        &self,
        username: &str,
        public_key: &[u8],
        email: &str,
        provider: &str,
        external_id: &str,
    ) -> Result<(Account, OAuthAccount)> {
        if public_key.len() != 32 {
            bail!("Public key must be 32 bytes");
        }

        // Start transaction
        let mut tx = self.pool.begin().await?;

        // Insert account with email
        let account_id = uuid::Uuid::new_v4().as_bytes().to_vec();
        sqlx::query(
            "INSERT INTO accounts (id, username, email, auth_provider) VALUES (?, ?, ?, ?)",
        )
        .bind(&account_id)
        .bind(username)
        .bind(email)
        .bind(provider)
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

        // Create OAuth account link
        let oauth_id = uuid::Uuid::new_v4().as_bytes().to_vec();
        sqlx::query(
            "INSERT INTO oauth_accounts (id, account_id, provider, external_id, email) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&oauth_id)
        .bind(&account_id)
        .bind(provider)
        .bind(external_id)
        .bind(email)
        .execute(&mut *tx)
        .await?;

        // Commit transaction
        tx.commit().await?;

        // Fetch and return both records
        let account = self
            .get_account(&account_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Account not found after creation"))?;

        let oauth_account = self
            .get_oauth_account(&oauth_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("OAuth account not found after creation"))?;

        Ok((account, oauth_account))
    }
}

/// OAuth account record from database
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct OAuthAccount {
    pub id: Vec<u8>,
    pub account_id: Vec<u8>,
    pub provider: String,
    pub external_id: String,
    pub email: Option<String>,
    pub created_at: i64,
}

#[cfg(test)]
mod tests;
