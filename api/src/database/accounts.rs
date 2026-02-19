use super::types::Database;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};

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
    // Email verification status
    pub email_verified: bool,
    // Profile fields (nullable)
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub profile_updated_at: Option<i64>,
    // Last login timestamp for activity tracking
    pub last_login_at: Option<i64>,
    // Admin flag for admin access control
    pub is_admin: bool,
    // Chatwoot Platform API user ID for support portal management
    pub chatwoot_user_id: Option<i64>,
    // Billing settings (nullable)
    pub billing_address: Option<String>,
    pub billing_vat_id: Option<String>,
    pub billing_country_code: Option<String>,
}

/// Account public key record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AccountPublicKey {
    pub id: Vec<u8>,
    pub account_id: Vec<u8>,
    pub public_key: Vec<u8>,
    pub is_active: bool,
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
    pub is_admin: bool,
    pub email_verified: bool,
    #[oai(skip_serializing_if_is_none)]
    pub email: Option<String>,
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

/// Billing settings for an account
#[derive(Debug, Clone, Serialize, Deserialize, poem_openapi::Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct BillingSettings {
    #[oai(skip_serializing_if_is_none)]
    pub billing_address: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub billing_vat_id: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub billing_country_code: Option<String>,
}

impl Database {
    /// Create a new account with initial public key
    pub async fn create_account(
        &self,
        username: &str,
        public_key: &[u8],
        email: &str,
    ) -> Result<Account> {
        if public_key.len() != 32 {
            bail!("Public key must be 32 bytes");
        }

        // Start transaction
        let mut tx = self.pool.begin().await?;

        // Insert account
        let account_id = uuid::Uuid::new_v4().as_bytes().to_vec();
        sqlx::query("INSERT INTO accounts (id, username, email) VALUES ($1, $2, $3)")
            .bind(&account_id)
            .bind(username)
            .bind(email)
            .execute(&mut *tx)
            .await?;

        // Insert initial public key
        let key_id = uuid::Uuid::new_v4().as_bytes().to_vec();
        sqlx::query(
            "INSERT INTO account_public_keys (id, account_id, public_key) VALUES ($1, $2, $3)",
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

    /// Create an email verification token for an account
    /// Returns the token bytes that should be sent via email
    pub async fn create_email_verification_token(
        &self,
        account_id: &[u8],
        email: &str,
    ) -> Result<Vec<u8>> {
        // Generate secure random token (32 bytes = 256 bits)
        let token = uuid::Uuid::new_v4().as_bytes().to_vec();
        let now = chrono::Utc::now().timestamp();
        let expires_at = now + (24 * 3600); // 24 hours expiry

        // Store token
        sqlx::query(
            "INSERT INTO email_verification_tokens (token, account_id, email, created_at, expires_at) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(&token)
        .bind(account_id)
        .bind(email)
        .bind(now)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(token)
    }

    /// Get the created_at timestamp of the most recent verification token for an account
    /// Used for rate limiting resend requests
    pub async fn get_latest_verification_token_time(
        &self,
        account_id: &[u8],
    ) -> Result<Option<i64>> {
        let result: Option<(i64,)> = sqlx::query_as(
            "SELECT created_at FROM email_verification_tokens WHERE account_id = $1 ORDER BY created_at DESC LIMIT 1"
        )
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|row| row.0))
    }

    /// Verify an email verification token and mark the email as verified
    /// Returns error if token is invalid, expired, or already used
    pub async fn verify_email_token(&self, token: &[u8]) -> Result<()> {
        // Start transaction
        let mut tx = self.pool.begin().await?;

        // Verify token (within transaction)
        let now = chrono::Utc::now().timestamp();
        let result = sqlx::query!(
            r#"SELECT account_id, expires_at, used_at
               FROM email_verification_tokens
               WHERE token = $1"#,
            token
        )
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = result else {
            tracing::warn!("Email verification failed: token not found in database");
            bail!("Invalid email verification token");
        };

        if row.used_at.is_some() {
            tracing::warn!("Email verification failed: token already used");
            bail!("Email verification token has already been used");
        }

        if now > row.expires_at {
            tracing::warn!(
                "Email verification failed: token expired (now={}, expires_at={})",
                now,
                row.expires_at
            );
            bail!("Email verification token has expired");
        }

        // Mark token as used
        sqlx::query!(
            "UPDATE email_verification_tokens SET used_at = $1 WHERE token = $2",
            now,
            token
        )
        .execute(&mut *tx)
        .await?;

        // Update email_verified flag on account
        sqlx::query!(
            "UPDATE accounts SET email_verified = TRUE WHERE id = $1",
            row.account_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    /// Get account by ID
    pub async fn get_account(&self, account_id: &[u8]) -> Result<Option<Account>> {
        let account = sqlx::query_as::<_, Account>(
            "SELECT id, username, created_at, updated_at, auth_provider, email, email_verified, display_name, bio, avatar_url, profile_updated_at, last_login_at, is_admin, chatwoot_user_id, billing_address, billing_vat_id, billing_country_code
             FROM accounts WHERE id = $1",
        )
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(account)
    }

    /// Get account by username (case-insensitive search)
    pub async fn get_account_by_username(&self, username: &str) -> Result<Option<Account>> {
        let account = sqlx::query_as::<_, Account>(
            "SELECT id, username, created_at, updated_at, auth_provider, email, email_verified, display_name, bio, avatar_url, profile_updated_at, last_login_at, is_admin, chatwoot_user_id, billing_address, billing_vat_id, billing_country_code
             FROM accounts WHERE LOWER(username) = LOWER($1)",
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
                        is_active: k.is_active,
                        device_name: k.device_name,
                        disabled_at: k.disabled_at,
                        disabled_by_key_id: k.disabled_by_key_id.map(|id| hex::encode(&id)),
                    })
                    .collect(),
                is_admin: account.is_admin,
                email_verified: account.email_verified,
                email: account.email.clone(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Get primary (first active) public key for a username. Used for notifications.
    pub async fn get_pubkey_by_username(&self, username: &str) -> Result<Option<Vec<u8>>> {
        let account = self.get_account_by_username(username).await?;
        let Some(account) = account else {
            return Ok(None);
        };

        let keys = self.get_active_account_keys(&account.id).await?;
        Ok(keys.first().map(|k| k.public_key.clone()))
    }

    /// Get all public keys for an account
    pub async fn get_account_keys(&self, account_id: &[u8]) -> Result<Vec<AccountPublicKey>> {
        let keys = sqlx::query_as::<_, AccountPublicKey>(
            "SELECT id, account_id, public_key, is_active, added_at, disabled_at, disabled_by_key_id, device_name
             FROM account_public_keys
             WHERE account_id = $1
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
             WHERE account_id = $1 AND is_active = TRUE
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
            "SELECT account_id FROM account_public_keys WHERE public_key = $1 AND is_active = TRUE",
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
            "INSERT INTO account_public_keys (id, account_id, public_key) VALUES ($1, $2, $3)",
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
             WHERE id = $1"
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
            sqlx::query_as("SELECT account_id FROM account_public_keys WHERE id = $1")
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
             SET is_active = FALSE, disabled_at = $1, disabled_by_key_id = $2
             WHERE id = $3",
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
            "SELECT 1::BIGINT FROM signature_audit
             WHERE nonce = $1 AND created_at > $2
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

        sqlx::query(
            "INSERT INTO signature_audit
             (account_id, action, payload, signature, public_key, timestamp, nonce, is_admin_action)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(account_id)
        .bind(action)
        .bind(payload)
        .bind(signature)
        .bind(public_key)
        .bind(timestamp)
        .bind(&nonce_bytes)
        .bind(is_admin_action)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Clean up old signature audit records (older than retention_days)
    /// Should be run periodically (e.g., daily) to maintain database hygiene
    pub async fn cleanup_signature_audit(&self, retention_days: i64) -> Result<u64> {
        let cutoff_time = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            - (retention_days * 24 * 60 * 60 * 1_000_000_000);

        let result = sqlx::query("DELETE FROM signature_audit WHERE created_at < $1")
            .bind(cutoff_time)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Get account with keys by public key (for login without username)
    /// Also updates last_login_at timestamp as this represents a login action
    pub async fn get_account_with_keys_by_public_key(
        &self,
        public_key: &[u8],
    ) -> Result<Option<AccountWithKeys>> {
        let account_id = match self.get_account_id_by_public_key(public_key).await? {
            Some(id) => id,
            None => return Ok(None),
        };

        // Update last login timestamp (best-effort, don't fail if this fails)
        if let Err(e) = self.update_last_login_by_public_key(public_key).await {
            tracing::warn!("Failed to update last_login_at: {:#}", e);
        }

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
                    is_active: k.is_active,
                    device_name: k.device_name,
                    disabled_at: k.disabled_at,
                    disabled_by_key_id: k.disabled_by_key_id.map(|id| hex::encode(&id)),
                })
                .collect(),
            is_admin: account.is_admin,
            email_verified: account.email_verified,
            email: account.email.clone(),
        }))
    }

    /// Update device name for a public key
    pub async fn update_device_name(
        &self,
        key_id: &[u8],
        device_name: Option<&str>,
    ) -> Result<AccountPublicKey> {
        sqlx::query("UPDATE account_public_keys SET device_name = $1 WHERE id = $2")
            .bind(device_name)
            .bind(key_id)
            .execute(&self.pool)
            .await?;

        let key = sqlx::query_as::<_, AccountPublicKey>(
            "SELECT id, account_id, public_key, is_active, added_at, disabled_at, disabled_by_key_id, device_name
             FROM account_public_keys
             WHERE id = $1",
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
             SET display_name = $1, bio = $2, avatar_url = $3, profile_updated_at = $4
             WHERE id = $5",
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

    /// Update account email
    /// When email changes, email_verified is reset to false
    pub async fn update_account_email(&self, account_id: &[u8], email: &str) -> Result<Account> {
        sqlx::query(
            "UPDATE accounts
             SET email = $1, email_verified = FALSE
             WHERE id = $2",
        )
        .bind(email)
        .bind(account_id)
        .execute(&self.pool)
        .await?;

        self.get_account(account_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Account not found after email update"))
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
            "INSERT INTO oauth_accounts (id, account_id, provider, external_id, email) VALUES ($1, $2, $3, $4, $5)"
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
             FROM oauth_accounts WHERE id = $1",
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
             FROM oauth_accounts WHERE provider = $1 AND external_id = $2",
        )
        .bind(provider)
        .bind(external_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(oauth_account)
    }

    /// Get account by email
    pub async fn get_account_by_email(&self, email: &str) -> Result<Option<Account>> {
        let account = sqlx::query_as::<_, Account>(
            "SELECT id, username, created_at, updated_at, auth_provider, email, email_verified, display_name, bio, avatar_url, profile_updated_at, last_login_at, is_admin, chatwoot_user_id, billing_address, billing_vat_id, billing_country_code
             FROM accounts WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(account)
    }

    /// Update last login timestamp for account by public key
    /// Returns true if an account was updated, false if no account found for this pubkey
    pub async fn update_last_login_by_public_key(&self, public_key: &[u8]) -> Result<bool> {
        let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let result = sqlx::query(
            "UPDATE accounts SET last_login_at = $1
             WHERE id IN (SELECT account_id FROM account_public_keys WHERE public_key = $2 AND is_active = TRUE)",
        )
        .bind(now)
        .bind(public_key)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
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

        // Insert account with email and verified status (OAuth providers have already verified the email)
        let account_id = uuid::Uuid::new_v4().as_bytes().to_vec();
        sqlx::query(
            "INSERT INTO accounts (id, username, email, auth_provider, email_verified) VALUES ($1, $2, $3, $4, TRUE)",
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
            "INSERT INTO account_public_keys (id, account_id, public_key) VALUES ($1, $2, $3)",
        )
        .bind(&key_id)
        .bind(&account_id)
        .bind(public_key)
        .execute(&mut *tx)
        .await?;

        // Create OAuth account link
        let oauth_id = uuid::Uuid::new_v4().as_bytes().to_vec();
        sqlx::query(
            "INSERT INTO oauth_accounts (id, account_id, provider, external_id, email) VALUES ($1, $2, $3, $4, $5)"
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

        // Queue welcome email (non-blocking, errors are logged)
        self.queue_email_safe(
            Some(email),
            "noreply@decent-cloud.org",
            "Welcome to Decent Cloud",
            &format!(
                "Welcome to Decent Cloud, {}!\n\n\
                Your account has been created successfully.\n\n\
                You can now start adding your offerings or renting offerings from existing providers on the platform.\n\n\
                If you have any questions, please visit our documentation or contact support.\n\n\
                Best regards,\n\
                The Decent Cloud Team",
                username
            ),
            false,
            super::email::EmailType::Welcome,  // Welcome emails: 12 attempts
        )
        .await;

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

    /// Set admin status for an account by username
    pub async fn set_admin_status(&self, username: &str, is_admin: bool) -> Result<()> {
        let result =
            sqlx::query("UPDATE accounts SET is_admin = $1 WHERE LOWER(username) = LOWER($2)")
                .bind(is_admin)
                .bind(username)
                .execute(&self.pool)
                .await?;

        if result.rows_affected() == 0 {
            bail!("Account not found: {}", username);
        }

        Ok(())
    }

    /// List all accounts with pagination
    /// Returns accounts ordered by username, with basic pagination support
    pub async fn list_all_accounts(&self, limit: i64, offset: i64) -> Result<Vec<Account>> {
        let accounts = sqlx::query_as::<_, Account>(
            "SELECT id, username, created_at, updated_at, auth_provider, email, email_verified, display_name, bio, avatar_url, profile_updated_at, last_login_at, is_admin, chatwoot_user_id, billing_address, billing_vat_id, billing_country_code
             FROM accounts ORDER BY username ASC LIMIT $1 OFFSET $2"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(accounts)
    }

    /// Get total count of all accounts
    pub async fn count_accounts(&self) -> Result<i64> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM accounts")
            .fetch_one(&self.pool)
            .await?;
        Ok(result.0)
    }

    /// List all admin accounts.
    /// Used by: `api-cli list-admins` command
    pub async fn list_admins(&self) -> Result<Vec<Account>> {
        let admins = sqlx::query_as::<_, Account>(
            "SELECT id, username, created_at, updated_at, auth_provider, email, email_verified, display_name, bio, avatar_url, profile_updated_at, last_login_at, is_admin, chatwoot_user_id, billing_address, billing_vat_id, billing_country_code
             FROM accounts WHERE is_admin = TRUE ORDER BY username ASC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(admins)
    }

    /// Admin: Set email verification status for an account
    pub async fn set_email_verified(&self, account_id: &[u8], verified: bool) -> Result<()> {
        let result = sqlx::query("UPDATE accounts SET email_verified = $1 WHERE id = $2")
            .bind(verified)
            .bind(account_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            bail!("Account not found");
        }

        Ok(())
    }

    /// Set Chatwoot user ID for an account
    pub async fn set_chatwoot_user_id(
        &self,
        account_id: &[u8],
        chatwoot_user_id: i64,
    ) -> Result<()> {
        let result = sqlx::query("UPDATE accounts SET chatwoot_user_id = $1 WHERE id = $2")
            .bind(chatwoot_user_id)
            .bind(account_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            bail!("Account not found");
        }

        Ok(())
    }

    /// Get Chatwoot user ID for an account by public key
    pub async fn get_chatwoot_user_id_by_public_key(
        &self,
        public_key: &[u8],
    ) -> Result<Option<i64>> {
        let result = sqlx::query_scalar!(
            r#"SELECT a.chatwoot_user_id as "chatwoot_user_id: i64"
               FROM accounts a
               JOIN account_public_keys pk ON pk.account_id = a.id
               WHERE pk.public_key = $1 AND pk.is_active = TRUE"#,
            public_key
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.flatten())
    }

    /// Get billing settings for an account
    pub async fn get_billing_settings(&self, account_id: &[u8]) -> Result<BillingSettings> {
        let row = sqlx::query(
            "SELECT billing_address, billing_vat_id, billing_country_code
             FROM accounts WHERE id = $1",
        )
        .bind(account_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(BillingSettings {
            billing_address: row.get("billing_address"),
            billing_vat_id: row.get("billing_vat_id"),
            billing_country_code: row.get("billing_country_code"),
        })
    }

    /// Update billing settings for an account
    pub async fn update_billing_settings(
        &self,
        account_id: &[u8],
        settings: &BillingSettings,
    ) -> Result<()> {
        let result = sqlx::query(
            "UPDATE accounts
             SET billing_address = $1, billing_vat_id = $2, billing_country_code = $3
             WHERE id = $4",
        )
        .bind(&settings.billing_address)
        .bind(&settings.billing_vat_id)
        .bind(&settings.billing_country_code)
        .bind(account_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            bail!("Account not found");
        }

        Ok(())
    }

    /// Ensure an account exists for a given pubkey, creating one if necessary.
    /// This is used for auto-creating accounts for orphan pubkeys during migration.
    ///
    /// Returns the account_id (existing or newly created).
    ///
    /// Generated username format: `user_<first8chars_of_hex_pubkey>`
    pub async fn ensure_account_for_pubkey(&self, pubkey: &[u8]) -> Result<Vec<u8>> {
        // Check if account already exists for this pubkey
        if let Some(account_id) = self.get_account_id_by_public_key(pubkey).await? {
            return Ok(account_id);
        }

        // Generate username from pubkey prefix
        let pubkey_hex = hex::encode(pubkey);
        let base_username = format!("user_{}", &pubkey_hex[..8]);

        // Try to create account with base username, append suffix if taken
        let mut username = base_username.clone();
        let mut suffix = 0u32;
        loop {
            // Use more of the pubkey for email uniqueness (suffix iteration + full pubkey hash)
            let email_suffix = if suffix == 0 {
                pubkey_hex[..16].to_string()
            } else {
                format!("{}_{}", &pubkey_hex[..16], suffix)
            };
            match self
                .create_account_internal(
                    &username,
                    pubkey,
                    &format!("auto-{}@noemail.local", email_suffix),
                )
                .await
            {
                Ok(account) => return Ok(account.id),
                Err(e) => {
                    let err_str = e.to_string().to_lowercase();
                    if err_str.contains("unique constraint") || err_str.contains("duplicate") {
                        // Username taken, try with suffix
                        suffix += 1;
                        username = format!("{}_{}", base_username, suffix);
                        if suffix > 100 {
                            bail!(
                                "Failed to generate unique username for pubkey after 100 attempts"
                            );
                        }
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    }

    /// Internal account creation without sending welcome email.
    /// Used by ensure_account_for_pubkey to avoid spamming auto-generated accounts.
    async fn create_account_internal(
        &self,
        username: &str,
        public_key: &[u8],
        email: &str,
    ) -> Result<Account> {
        if public_key.len() != 32 {
            bail!("Public key must be 32 bytes");
        }

        let mut tx = self.pool.begin().await?;

        let account_id = uuid::Uuid::new_v4().as_bytes().to_vec();
        sqlx::query("INSERT INTO accounts (id, username, email) VALUES ($1, $2, $3)")
            .bind(&account_id)
            .bind(username)
            .bind(email)
            .execute(&mut *tx)
            .await?;

        let key_id = uuid::Uuid::new_v4().as_bytes().to_vec();
        sqlx::query(
            "INSERT INTO account_public_keys (id, account_id, public_key) VALUES ($1, $2, $3)",
        )
        .bind(&key_id)
        .bind(&account_id)
        .bind(public_key)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.get_account(&account_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Account not found after creation"))
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

/// Summary of resources deleted when deleting an account
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccountDeletionSummary {
    pub offerings_deleted: i64,
    pub contracts_as_requester: i64,
    pub contracts_as_provider: i64,
    pub public_keys_deleted: i64,
    pub provider_profile_deleted: bool,
}

impl Database {
    /// Admin: Update email for an account (can set to new value or clear)
    pub async fn admin_set_account_email(
        &self,
        account_id: &[u8],
        email: Option<&str>,
    ) -> Result<()> {
        let result =
            sqlx::query("UPDATE accounts SET email = $1, email_verified = FALSE WHERE id = $2")
                .bind(email)
                .bind(account_id)
                .execute(&self.pool)
                .await?;

        if result.rows_affected() == 0 {
            bail!("Account not found");
        }

        Ok(())
    }

    /// Admin: Delete an account and all associated resources
    /// Returns a summary of what was deleted
    pub async fn admin_delete_account(&self, account_id: &[u8]) -> Result<AccountDeletionSummary> {
        let mut tx = self.pool.begin().await?;
        let mut summary = AccountDeletionSummary::default();

        // Get all public keys for this account to find offerings
        let pubkeys: Vec<Vec<u8>> =
            sqlx::query_scalar("SELECT public_key FROM account_public_keys WHERE account_id = $1")
                .bind(account_id)
                .fetch_all(&mut *tx)
                .await?;

        // Delete offerings (by account_id - covers both pubkey and account_id FK)
        let result = sqlx::query("DELETE FROM provider_offerings WHERE account_id = $1")
            .bind(account_id)
            .execute(&mut *tx)
            .await?;
        summary.offerings_deleted += result.rows_affected() as i64;
        // Also delete any offerings by pubkey (for backwards compatibility / incomplete backfill)
        for pubkey in &pubkeys {
            let result = sqlx::query(
                "DELETE FROM provider_offerings WHERE pubkey = $1 AND account_id IS NULL",
            )
            .bind(pubkey)
            .execute(&mut *tx)
            .await?;
            summary.offerings_deleted += result.rows_affected() as i64;
        }

        // Count contracts where user is requester (contracts reference pubkeys)
        let requester_contracts: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM contract_sign_requests WHERE requester_account_id = $1",
        )
        .bind(account_id)
        .fetch_one(&mut *tx)
        .await?;
        summary.contracts_as_requester = requester_contracts.0;

        // Count contracts where user is provider
        let provider_contracts: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM contract_sign_requests WHERE provider_account_id = $1",
        )
        .bind(account_id)
        .fetch_one(&mut *tx)
        .await?;
        summary.contracts_as_provider = provider_contracts.0;

        // Note: We don't delete contracts - they are historical records
        // Instead, we nullify the account references
        sqlx::query(
            "UPDATE contract_sign_requests SET requester_account_id = NULL WHERE requester_account_id = $1",
        )
        .bind(account_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE contract_sign_requests SET provider_account_id = NULL WHERE provider_account_id = $1",
        )
        .bind(account_id)
        .execute(&mut *tx)
        .await?;

        // Delete provider profiles (by account_id - covers both pubkey and account_id FK)
        let result = sqlx::query("DELETE FROM provider_profiles WHERE account_id = $1")
            .bind(account_id)
            .execute(&mut *tx)
            .await?;
        if result.rows_affected() > 0 {
            summary.provider_profile_deleted = true;
        }
        // Also delete any profiles by pubkey (for backwards compatibility / incomplete backfill)
        for pubkey in &pubkeys {
            let result = sqlx::query(
                "DELETE FROM provider_profiles WHERE pubkey = $1 AND account_id IS NULL",
            )
            .bind(pubkey)
            .execute(&mut *tx)
            .await?;
            if result.rows_affected() > 0 {
                summary.provider_profile_deleted = true;
            }
        }

        // Delete public keys (count them first)
        let keys_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM account_public_keys WHERE account_id = $1")
                .bind(account_id)
                .fetch_one(&mut *tx)
                .await?;
        summary.public_keys_deleted = keys_count.0;

        sqlx::query("DELETE FROM account_public_keys WHERE account_id = $1")
            .bind(account_id)
            .execute(&mut *tx)
            .await?;

        // Delete email verification tokens (CASCADE should handle, but be explicit)
        sqlx::query("DELETE FROM email_verification_tokens WHERE account_id = $1")
            .bind(account_id)
            .execute(&mut *tx)
            .await?;

        // Delete oauth accounts
        sqlx::query("DELETE FROM oauth_accounts WHERE account_id = $1")
            .bind(account_id)
            .execute(&mut *tx)
            .await?;

        // Delete account contacts, socials, external keys
        sqlx::query("DELETE FROM account_contacts WHERE account_id = $1")
            .bind(account_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM account_socials WHERE account_id = $1")
            .bind(account_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM account_external_keys WHERE account_id = $1")
            .bind(account_id)
            .execute(&mut *tx)
            .await?;

        // Delete recovery tokens
        sqlx::query("DELETE FROM recovery_tokens WHERE account_id = $1")
            .bind(account_id)
            .execute(&mut *tx)
            .await?;

        // Delete signature audit records (FK without cascade)
        sqlx::query("DELETE FROM signature_audit WHERE account_id = $1")
            .bind(account_id)
            .execute(&mut *tx)
            .await?;

        // Finally delete the account itself
        let result = sqlx::query("DELETE FROM accounts WHERE id = $1")
            .bind(account_id)
            .execute(&mut *tx)
            .await?;

        if result.rows_affected() == 0 {
            bail!("Account not found");
        }

        tx.commit().await?;
        Ok(summary)
    }
}

#[cfg(test)]
mod tests;
