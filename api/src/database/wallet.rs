use super::types::Database;
use anyhow::{ensure, Context, Result};
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

/// Outcome of an idempotent wallet credit (used for Stripe top-ups).
///
/// A Stripe `checkout.session.completed` webhook is delivered at-least-once.
/// Re-delivery for the same session id MUST NOT credit the balance a second
/// time — see the partial unique index on `wallet_ledger(reference)` for
/// `entry_type='topup'` (migration 056).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalletCreditResult {
    /// This call credited the wallet for the first time for this reference.
    NewlyCredited {
        /// Wallet balance immediately after this credit (e9s).
        balance_e9s: i64,
    },
    /// A credit with this reference was already processed; this call was a
    /// no-op replay that returned the existing balance without re-crediting.
    AlreadyProcessed {
        /// Current committed wallet balance (unchanged by this call) in e9s.
        balance_e9s: i64,
    },
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

    /// Idempotent wallet top-up credit, keyed on the Stripe checkout session
    /// id (`reference`). The underlying `wallet_ledger` partial unique index
    /// (migration 056) guarantees a session can only credit the balance ONCE,
    /// even when Stripe replays the `checkout.session.completed` webhook.
    ///
    /// On a replay (same `reference` already credited) this returns
    /// [`WalletCreditResult::AlreadyProcessed`] with the current balance,
    /// having re-credited nothing. The balance upsert + ledger insert run in
    /// ONE transaction, so a unique violation on the ledger INSERT aborts the
    /// whole transaction (the balance upsert is rolled back too) — the money
    /// is credited exactly once.
    ///
    /// Use [`credit_wallet_balance`] for non-topup credits (refunds), which are
    /// NOT keyed on reference uniqueness.
    ///
    /// `amount_e9s` must be strictly positive.
    pub async fn credit_wallet_balance_idempotent(
        &self,
        pubkey_hex: &str,
        amount_e9s: i64,
        reference: &str,
    ) -> Result<WalletCreditResult> {
        ensure!(amount_e9s > 0, "credit amount must be positive");
        let mut tx = self.pool.begin().await?;

        // Balance upsert first (same as the non-idempotent credit).
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

        // Append the ledger row. Migration 056's partial unique index
        // (reference WHERE entry_type='topup') rejects a duplicate session id
        // with SQLSTATE 23505 unique_violation — that is the replay signal.
        // Reusing the exact SQL string of `credit_wallet_balance` keeps a
        // single cached query plan (entry_type/reference stay bound params).
        let insert = sqlx::query!(
            r#"INSERT INTO wallet_ledger (pubkey, amount_e9s, balance_after_e9s, entry_type, reference)
               VALUES ($1, $2, $3, $4, $5)"#,
            pubkey_hex,
            amount_e9s,
            new_balance,
            "topup",
            reference,
        )
        .execute(&mut *tx)
        .await;

        match insert {
            Ok(_) => {
                tx.commit().await?;
                Ok(WalletCreditResult::NewlyCredited {
                    balance_e9s: new_balance,
                })
            }
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                // Idempotent replay: this session id already credited the
                // wallet. The ledger INSERT above failed on the partial unique
                // index, which also aborted the transaction (so the balance
                // upsert was rolled back automatically — the money is never
                // double-counted). Roll back explicitly to release the
                // connection cleanly, then return the committed balance.
                tx.rollback()
                    .await
                    .context("rollback after idempotent top-up replay")?;
                let balance = self
                    .get_wallet_balance(pubkey_hex)
                    .await?
                    .context("wallet balance row missing after idempotent top-up replay")?;
                Ok(WalletCreditResult::AlreadyProcessed {
                    balance_e9s: balance,
                })
            }
            Err(e) => {
                // Any other ledger INSERT failure must not leave the balance
                // upsert committed without its audit row.
                tx.rollback()
                    .await
                    .context("rollback after ledger insert failure")?;
                Err(e.into())
            }
        }
    }


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

    /// Atomically debit the wallet AND mark the contract as paid via wallet,
    /// in a single DB transaction. This is the money-safe rental-payment
    /// primitive: the wallet debit, the ledger row, and the contract
    /// `payment_status='succeeded'` + `payment_method='wallet'` update all
    /// commit together, so a crash can never leave the wallet debited but the
    /// contract unpaid (or vice-versa).
    ///
    /// Returns `Ok(())` on success, or `Err` if the user has no wallet /
    /// insufficient balance. `amount_e9s` must be strictly positive.
    pub async fn debit_wallet_for_contract(
        &self,
        requester_pubkey_hex: &str,
        contract_id: &[u8],
        amount_e9s: i64,
    ) -> Result<()> {
        ensure!(amount_e9s > 0, "debit amount must be positive");
        let mut tx = self.pool.begin().await?;

        // Debit wallet row-level (rejects overdraft via WHERE balance >= amount).
        let new_balance = sqlx::query_scalar!(
            r#"UPDATE wallet_balances
               SET balance_e9s = balance_e9s - $2, updated_at = NOW()
               WHERE pubkey = $1 AND balance_e9s >= $2
               RETURNING balance_e9s as "balance_e9s!: i64""#,
            requester_pubkey_hex,
            amount_e9s,
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Insufficient wallet balance"))?;

        let contract_id_hex = hex::encode(contract_id);

        // Append immutable ledger entry (signed negative amount).
        sqlx::query!(
            r#"INSERT INTO wallet_ledger (pubkey, amount_e9s, balance_after_e9s, entry_type, reference)
               VALUES ($1, $2, $3, 'rental_debit', $4)"#,
            requester_pubkey_hex,
            -amount_e9s,
            new_balance,
            contract_id_hex,
        )
        .execute(&mut *tx)
        .await?;

        // Mark the contract as paid via wallet (no Stripe session/PI).
        sqlx::query(
            "UPDATE contract_sign_requests SET payment_status = 'succeeded', payment_method = 'wallet' WHERE contract_id = $1",
        )
        .bind(contract_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
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
    use super::{Database, WalletCreditResult};
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

    // ===== debit_wallet_for_contract tests =====

    /// Insert a minimal unpaid contract row for wallet-payment tests.
    async fn insert_unpaid_contract(db: &Database, contract_id: &[u8], requester_hex: &str, amount_e9s: i64) {
        sqlx::query!(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, payment_method, payment_status, currency) VALUES ($1, $2, 'ssh-key', 'contact', $3, 'off-1', $4, 'memo', 0, 'requested', 'stripe', 'pending', 'usd')",
            contract_id,
            hex::decode(requester_hex).unwrap(),
            hex::decode(pk(99)).unwrap(),
            amount_e9s,
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }

    /// Read a contract's payment_status + payment_method.
    async fn contract_payment_info(db: &Database, contract_id: &[u8]) -> (String, String) {
        let row = sqlx::query!(
            "SELECT payment_status as \"ps!\", payment_method as \"pm!\" FROM contract_sign_requests WHERE contract_id = $1",
            contract_id,
        )
        .fetch_one(&db.pool)
        .await
        .unwrap();
        (row.ps, row.pm)
    }

    #[tokio::test]
    async fn debit_for_contract_marks_paid_and_debits_wallet() {
        let db = setup_test_db().await;
        db.credit_wallet_balance(&pk(20), 10_000_000_000, "topup", None)
            .await
            .unwrap();
        let cid = hex::decode("aabbccdd").unwrap();
        insert_unpaid_contract(&db, &cid, &pk(20), 4_000_000_000).await;

        db.debit_wallet_for_contract(&pk(20), &cid, 4_000_000_000)
            .await
            .unwrap();

        // Wallet debited.
        assert_eq!(db.get_wallet_balance(&pk(20)).await.unwrap(), Some(6_000_000_000));
        // Contract marked paid via wallet.
        let (ps, pm) = contract_payment_info(&db, &cid).await;
        assert_eq!(ps, "succeeded");
        assert_eq!(pm, "wallet");
        // Ledger has the rental_debit entry.
        let ledger = db.get_wallet_ledger(&pk(20), 5).await.unwrap();
        assert_eq!(ledger.len(), 2); // topup + rental_debit
        assert_eq!(ledger[0].entry_type, "rental_debit");
        assert_eq!(ledger[0].amount_e9s, -4_000_000_000);
    }

    #[tokio::test]
    async fn debit_for_contract_fails_on_insufficient_balance() {
        let db = setup_test_db().await;
        db.credit_wallet_balance(&pk(21), 1_000_000_000, "topup", None)
            .await
            .unwrap(); // $1.00
        let cid = hex::decode("11223344").unwrap();
        insert_unpaid_contract(&db, &cid, &pk(21), 5_000_000_000).await; // $5.00

        let err = db.debit_wallet_for_contract(&pk(21), &cid, 5_000_000_000).await;
        assert!(err.is_err(), "must fail: $1 wallet < $5 contract");

        // Wallet unchanged.
        assert_eq!(db.get_wallet_balance(&pk(21)).await.unwrap(), Some(1_000_000_000));
        // Contract NOT marked paid (atomic rollback).
        let (ps, pm) = contract_payment_info(&db, &cid).await;
        assert_eq!(ps, "pending");
        assert_eq!(pm, "stripe");
    }

    #[tokio::test]
    async fn debit_for_contract_fails_with_no_wallet() {
        let db = setup_test_db().await;
        let cid = hex::decode("55667788").unwrap();
        insert_unpaid_contract(&db, &cid, &pk(22), 1_000_000_000).await;

        assert!(db.debit_wallet_for_contract(&pk(22), &cid, 1_000_000_000)
            .await
            .is_err());
        let (ps, _) = contract_payment_info(&db, &cid).await;
        assert_eq!(ps, "pending", "contract must stay unpaid");
    }

    // ===== top-up idempotency tests (Stripe webhook replay) =====

    /// Money-safety regression: crediting the SAME pubkey + amount + top-up
    /// reference twice (as happens when Stripe replays `checkout.session.
    /// completed`) must credit the balance exactly ONCE. The second call must
    /// be an idempotent no-op, NOT a second credit.
    #[tokio::test]
    async fn topup_idempotent_on_replay_does_not_double_credit() {
        let db = setup_test_db().await;
        let pubkey = pk(30);
        let amount = 5_000_000_000; // $5.00
        let reference = "cs_test_session_123";

        // First credit: a genuine top-up.
        let first = db
            .credit_wallet_balance_idempotent(&pubkey, amount, reference)
            .await
            .expect("first top-up must succeed");
        assert_eq!(
            first,
            WalletCreditResult::NewlyCredited {
                balance_e9s: 5_000_000_000
            },
            "first credit must be NewlyCredited"
        );

        // Second credit with the SAME reference: idempotent replay (Stripe
        // redelivered the webhook). Must NOT add money again, and must NOT
        // hard-error (the webhook handler returns 200 so Stripe stops retrying).
        let second = db
            .credit_wallet_balance_idempotent(&pubkey, amount, reference)
            .await
            .expect("replay must not hard-error (webhook must 200)");
        assert_eq!(
            second,
            WalletCreditResult::AlreadyProcessed {
                balance_e9s: 5_000_000_000
            },
            "second credit with same reference must be AlreadyProcessed (replay)"
        );

        // Money-safety: balance reflects a SINGLE credit, never doubled.
        assert_eq!(
            db.get_wallet_balance(&pubkey).await.unwrap(),
            Some(5_000_000_000),
            "balance must reflect one credit, not a double credit"
        );

        // The ledger must contain exactly ONE top-up row for this reference.
        let ledger = db.get_wallet_ledger(&pubkey, 10).await.unwrap();
        let topups: Vec<_> = ledger.iter().filter(|e| e.entry_type == "topup").collect();
        assert_eq!(topups.len(), 1, "exactly one top-up ledger row on replay");
        assert_eq!(topups[0].reference.as_deref(), Some(reference));
        assert_eq!(topups[0].amount_e9s, amount);
    }

    /// Distinct Stripe sessions (distinct references) must each credit
    /// independently — the idempotency key is the reference, not the pubkey.
    #[tokio::test]
    async fn topup_distinct_references_each_credit() {
        let db = setup_test_db().await;
        let pubkey = pk(31);

        let a = db
            .credit_wallet_balance_idempotent(&pubkey, 2_000_000_000, "cs_session_A")
            .await
            .unwrap();
        assert_eq!(
            a,
            WalletCreditResult::NewlyCredited {
                balance_e9s: 2_000_000_000
            }
        );

        let b = db
            .credit_wallet_balance_idempotent(&pubkey, 3_000_000_000, "cs_session_B")
            .await
            .unwrap();
        assert_eq!(
            b,
            WalletCreditResult::NewlyCredited {
                balance_e9s: 5_000_000_000
            }
        );

        assert_eq!(
            db.get_wallet_balance(&pubkey).await.unwrap(),
            Some(5_000_000_000)
        );
    }

    /// The top-up idempotency index (migration 056) is scoped to
    /// `entry_type='topup'`. Refunds (`rental_refund`) are keyed on the
    /// contract id, and a contract may legitimately accrue multiple refund
    /// entries — the unique index MUST NOT block them. This guards against
    /// an over-broad unique constraint that would break the refund path.
    #[tokio::test]
    async fn refunds_with_same_reference_are_not_blocked_by_topup_index() {
        let db = setup_test_db().await;
        let pubkey = pk(32);

        // Seed a balance via a top-up (distinct reference, not the refund's).
        db.credit_wallet_balance(&pubkey, 10_000_000_000, "topup", Some("cs_seed"))
            .await
            .unwrap();

        // Two refunds referencing the SAME contract id must both succeed.
        db.credit_wallet_balance(&pubkey, 1_000_000_000, "rental_refund", Some("contract-7"))
            .await
            .expect("first refund must succeed");
        db.credit_wallet_balance(&pubkey, 2_000_000_000, "rental_refund", Some("contract-7"))
            .await
            .expect("second refund with same reference must succeed (refunds are not unique-keyed)");

        // Both refunds credited: 10e9 + 1e9 + 2e9 = 13e9.
        assert_eq!(
            db.get_wallet_balance(&pubkey).await.unwrap(),
            Some(13_000_000_000)
        );

        let ledger = db.get_wallet_ledger(&pubkey, 10).await.unwrap();
        let refunds: Vec<_> = ledger
            .iter()
            .filter(|e| e.entry_type == "rental_refund")
            .collect();
        assert_eq!(refunds.len(), 2, "both refund ledger rows must be present");
        assert!(refunds.iter().all(|e| e.reference.as_deref() == Some("contract-7")));
    }
}
