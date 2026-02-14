use super::types::Database;
use anyhow::Result;
use uuid::Uuid;

impl Database {
    /// Upsert acme-dns account for a provider's dc_id.
    /// On conflict (same dc_id), replaces credentials only if provider_pubkey matches.
    /// Returns `Ok(false)` if dc_id is owned by a different provider (ownership violation).
    pub async fn upsert_acme_dns_account(
        &self,
        username: Uuid,
        password_hash: &str,
        dc_id: &str,
        provider_pubkey: &[u8],
    ) -> Result<bool> {
        let result = sqlx::query!(
            r#"INSERT INTO acme_dns_accounts (username, password_hash, dc_id, provider_pubkey)
               VALUES ($1, $2, $3, $4)
               ON CONFLICT (dc_id) DO UPDATE
               SET username = EXCLUDED.username,
                   password_hash = EXCLUDED.password_hash,
                   created_at = NOW()
               WHERE acme_dns_accounts.provider_pubkey = EXCLUDED.provider_pubkey"#,
            username,
            password_hash,
            dc_id,
            provider_pubkey,
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Look up acme-dns account by username UUID.
    /// Returns (password_hash, dc_id) if found.
    pub async fn get_acme_dns_account(
        &self,
        username: Uuid,
    ) -> Result<Option<(String, String)>> {
        let row = sqlx::query!(
            "SELECT password_hash, dc_id FROM acme_dns_accounts WHERE username = $1",
            username,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| (r.password_hash, r.dc_id)))
    }

    /// Verify that a dc_id is owned by the given provider.
    /// Returns true if the dc_id exists and belongs to this provider.
    pub async fn verify_dc_id_owner(
        &self,
        dc_id: &str,
        provider_pubkey: &[u8],
    ) -> Result<bool> {
        let row = sqlx::query!(
            "SELECT 1 as found FROM acme_dns_accounts WHERE dc_id = $1 AND provider_pubkey = $2",
            dc_id,
            provider_pubkey,
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_helpers::setup_test_db;

    #[tokio::test]
    async fn test_upsert_and_get_acme_dns_account() {
        let db = setup_test_db().await;
        let username = Uuid::new_v4();
        let provider_pubkey = b"provider1";

        let ok = db
            .upsert_acme_dns_account(username, "sha256:abc", "dc-lk", provider_pubkey)
            .await
            .unwrap();
        assert!(ok);

        let (hash, id) = db
            .get_acme_dns_account(username)
            .await
            .unwrap()
            .expect("account should exist");
        assert_eq!(hash, "sha256:abc");
        assert_eq!(id, "dc-lk");
    }

    #[tokio::test]
    async fn test_get_acme_dns_account_not_found() {
        let db = setup_test_db().await;
        let result = db.get_acme_dns_account(Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_upsert_replaces_on_same_dc_id_same_provider() {
        let db = setup_test_db().await;
        let dc_id = "dc-us";
        let provider = b"provider-a";

        let username1 = Uuid::new_v4();
        assert!(db
            .upsert_acme_dns_account(username1, "hash1", dc_id, provider)
            .await
            .unwrap());

        // Re-register same dc_id, same provider → succeeds
        let username2 = Uuid::new_v4();
        assert!(db
            .upsert_acme_dns_account(username2, "hash2", dc_id, provider)
            .await
            .unwrap());

        // Old username gone
        assert!(db.get_acme_dns_account(username1).await.unwrap().is_none());

        // New username works
        let (hash, id) = db
            .get_acme_dns_account(username2)
            .await
            .unwrap()
            .expect("new account should exist");
        assert_eq!(hash, "hash2");
        assert_eq!(id, dc_id);
    }

    #[tokio::test]
    async fn test_upsert_rejects_different_provider() {
        let db = setup_test_db().await;
        let dc_id = "dc-eu";

        // Provider A registers dc_id
        let username1 = Uuid::new_v4();
        assert!(db
            .upsert_acme_dns_account(username1, "hash1", dc_id, b"provider-a")
            .await
            .unwrap());

        // Provider B tries to hijack same dc_id → rejected
        let username2 = Uuid::new_v4();
        let ok = db
            .upsert_acme_dns_account(username2, "hash2", dc_id, b"provider-b")
            .await
            .unwrap();
        assert!(!ok, "should reject different provider");

        // Original credentials unchanged
        let (hash, _) = db
            .get_acme_dns_account(username1)
            .await
            .unwrap()
            .expect("original should still exist");
        assert_eq!(hash, "hash1");
    }

    #[tokio::test]
    async fn test_verify_dc_id_owner() {
        let db = setup_test_db().await;
        let provider = b"owner-key";

        db.upsert_acme_dns_account(Uuid::new_v4(), "hash", "dc-sg", provider)
            .await
            .unwrap();

        // Correct owner
        assert!(db.verify_dc_id_owner("dc-sg", provider).await.unwrap());

        // Wrong owner
        assert!(!db
            .verify_dc_id_owner("dc-sg", b"attacker")
            .await
            .unwrap());

        // Nonexistent dc_id
        assert!(!db
            .verify_dc_id_owner("dc-xx", provider)
            .await
            .unwrap());
    }
}
