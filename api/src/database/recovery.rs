use super::types::Database;
use anyhow::{bail, Context, Result};

const RECOVERY_TOKEN_EXPIRY_HOURS: i64 = 24;

impl Database {
    /// Create a recovery token for an account by email
    /// Returns the token bytes that should be sent via email
    pub async fn create_recovery_token(&self, email: &str) -> Result<Vec<u8>> {
        // Find account by email - check both accounts table and oauth_accounts table
        let account = sqlx::query!(
            r#"SELECT COALESCE(a.id, oa.account_id) as "id!"
               FROM accounts a
               LEFT JOIN oauth_accounts oa ON a.id = oa.account_id
               WHERE a.email = $1 OR oa.email = $2
               LIMIT 1"#,
            email,
            email
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(account) = account else {
            bail!("No account found with email: {}", email);
        };

        // Generate secure random token (32 bytes = 256 bits)
        let token = uuid::Uuid::new_v4().as_bytes().to_vec();
        let now = chrono::Utc::now().timestamp();
        let expires_at = now + (RECOVERY_TOKEN_EXPIRY_HOURS * 3600);

        // Store token
        sqlx::query!(
            "INSERT INTO recovery_tokens (token, account_id, created_at, expires_at) VALUES ($1, $2, $3, $4)",
            token,
            account.id,
            now,
            expires_at
        )
        .execute(&self.pool)
        .await
        .context("Failed to store recovery token")?;

        Ok(token)
    }

    /// Verify a recovery token and return the account ID if valid
    /// Returns error if token is invalid, expired, or already used
    #[allow(dead_code)]
    pub async fn verify_recovery_token(&self, token: &[u8]) -> Result<Vec<u8>> {
        let now = chrono::Utc::now().timestamp();

        let result = sqlx::query!(
            r#"SELECT account_id, expires_at, used_at
               FROM recovery_tokens
               WHERE token = $1"#,
            token
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = result else {
            bail!("Invalid recovery token");
        };

        if row.used_at.is_some() {
            bail!("Recovery token has already been used");
        }

        if now > row.expires_at {
            bail!("Recovery token has expired");
        }

        Ok(row.account_id)
    }

    /// Consume a recovery token and add a new public key to the account
    /// This completes the recovery flow
    pub async fn complete_recovery(&self, token: &[u8], new_public_key: &[u8]) -> Result<()> {
        // Start transaction
        let mut tx = self.pool.begin().await?;

        // Verify token (recheck within transaction)
        let now = chrono::Utc::now().timestamp();
        let result = sqlx::query!(
            r#"SELECT account_id, expires_at, used_at
               FROM recovery_tokens
               WHERE token = $1"#,
            token
        )
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = result else {
            bail!("Invalid recovery token");
        };

        if row.used_at.is_some() {
            bail!("Recovery token has already been used");
        }

        if now > row.expires_at {
            bail!("Recovery token has expired");
        }

        // Mark token as used
        sqlx::query!(
            "UPDATE recovery_tokens SET used_at = $1 WHERE token = $2",
            now,
            token
        )
        .execute(&mut *tx)
        .await?;

        // Add new public key to account
        let key_id = uuid::Uuid::new_v4().as_bytes().to_vec();
        sqlx::query!(
            r#"INSERT INTO account_public_keys
               (id, account_id, public_key, is_active, added_at)
               VALUES ($1, $2, $3, 1, $4)"#,
            key_id,
            row.account_id,
            new_public_key,
            now
        )
        .execute(&mut *tx)
        .await
        .context("Failed to add recovery key")?;

        tx.commit().await?;

        Ok(())
    }

    /// Clean up expired recovery tokens (should be called periodically)
    #[allow(dead_code)]
    pub async fn cleanup_expired_recovery_tokens(&self) -> Result<u64> {
        let now = chrono::Utc::now().timestamp();

        let result = sqlx::query!("DELETE FROM recovery_tokens WHERE expires_at < $1", now)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests;
