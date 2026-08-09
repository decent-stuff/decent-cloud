use super::types::Database;
use anyhow::{ensure, Result};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

/// A wallet ledger entry (immutable audit row).
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct WalletLedgerEntry {
    pub id: i64,
    /// Signed: positive = credit (top-up/refund), negative = debit (rental).
    pub amount_e9s: i64,
    /// Running balance immediately after this entry was applied.
    pub balance_after_e9s: i64,
    pub entry_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub reference: Option<String>,
    /// Creation time (Unix seconds).
    pub created_at: i64,
}

impl Database {
    /// Get the wallet balance for a user (hex pubkey), in nano-USD.
    /// Returns `None` if the user has never topped up (no balance row).
    pub async fn get_wallet_balance(&self, pubkey_hex: &str) -> Result<Option<i64>> {
        let row = sqlx::query_scalar!(
            r#"SELECT balance_e9s as "balance_e9s!: i64" FROM wallet_balances WHERE pubkey = $1"#,
            pubkey_hex
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Credit the wallet (top-up or refund). Atomically upserts the balance row
    /// and appends a ledger entry in one transaction. Returns the new balance.
    ///
    /// `amount_e9s` must be strictly positive.
    pub async fn credit_wallet_balance(
        &self,
        pubkey_hex: &str,
        amount_e9s: i64,
        entry_type: &str,
        reference: Option<&str>,
    ) -> Result<i64> {
        ensure!(amount_e9s > 0, "credit amount must be positive");
        let mut tx = self.pool.begin().await?;
        let new_balance = sqlx::query_scalar!(
            r#"INSERT INTO wallet_balances (pubkey, balance_e9s)
               VALUES ($1, $2)
               ON CONFLICT (pubkey) DO UPDATE
               SET balance_e9s = wallet_balances.balance_e9s + $2,
                   updated_at = NOW()
               RETURNING balance_e9s as "balance_e9s!: i64""#,
            pubkey_hex,
            amount_e9s,
        )
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query!(
            r#"INSERT INTO wallet_ledger (pubkey, amount_e9s, balance_after_e9s, entry_type, reference)
               VALUES ($1, $2, $3, $4, $5)"#,
            pubkey_hex,
            amount_e9s,
            new_balance,
            entry_type,
            reference,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(new_balance)
    }

    /// Debit the wallet (rental payment). Atomically decrements the balance,
    /// rejecting overdrafts at the row level (`WHERE balance_e9s >= amount`).
    /// Returns the new balance, or an error if the user has no wallet /
    /// insufficient funds.
    ///
    /// `amount_e9s` must be strictly positive.
    #[allow(dead_code)] // wired in Unit 4 (rentals → balance debit)
    pub async fn debit_wallet_balance(
        &self,
        pubkey_hex: &str,
        amount_e9s: i64,
        entry_type: &str,
        reference: Option<&str>,
    ) -> Result<i64> {
        ensure!(amount_e9s > 0, "debit amount must be positive");
        let mut tx = self.pool.begin().await?;
        let new_balance = sqlx::query_scalar!(
            r#"UPDATE wallet_balances
               SET balance_e9s = balance_e9s - $2, updated_at = NOW()
               WHERE pubkey = $1 AND balance_e9s >= $2
               RETURNING balance_e9s as "balance_e9s!: i64""#,
            pubkey_hex,
            amount_e9s,
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Insufficient wallet balance for debit"))?;
        sqlx::query!(
            r#"INSERT INTO wallet_ledger (pubkey, amount_e9s, balance_after_e9s, entry_type, reference)
               VALUES ($1, $2, $3, $4, $5)"#,
            pubkey_hex,
            -amount_e9s,
            new_balance,
            entry_type,
            reference,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(new_balance)
    }

    /// Get recent wallet ledger entries for a user (newest first).
    pub async fn get_wallet_ledger(
        &self,
        pubkey_hex: &str,
        limit: i64,
    ) -> Result<Vec<WalletLedgerEntry>> {
        let rows = sqlx::query!(
            r#"SELECT id,
                      amount_e9s,
                      balance_after_e9s,
                      entry_type,
                      reference,
                      COALESCE(EXTRACT(EPOCH FROM created_at)::BIGINT, 0) as "created_at!: i64"
               FROM wallet_ledger
               WHERE pubkey = $1
               ORDER BY created_at DESC
               LIMIT $2"#,
            pubkey_hex,
            limit,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| WalletLedgerEntry {
                id: r.id,
                amount_e9s: r.amount_e9s,
                balance_after_e9s: r.balance_after_e9s,
                entry_type: r.entry_type,
                reference: r.reference,
                created_at: r.created_at,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::database::test_helpers::setup_test_db;

    /// Deterministic hex pubkey for wallet tests (distinct from other suites).
    fn pk(suffix: u8) -> String {
        format!("{:064x}", suffix)
    }

    #[tokio::test]
    async fn get_balance_nonexistent_is_none() {
        let db = setup_test_db().await;
        assert!(db.get_wallet_balance(&pk(1)).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn credit_creates_balance_and_returns_new_total() {
        let db = setup_test_db().await;
        let bal = db
            .credit_wallet_balance(&pk(2), 5_000_000_000, "topup", Some("cs_test_1"))
            .await
            .unwrap();
        assert_eq!(bal, 5_000_000_000); // $5.00
        // Second top-up accumulates.
        let bal2 = db
            .credit_wallet_balance(&pk(2), 3_000_000_000, "topup", Some("cs_test_2"))
            .await
            .unwrap();
        assert_eq!(bal2, 8_000_000_000); // $8.00
        assert_eq!(
            db.get_wallet_balance(&pk(2)).await.unwrap(),
            Some(8_000_000_000)
        );
    }

    #[tokio::test]
    async fn debit_succeeds_when_sufficient() {
        let db = setup_test_db().await;
        db.credit_wallet_balance(&pk(3), 10_000_000_000, "topup", None)
            .await
            .unwrap();
        let bal = db
            .debit_wallet_balance(&pk(3), 4_000_000_000, "rental_debit", Some("contract-99"))
            .await
            .unwrap();
        assert_eq!(bal, 6_000_000_000); // $6.00 remaining
    }

    #[tokio::test]
    async fn debit_fails_when_insufficient() {
        let db = setup_test_db().await;
        db.credit_wallet_balance(&pk(4), 1_000_000_000, "topup", None)
            .await
            .unwrap(); // $1.00
        let err = db
            .debit_wallet_balance(&pk(4), 2_000_000_000, "rental_debit", None)
            .await;
        assert!(err.is_err(), "debit larger than balance must fail");
        // Balance unchanged after failed debit.
        assert_eq!(
            db.get_wallet_balance(&pk(4)).await.unwrap(),
            Some(1_000_000_000)
        );
    }

    #[tokio::test]
    async fn debit_fails_when_no_wallet() {
        let db = setup_test_db().await;
        let err = db
            .debit_wallet_balance(&pk(5), 1_000_000_000, "rental_debit", None)
            .await;
        assert!(err.is_err(), "debit with no wallet must fail");
    }

    #[tokio::test]
    async fn balance_cannot_go_negative() {
        let db = setup_test_db().await;
        db.credit_wallet_balance(&pk(6), 1_000_000_000, "topup", None)
            .await
            .unwrap();
        // Exact-amount debit succeeds.
        let bal = db
            .debit_wallet_balance(&pk(6), 1_000_000_000, "rental_debit", None)
            .await
            .unwrap();
        assert_eq!(bal, 0);
        // Zero balance, any further debit fails.
        assert!(db
            .debit_wallet_balance(&pk(6), 1, "rental_debit", None)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn credit_rejects_nonpositive_amount() {
        let db = setup_test_db().await;
        assert!(db
            .credit_wallet_balance(&pk(7), 0, "topup", None)
            .await
            .is_err());
        assert!(db
            .credit_wallet_balance(&pk(7), -1, "topup", None)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn debit_rejects_nonpositive_amount() {
        let db = setup_test_db().await;
        db.credit_wallet_balance(&pk(8), 1_000_000_000, "topup", None)
            .await
            .unwrap();
        assert!(db
            .debit_wallet_balance(&pk(8), 0, "rental_debit", None)
            .await
            .is_err());
        assert!(db
            .debit_wallet_balance(&pk(8), -1, "rental_debit", None)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn ledger_records_signed_amounts_and_running_balance() {
        let db = setup_test_db().await;
        db.credit_wallet_balance(&pk(9), 10_000_000_000, "topup", Some("cs_a"))
            .await
            .unwrap();
        db.debit_wallet_balance(&pk(9), 3_000_000_000, "rental_debit", Some("c1"))
            .await
            .unwrap();
        db.credit_wallet_balance(&pk(9), 2_000_000_000, "rental_refund", Some("c1"))
            .await
            .unwrap();

        let ledger = db.get_wallet_ledger(&pk(9), 10).await.unwrap();
        assert_eq!(ledger.len(), 3, "three entries expected");
        // Newest first: refund (+2e9, bal 9e9), debit (-3e9, bal 7e9), topup (+10e9, bal 10e9).
        assert_eq!(ledger[0].amount_e9s, 2_000_000_000);
        assert_eq!(ledger[0].balance_after_e9s, 9_000_000_000);
        assert_eq!(ledger[0].entry_type, "rental_refund");
        assert_eq!(ledger[1].amount_e9s, -3_000_000_000);
        assert_eq!(ledger[1].balance_after_e9s, 7_000_000_000);
        assert_eq!(ledger[1].entry_type, "rental_debit");
        assert_eq!(ledger[2].amount_e9s, 10_000_000_000);
        assert_eq!(ledger[2].balance_after_e9s, 10_000_000_000);
        assert_eq!(ledger[2].entry_type, "topup");
    }

    #[tokio::test]
    async fn get_wallet_ledger_empty_for_new_user() {
        let db = setup_test_db().await;
        let ledger = db.get_wallet_ledger(&pk(10), 10).await.unwrap();
        assert!(ledger.is_empty());
    }

    #[tokio::test]
    async fn credit_then_refund_returns_to_original() {
        let db = setup_test_db().await;
        db.credit_wallet_balance(&pk(11), 5_000_000_000, "topup", None)
            .await
            .unwrap();
        db.debit_wallet_balance(&pk(11), 2_000_000_000, "rental_debit", Some("c1"))
            .await
            .unwrap();
        db.credit_wallet_balance(&pk(11), 2_000_000_000, "rental_refund", Some("c1"))
            .await
            .unwrap();
        // Full refund restores original balance.
        assert_eq!(
            db.get_wallet_balance(&pk(11)).await.unwrap(),
            Some(5_000_000_000)
        );
    }
}
