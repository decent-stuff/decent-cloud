use super::types::Database;
use anyhow::{bail, Result};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::FromRow;

/// A raw 32-byte random token value (hex-encoded for the user).
/// Only exists transiently — never stored; only its SHA-256 hash is stored.
pub struct RawToken(pub [u8; 32]);

impl RawToken {
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        Self(bytes)
    }

    pub fn as_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn sha256_hash(&self) -> Vec<u8> {
        Sha256::digest(self.0).to_vec()
    }
}

/// Compute SHA-256 of a hex-encoded token string (for lookups).
pub fn hash_token_hex(token_hex: &str) -> Result<Vec<u8>> {
    let bytes =
        hex::decode(token_hex).map_err(|e| anyhow::anyhow!("Invalid token hex encoding: {}", e))?;
    Ok(Sha256::digest(bytes).to_vec())
}

#[derive(Debug, Clone, FromRow)]
pub struct ApiToken {
    pub id: uuid::Uuid,
    pub name: String,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub revoked_at: Option<i64>,
}

impl ApiToken {
    pub fn is_active(&self) -> anyhow::Result<bool> {
        let now = crate::now_ns()?;
        Ok(self.revoked_at.is_none() && self.expires_at.is_none_or(|exp| exp > now))
    }
}

impl Database {
    /// Create a new API token for a user.
    /// Returns the `ApiToken` record plus the raw hex token (shown once to the user).
    pub async fn create_api_token(
        &self,
        user_pubkey: &[u8],
        name: &str,
        expires_in_days: Option<i64>,
    ) -> Result<(ApiToken, String)> {
        if name.trim().is_empty() {
            bail!("Token name must not be empty");
        }
        if name.len() > 100 {
            bail!("Token name must not exceed 100 characters");
        }

        let raw = RawToken::generate();
        let token_hex = raw.as_hex();
        let token_hash = raw.sha256_hash();

        let now = crate::now_ns()?;
        let expires_at = expires_in_days.map(|days| now + days * 24 * 3600 * 1_000_000_000i64);

        let id = uuid::Uuid::new_v4();

        sqlx::query(
            "INSERT INTO api_tokens (id, user_pubkey, name, token_hash, created_at, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(user_pubkey)
        .bind(name)
        .bind(&token_hash)
        .bind(now)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        let token = self
            .get_api_token_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("API token not found after creation"))?;

        Ok((token, token_hex))
    }

    /// List all tokens for a user (does not include the raw token value).
    pub async fn list_api_tokens(&self, user_pubkey: &[u8]) -> Result<Vec<ApiToken>> {
        let tokens = sqlx::query_as::<_, ApiToken>(
            "SELECT id, name, created_at, last_used_at, expires_at, revoked_at
             FROM api_tokens
             WHERE user_pubkey = $1
             ORDER BY created_at DESC",
        )
        .bind(user_pubkey)
        .fetch_all(&self.pool)
        .await?;

        Ok(tokens)
    }

    /// Revoke a token by setting revoked_at. Only the owning user can revoke their token.
    pub async fn revoke_api_token(&self, token_id: uuid::Uuid, user_pubkey: &[u8]) -> Result<()> {
        let now = crate::now_ns()?;

        let result = sqlx::query(
            "UPDATE api_tokens SET revoked_at = $1
             WHERE id = $2 AND user_pubkey = $3 AND revoked_at IS NULL",
        )
        .bind(now)
        .bind(token_id)
        .bind(user_pubkey)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            bail!("API token not found or already revoked");
        }

