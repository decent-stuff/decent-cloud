//! Persistent refund-attempt audit trail (issue #411).
//!
//! Every Stripe refund call MUST go through `record_refund_attempt` BEFORE the
//! network request and then `mark_refund_succeeded` / `mark_refund_failed`
//! after. The `idempotency_key` (`UNIQUE` in the schema) is the same string
//! sent to Stripe in the `Idempotency-Key` header so that a transient retry
//! is collapsed by both sides onto one Refund + one audit row.
//!
//! Runtime-checked queries (`sqlx::query`/`query_as`) are used throughout so
//! the new schema does not require `cargo sqlx prepare` regeneration; this
//! follows the same pattern as `agents_waitlist.rs` and `api_tokens.rs`.

use super::types::Database;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, PartialEq, Eq)]
pub struct RefundAudit {
    pub id: i64,
    pub contract_id: Vec<u8>,
    pub idempotency_key: String,
    pub stripe_payment_intent_id: Option<String>,
    pub stripe_charge_id: Option<String>,
    pub amount_cents: i64,
    pub currency: String,
    pub reason: String,
    pub status: String,
    pub stripe_refund_id: Option<String>,
    pub error_message: Option<String>,
    pub request_payload: serde_json::Value,
    pub response_payload: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Inputs for `record_refund_attempt`. Borrowed everywhere so refund call
/// sites avoid allocations on the hot path.
pub struct RefundAttemptInput<'a> {
    pub contract_id: &'a [u8],
    pub idempotency_key: &'a str,
    pub stripe_payment_intent_id: Option<&'a str>,
    pub stripe_charge_id: Option<&'a str>,
    pub amount_cents: i64,
    pub currency: &'a str,
    pub reason: &'a str,
    pub request_payload: &'a serde_json::Value,
}

