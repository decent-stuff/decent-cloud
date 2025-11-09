use super::types::{Database, LedgerEntryData};
use anyhow::Result;
use dcc_common::{FundsTransfer, FundsTransferApproval};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct TokenTransfer {
    pub from_account: String,
    pub to_account: String,
    pub amount_e9s: i64,
    pub fee_e9s: i64,
    pub memo: Option<String>,
    pub created_at_ns: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct TokenApproval {
    pub owner_account: String,
    pub spender_account: String,
    pub amount_e9s: i64,
    pub expires_at_ns: Option<i64>,
    pub created_at_ns: i64,
}

impl Database {
    /// Get token transfers for an account
    pub async fn get_account_transfers(
        &self,
        account: &str,
        limit: i64,
    ) -> Result<Vec<TokenTransfer>> {
        let transfers = sqlx::query_as::<_, TokenTransfer>(
            "SELECT from_account, to_account, amount_e9s, fee_e9s, memo, created_at_ns
             FROM token_transfers
             WHERE from_account = ? OR to_account = ?
             ORDER BY created_at_ns DESC LIMIT ?",
        )
        .bind(account)
        .bind(account)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(transfers)
    }

    /// Get recent token transfers
    pub async fn get_recent_transfers(&self, limit: i64) -> Result<Vec<TokenTransfer>> {
        let transfers = sqlx::query_as::<_, TokenTransfer>(
            "SELECT from_account, to_account, amount_e9s, fee_e9s, memo, created_at_ns
             FROM token_transfers
             ORDER BY created_at_ns DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(transfers)
    }

    /// Get account balance (sum of all transfers)
    pub async fn get_account_balance(&self, account: &str) -> Result<i64> {
        let received: (Option<i64>,) =
            sqlx::query_as("SELECT SUM(amount_e9s) FROM token_transfers WHERE to_account = ?")
                .bind(account)
                .fetch_one(&self.pool)
                .await?;

        let sent: (Option<i64>,) = sqlx::query_as(
            "SELECT SUM(amount_e9s + fee_e9s) FROM token_transfers WHERE from_account = ?",
        )
        .bind(account)
        .fetch_one(&self.pool)
        .await?;

        Ok(received.0.unwrap_or(0) - sent.0.unwrap_or(0))
    }

    /// Get token approvals for an account
    pub async fn get_account_approvals(&self, account: &str) -> Result<Vec<TokenApproval>> {
        let approvals = sqlx::query_as::<_, TokenApproval>(
            "SELECT owner_account, spender_account, amount_e9s, expires_at_ns, created_at_ns
             FROM token_approvals
             WHERE owner_account = ? OR spender_account = ?
             ORDER BY created_at_ns DESC",
        )
        .bind(account)
        .bind(account)
        .fetch_all(&self.pool)
        .await?;

        Ok(approvals)
    }
    // Token transfers
    pub(crate) async fn insert_token_transfers(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let transfer = FundsTransfer::from_bytes(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse transfer: {}", e))?;

            sqlx::query(
                "INSERT INTO token_transfers (from_account, to_account, amount_e9s, fee_e9s, memo, created_at_ns, block_hash, block_offset) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(transfer.from().to_string())
            .bind(transfer.to().to_string())
            .bind(transfer.amount() as i64)
            .bind(transfer.fee().unwrap_or(0) as i64)
            .bind(String::from_utf8_lossy(transfer.memo()).to_string())
            .bind(entry.block_timestamp_ns as i64)
            .bind(&entry.block_hash)
            .bind(entry.block_offset as i64)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Token approvals
    pub(crate) async fn insert_token_approvals(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let approval = FundsTransferApproval::deserialize(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse approval: {}", e))?;

            sqlx::query(
                "INSERT INTO token_approvals (owner_account, spender_account, amount_e9s, expires_at_ns, created_at_ns) VALUES (?, ?, ?, ?, ?)"
            )
            .bind(approval.approver().to_string())
            .bind(approval.spender().to_string())
            .bind(approval.allowance().allowance.0.to_string().parse::<i64>().unwrap_or(0))
            .bind(approval.allowance().expires_at.map(|v| v as i64))
            .bind(entry.block_timestamp_ns as i64)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_test_db() -> Database {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        let migration_sql = include_str!("../../migrations/001_original_schema.sql");
        sqlx::query(migration_sql).execute(&pool).await.unwrap();
        Database { pool }
    }

    async fn insert_transfer(
        db: &Database,
        from: &str,
        to: &str,
        amount: i64,
        fee: i64,
        timestamp: i64,
    ) {
        sqlx::query("INSERT INTO token_transfers (from_account, to_account, amount_e9s, fee_e9s, memo, created_at_ns) VALUES (?, ?, ?, ?, '', ?)")
            .bind(from).bind(to).bind(amount).bind(fee).bind(timestamp).execute(&db.pool).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_account_transfers_empty() {
        let db = setup_test_db().await;
        let transfers = db.get_account_transfers("alice", 10).await.unwrap();
        assert_eq!(transfers.len(), 0);
    }

    #[tokio::test]
    async fn test_get_account_transfers() {
        let db = setup_test_db().await;

        insert_transfer(&db, "alice", "bob", 100, 1, 1000).await;
        insert_transfer(&db, "bob", "alice", 50, 1, 2000).await;
        insert_transfer(&db, "charlie", "dave", 200, 1, 3000).await;

        let transfers = db.get_account_transfers("alice", 10).await.unwrap();
        assert_eq!(transfers.len(), 2);
    }

    #[tokio::test]
    async fn test_get_account_transfers_limit() {
        let db = setup_test_db().await;

        for i in 0..5 {
            insert_transfer(&db, "alice", "bob", 100, 1, i * 1000).await;
        }

        let transfers = db.get_account_transfers("alice", 3).await.unwrap();
        assert_eq!(transfers.len(), 3);
    }

    #[tokio::test]
    async fn test_get_recent_transfers() {
        let db = setup_test_db().await;

        insert_transfer(&db, "alice", "bob", 100, 1, 1000).await;
        insert_transfer(&db, "bob", "charlie", 50, 1, 2000).await;

        let transfers = db.get_recent_transfers(10).await.unwrap();
        assert_eq!(transfers.len(), 2);
        assert_eq!(transfers[0].created_at_ns, 2000);
    }

    #[tokio::test]
    async fn test_get_account_balance_zero() {
        let db = setup_test_db().await;
        let balance = db.get_account_balance("alice").await.unwrap();
        assert_eq!(balance, 0);
    }

    #[tokio::test]
    async fn test_get_account_balance() {
        let db = setup_test_db().await;

        insert_transfer(&db, "alice", "bob", 100, 10, 1000).await;
        insert_transfer(&db, "charlie", "alice", 200, 5, 2000).await;
        insert_transfer(&db, "alice", "dave", 50, 5, 3000).await;

        let balance = db.get_account_balance("alice").await.unwrap();
        assert_eq!(balance, 200 - 100 - 10 - 50 - 5);
    }

    #[tokio::test]
    async fn test_get_account_approvals_empty() {
        let db = setup_test_db().await;
        let approvals = db.get_account_approvals("alice").await.unwrap();
        assert_eq!(approvals.len(), 0);
    }

    #[tokio::test]
    async fn test_get_account_approvals() {
        let db = setup_test_db().await;

        sqlx::query("INSERT INTO token_approvals (owner_account, spender_account, amount_e9s, expires_at_ns, created_at_ns) VALUES ('alice', 'bob', 1000, NULL, 0)")
            .execute(&db.pool).await.unwrap();
        sqlx::query("INSERT INTO token_approvals (owner_account, spender_account, amount_e9s, expires_at_ns, created_at_ns) VALUES ('bob', 'alice', 500, NULL, 1000)")
            .execute(&db.pool).await.unwrap();

        let approvals = db.get_account_approvals("alice").await.unwrap();
        assert_eq!(approvals.len(), 2);
    }
}