        Ok(())
    }

    /// Look up user pubkey by token hash. Updates last_used_at.
    /// Returns None if token is not found, expired, or revoked.
    pub async fn lookup_api_token_pubkey(&self, token_hash: &[u8]) -> Result<Option<Vec<u8>>> {
        let now = crate::now_ns()?;

        let result: Option<(uuid::Uuid, Vec<u8>)> = sqlx::query_as(
            "SELECT id, user_pubkey FROM api_tokens
             WHERE token_hash = $1
               AND revoked_at IS NULL
               AND (expires_at IS NULL OR expires_at > $2)",
        )
        .bind(token_hash)
        .bind(now)
        .fetch_optional(&self.pool)
        .await?;

        let Some((id, user_pubkey)) = result else {
            return Ok(None);
        };

        // Update last_used_at (best-effort)
        sqlx::query("UPDATE api_tokens SET last_used_at = $1 WHERE id = $2")
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(Some(user_pubkey))
    }

    /// Get a single API token by its ID (internal helper).
    async fn get_api_token_by_id(&self, id: uuid::Uuid) -> Result<Option<ApiToken>> {
        let token = sqlx::query_as::<_, ApiToken>(
            "SELECT id, name, created_at, last_used_at, expires_at, revoked_at
             FROM api_tokens WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_helpers::setup_test_db;

    #[tokio::test]
    async fn test_create_and_list_api_tokens() {
        let db = setup_test_db().await;
        let pubkey = [10u8; 32];

        let (token, raw_hex) = db
            .create_api_token(&pubkey, "ci-token", None)
            .await
            .expect("Failed to create API token");

        // raw token must be 64 hex chars (32 bytes)
        assert_eq!(raw_hex.len(), 64);
        assert_eq!(token.name, "ci-token");
        assert!(token.revoked_at.is_none());
        assert!(token.expires_at.is_none());
        assert!(token.is_active().unwrap());

        let list = db
            .list_api_tokens(&pubkey)
            .await
            .expect("Failed to list API tokens");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "ci-token");
    }

    #[tokio::test]
    async fn test_revoke_api_token() {
        let db = setup_test_db().await;
        let pubkey = [11u8; 32];

        let (token, _) = db
            .create_api_token(&pubkey, "revoke-me", None)
            .await
            .expect("Failed to create API token");

        db.revoke_api_token(token.id, &pubkey)
            .await
            .expect("Failed to revoke token");

        let list = db
            .list_api_tokens(&pubkey)
            .await
            .expect("Failed to list API tokens");
        assert_eq!(list.len(), 1);
        assert!(list[0].revoked_at.is_some());
        assert!(!list[0].is_active().unwrap());

        // Revoking again must fail
        let err = db
            .revoke_api_token(token.id, &pubkey)
            .await
            .expect_err("Expected error revoking already-revoked token");
        assert!(err.to_string().contains("not found or already revoked"));
    }

    #[tokio::test]
    async fn test_revoke_wrong_owner_denied() {
        let db = setup_test_db().await;
        let owner = [12u8; 32];
        let other = [13u8; 32];

        let (token, _) = db
            .create_api_token(&owner, "owned", None)
            .await
            .expect("Failed to create API token");

        let err = db
            .revoke_api_token(token.id, &other)
            .await
            .expect_err("Expected error revoking another user's token");
        assert!(err.to_string().contains("not found or already revoked"));
    }

    #[tokio::test]
    async fn test_lookup_by_token_hash() {
        let db = setup_test_db().await;
        let pubkey = [14u8; 32];

        let (_, raw_hex) = db
            .create_api_token(&pubkey, "lookup-test", None)
            .await
            .expect("Failed to create API token");

        let hash = hash_token_hex(&raw_hex).expect("Failed to hash token");

        let found = db
            .lookup_api_token_pubkey(&hash)
            .await
            .expect("Failed to look up token");

        assert_eq!(found, Some(pubkey.to_vec()));

        // last_used_at should be set now
        let list = db.list_api_tokens(&pubkey).await.unwrap();
        assert!(list[0].last_used_at.is_some());
    }

    #[tokio::test]
    async fn test_lookup_revoked_token_returns_none() {
        let db = setup_test_db().await;
        let pubkey = [15u8; 32];

        let (token, raw_hex) = db
            .create_api_token(&pubkey, "revoked-lookup", None)
            .await
            .expect("Failed to create API token");

        db.revoke_api_token(token.id, &pubkey).await.unwrap();

        let hash = hash_token_hex(&raw_hex).expect("Failed to hash token");
        let found = db.lookup_api_token_pubkey(&hash).await.unwrap();
        assert_eq!(found, None);
    }

    #[tokio::test]
    async fn test_empty_name_rejected() {
        let db = setup_test_db().await;
        let pubkey = [16u8; 32];
        let err = db
            .create_api_token(&pubkey, "", None)
            .await
            .expect_err("Expected error for empty name");
        assert!(err.to_string().contains("must not be empty"));
    }
}