impl Database {
    /// Insert a `requested` audit row before the Stripe call. Returns the
    /// row id; on a duplicate `idempotency_key` (retry of the same logical
    /// refund) returns the existing row id and leaves the original payload
    /// untouched -- a true retry must NOT overwrite the original `requested`
    /// timestamp or request body.
    pub async fn record_refund_attempt(&self, input: RefundAttemptInput<'_>) -> Result<i64> {
        let id: i64 = sqlx::query_scalar(
            r#"INSERT INTO refund_audit (
                contract_id, idempotency_key, stripe_payment_intent_id,
                stripe_charge_id, amount_cents, currency, reason, status,
                request_payload
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, 'requested', $8)
            ON CONFLICT (idempotency_key)
            DO UPDATE SET idempotency_key = EXCLUDED.idempotency_key
            RETURNING id"#,
        )
        .bind(input.contract_id)
        .bind(input.idempotency_key)
        .bind(input.stripe_payment_intent_id)
        .bind(input.stripe_charge_id)
        .bind(input.amount_cents)
        .bind(input.currency)
        .bind(input.reason)
        .bind(input.request_payload)
        .fetch_one(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to record refund attempt (idempotency_key={})",
                input.idempotency_key
            )
        })?;
        Ok(id)
    }

    /// Mark a previously-recorded attempt as succeeded. Idempotent on the
    /// `succeeded` state: replaying with the same `stripe_refund_id` is a
    /// no-op once `completed_at` is set, so a redundant webhook does not
    /// overwrite the earlier completion timestamp.
    pub async fn mark_refund_succeeded(
        &self,
        audit_id: i64,
        stripe_refund_id: &str,
        response_payload: &serde_json::Value,
    ) -> Result<()> {
        sqlx::query(
            r#"UPDATE refund_audit
               SET status = 'succeeded',
                   stripe_refund_id = $1,
                   response_payload = $2,
                   completed_at = NOW()
               WHERE id = $3 AND completed_at IS NULL"#,
        )
        .bind(stripe_refund_id)
        .bind(response_payload)
        .bind(audit_id)
        .execute(&self.pool)
        .await
        .with_context(|| format!("Failed to mark refund {} succeeded", audit_id))?;
        Ok(())
    }

    /// Mark a previously-recorded attempt as failed. Captures the error
    /// message + raw response (if any) so an operator can replay manually.
    /// A subsequent call from a different idempotency_key creates a fresh
    /// audit row -- this row stays as the historical "failed attempt".
    pub async fn mark_refund_failed(
        &self,
        audit_id: i64,
        error_message: &str,
        response_payload: Option<&serde_json::Value>,
    ) -> Result<()> {
        sqlx::query(
            r#"UPDATE refund_audit
               SET status = 'failed',
                   error_message = $1,
                   response_payload = $2,
                   completed_at = NOW()
               WHERE id = $3 AND completed_at IS NULL"#,
        )
        .bind(error_message)
        .bind(response_payload)
        .bind(audit_id)
        .execute(&self.pool)
        .await
        .with_context(|| format!("Failed to mark refund {} failed", audit_id))?;
        Ok(())
    }

    /// Look up an audit row by its idempotency_key. Used by ops dashboards
    /// and by tests asserting wiring end-to-end.
    pub async fn find_audit_by_idempotency_key(
        &self,
        key: &str,
    ) -> Result<Option<RefundAudit>> {
        let row: Option<RefundAudit> = sqlx::query_as(
            r#"SELECT id, contract_id, idempotency_key, stripe_payment_intent_id,
                       stripe_charge_id, amount_cents, currency, reason, status,
                       stripe_refund_id, error_message, request_payload,
                       response_payload, created_at, completed_at
                FROM refund_audit
                WHERE idempotency_key = $1"#,
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to look up refund audit for key {}", key))?;
        Ok(row)
    }

    /// Single audited entry point for every Stripe refund call site
    /// (cancel, reject, dispute_lost, ops manual). Records a `requested` row
    /// in `refund_audit` BEFORE the network call, then on response writes
    /// either `succeeded` (with stripe_refund_id) or `failed` (with
    /// error_message) and propagates the error to the caller. Returns
    /// `Some(refund_id)` on a successful Stripe call, `None` when
    /// `stripe_client` is not configured (pure-DB tests / dry-runs).
    pub async fn issue_audited_refund(
        &self,
        input: AuditedRefundInput<'_>,
    ) -> Result<Option<String>> {
        let mut request_payload = serde_json::json!({
            "payment_intent_id": input.payment_intent_id,
            "amount_cents": input.refund_cents,
            "currency": input.currency,
            "reason": input.reason,
        });
        if let Some(dispute_id) = input.stripe_dispute_id {
            request_payload["stripe_dispute_id"] =
                serde_json::Value::String(dispute_id.to_string());
        }

        let audit_id = self
            .record_refund_attempt(RefundAttemptInput {
                contract_id: input.contract_id,
                idempotency_key: input.idempotency_key,
                stripe_payment_intent_id: Some(input.payment_intent_id),
                stripe_charge_id: None,
                amount_cents: input.refund_cents,
                currency: input.currency,
                reason: input.reason,
                request_payload: &request_payload,
            })
            .await?;

        let Some(client) = input.stripe_client else {
            return Ok(None);
        };

        match client
            .create_refund(
                input.payment_intent_id,
                Some(input.refund_cents),
                input.idempotency_key,
            )
            .await
        {
            Ok(refund_id) => {
                let response = serde_json::json!({"id": &refund_id});
                self.mark_refund_succeeded(audit_id, &refund_id, &response)
                    .await?;
                Ok(Some(refund_id))
            }
            Err(e) => {
                let err_msg = format!("{:#}", e);
                self.mark_refund_failed(audit_id, &err_msg, None).await?;
                Err(e)
            }
        }
    }
}

