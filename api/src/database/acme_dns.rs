use super::types::Database;
use anyhow::Result;
use uuid::Uuid;

impl Database {
    /// Upsert acme-dns account for a provider's dc_id.
    /// On conflict (same dc_id), replaces credentials.
    pub async fn upsert_acme_dns_account(
        &self,
        username: Uuid,
        password_hash: &str,
        dc_id: &str,
    ) -> Result<()> {
        sqlx::query!(
            r#"INSERT INTO acme_dns_accounts (username, password_hash, dc_id)
               VALUES ($1, $2, $3)
               ON CONFLICT (dc_id) DO UPDATE
               SET username = EXCLUDED.username,
                   password_hash = EXCLUDED.password_hash,
                   created_at = NOW()"#,
            username,
            password_hash,
            dc_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_helpers::setup_test_db;

    #[tokio::test]
    async fn test_upsert_and_get_acme_dns_account() {
        let db = setup_test_db().await;
        let username = Uuid::new_v4();
        let password_hash = "sha256:abc123";
        let dc_id = "dc-lk";

        db.upsert_acme_dns_account(username, password_hash, dc_id)
            .await
            .unwrap();

        let result = db.get_acme_dns_account(username).await.unwrap();
        let (hash, id) = result.expect("account should exist");
        assert_eq!(hash, password_hash);
        assert_eq!(id, dc_id);
    }

    #[tokio::test]
    async fn test_get_acme_dns_account_not_found() {
        let db = setup_test_db().await;
        let result = db.get_acme_dns_account(Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_upsert_replaces_on_same_dc_id() {
        let db = setup_test_db().await;
        let dc_id = "dc-us";

        let username1 = Uuid::new_v4();
        db.upsert_acme_dns_account(username1, "hash1", dc_id)
            .await
            .unwrap();

        // Re-register same dc_id with new credentials
        let username2 = Uuid::new_v4();
        db.upsert_acme_dns_account(username2, "hash2", dc_id)
            .await
            .unwrap();

        // Old username should be gone
        assert!(db.get_acme_dns_account(username1).await.unwrap().is_none());

        // New username should work
        let (hash, id) = db
            .get_acme_dns_account(username2)
            .await
            .unwrap()
            .expect("new account should exist");
        assert_eq!(hash, "hash2");
        assert_eq!(id, dc_id);
    }
}