/// Inputs for [`Database::issue_audited_refund`]. Borrowed so callers in the
/// hot cancel/reject/dispute paths avoid extra allocations.
pub struct AuditedRefundInput<'a> {
    pub contract_id: &'a [u8],
    pub idempotency_key: &'a str,
    pub payment_intent_id: &'a str,
    pub refund_cents: i64,
    pub currency: &'a str,
    pub reason: &'a str,
    /// Only set on the dispute-lost path; included in the audit request
    /// payload for ops correlation.
    pub stripe_dispute_id: Option<&'a str>,
    pub stripe_client: Option<&'a crate::stripe_client::StripeClient>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_helpers::setup_test_db;

    fn req_payload() -> serde_json::Value {
        serde_json::json!({"payment_intent_id": "pi_test", "amount_cents": 500})
    }

    fn input<'a>(
        contract_id: &'a [u8],
        idempotency_key: &'a str,
        amount_cents: i64,
        reason: &'a str,
        payload: &'a serde_json::Value,
    ) -> RefundAttemptInput<'a> {
        RefundAttemptInput {
            contract_id,
            idempotency_key,
            stripe_payment_intent_id: Some("pi_test"),
            stripe_charge_id: None,
            amount_cents,
            currency: "usd",
            reason,
            request_payload: payload,
        }
    }

    #[tokio::test]
    async fn test_record_refund_attempt_idempotent() {
        let db = setup_test_db().await;
        let cid = vec![1u8; 32];
        let key = "cancel:abcd:cancel:1";
        let payload = req_payload();

        let id1 = db
            .record_refund_attempt(input(&cid, key, 500, "cancel", &payload))
            .await
            .expect("first insert must succeed");

        // Replay with the same key MUST collapse: same id, exactly one row
        // in the table. This is the contract that protects us against
        // double-refunds when the network retries the wrapper.
        let id2 = db
            .record_refund_attempt(input(&cid, key, 500, "cancel", &payload))
            .await
            .expect("retry with same key must not error");
        assert_eq!(id1, id2, "duplicate idempotency_key must return same row id");

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM refund_audit WHERE idempotency_key = $1")
            .bind(key)
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(count, 1, "exactly one audit row per idempotency_key");
    }

    #[tokio::test]
    async fn test_mark_succeeded_updates_status_and_completed_at() {
        let db = setup_test_db().await;
        let cid = vec![2u8; 32];
        let key = "cancel:cd:cancel:2";
        let payload = req_payload();

        let id = db
            .record_refund_attempt(input(&cid, key, 1000, "cancel", &payload))
            .await
            .unwrap();

        let response = serde_json::json!({"id": "re_test_xyz", "status": "succeeded"});
        db.mark_refund_succeeded(id, "re_test_xyz", &response)
            .await
            .expect("mark succeeded must not fail");

        let row = db
            .find_audit_by_idempotency_key(key)
            .await
            .unwrap()
            .expect("row must be findable");
        assert_eq!(row.status, "succeeded");
        assert_eq!(row.stripe_refund_id.as_deref(), Some("re_test_xyz"));
        assert!(row.completed_at.is_some(), "completed_at must be set");
        assert_eq!(
            row.response_payload.as_ref(),
            Some(&response),
            "response payload must round-trip"
        );
    }

    #[tokio::test]
    async fn test_mark_failed_stores_error_and_does_not_block_retry_with_new_key() {
        let db = setup_test_db().await;
        let cid = vec![3u8; 32];
        let key1 = "cancel:cd:cancel:3";
        let key2 = "cancel:cd:cancel:4";
        let payload = req_payload();

        let id1 = db
            .record_refund_attempt(input(&cid, key1, 1500, "cancel", &payload))
            .await
            .unwrap();
        db.mark_refund_failed(id1, "stripe 502 bad gateway", None)
            .await
            .expect("mark failed must not fail");

        let row1 = db
            .find_audit_by_idempotency_key(key1)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row1.status, "failed");
        assert_eq!(row1.error_message.as_deref(), Some("stripe 502 bad gateway"));
        assert!(row1.completed_at.is_some());
        assert!(row1.stripe_refund_id.is_none(), "no refund_id on failure");

        // A NEW key (e.g. ops-initiated retry with a fresh unique_token) must
        // produce a SEPARATE audit row -- the audit table is per-attempt-key,
        // not per-contract.
        let id2 = db
            .record_refund_attempt(input(&cid, key2, 1500, "cancel", &payload))
            .await
            .unwrap();
        assert_ne!(id1, id2);

        let count_for_contract: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM refund_audit WHERE contract_id = $1")
                .bind(&cid)
                .fetch_one(&db.pool)
                .await
                .unwrap();
        assert_eq!(count_for_contract, 2);
    }
}
