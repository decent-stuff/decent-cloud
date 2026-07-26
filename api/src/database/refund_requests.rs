//! Refund approval gate (issue: cost-safe billing).
//!
//! Every Stripe refund MUST go through [`Database::process_gated_refund`]. The
//! gate creates a `refund_requests` row FIRST (unbypassable audit), then:
//!
//! - **Auto-issues** when `refund_e9s <= user_latest_stripe_payment_e9s`
//! - **Holds for admin approval** when the refund exceeds the cap
//!
//! A DB trigger (`enforce_refund_approval_gate`, migration 051) is the
//! backstop: `contract_sign_requests.payment_status` cannot transition to
//! `'refunded'` and `stripe_refund_id` cannot be set unless a matching
//! `refund_requests` row exists with `status IN ('auto_issued', 'approved')`.

use super::types::Database;
use anyhow::{Context, Result};
use sqlx::FromRow;

/// Materialized refund-request row. Returned by listing/approval queries.
#[derive(Debug, Clone, FromRow)]
pub struct RefundRequest {
    pub id: i64,
    pub contract_id: Vec<u8>,
    pub requester_pubkey: Vec<u8>,
    pub refund_amount_e9s: i64,
    pub reason: String,
    pub status: String,
    pub user_latest_payment_e9s: i64,
    pub cap_exceeded: bool,
    pub payment_intent_id: String,
    pub currency: String,
    pub stripe_dispute_id: Option<String>,
    pub stripe_refund_id: Option<String>,
    pub idempotency_key: String,
    pub created_at_ns: i64,
    pub reviewed_at_ns: Option<i64>,
    pub reviewed_by: Option<Vec<u8>>,
    pub review_note: Option<String>,
}

/// Outcome of the refund gate. Callers use this to decide whether to flip
/// `payment_status` to `'refunded'` (only when a real Stripe refund id exists).
#[derive(Debug)]
pub enum RefundGateOutcome {
    /// Cap passed; refund was issued (or computed but no Stripe client —
    /// `stripe_refund_id` is `None` in that case, matching `issue_audited_refund`).
    AutoIssued {
        refund_amount_e9s: i64,
        stripe_refund_id: Option<String>,
    },
    /// Cap exceeded; refund held for admin approval. The contract action
    /// (cancel/reject) still proceeds, but `payment_status` stays unchanged.
    PendingApproval {
        refund_amount_e9s: i64,
        user_latest_payment_e9s: i64,
    },
    /// Nothing owed (refund_e9s <= 0).
    NoRefund,
}

/// Borrowed inputs for [`Database::process_gated_refund`].
pub struct GatedRefundInput<'a> {
    pub contract_id: &'a [u8],
    pub requester_pubkey: &'a [u8],
    pub refund_e9s: i64,
    pub reason: &'a str,
    pub payment_intent_id: &'a str,
    pub currency: &'a str,
    pub stripe_dispute_id: Option<&'a str>,
    pub stripe_client: Option<&'a crate::stripe_client::StripeClient>,
}

impl Database {
    /// Look up the user's most recent succeeded Stripe payment across ALL
    /// contracts. This is the cap: any auto-refund above this amount is held
    /// for admin approval. Returns `None` when the user has no succeeded
    /// Stripe payment (cap = 0, so any non-zero refund is held).
    pub async fn get_user_latest_stripe_payment(
        &self,
        requester_pubkey: &[u8],
    ) -> Result<Option<i64>> {
        let amount: Option<i64> = sqlx::query_scalar(
            r#"SELECT payment_amount_e9s
                 FROM contract_sign_requests
                WHERE requester_pubkey = $1
                  AND payment_method = 'stripe'
                  AND payment_status = 'succeeded'
                ORDER BY created_at_ns DESC
                LIMIT 1"#,
        )
        .bind(requester_pubkey)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to query user latest stripe payment for pubkey {}",
                hex::encode(requester_pubkey)
            )
        })?;
        Ok(amount)
    }

    /// Central refund gate. Replaces direct `issue_audited_refund` calls in
    /// ALL refund paths (cancel, reject, dispute_lost, provisioning_failed).
    ///
    /// 1. Creates a `refund_requests` row FIRST (unbypassable audit).
    /// 2. If `refund_e9s <= user_latest_payment` → `status='auto_issued'` →
    ///    calls `issue_audited_refund` → Telegram alert.
    /// 3. If `refund_e9s > user_latest_payment` → `status='pending'` →
    ///    Telegram alert → no Stripe call.
    pub async fn process_gated_refund(
        &self,
        input: GatedRefundInput<'_>,
    ) -> Result<RefundGateOutcome> {
        if input.refund_e9s <= 0 {
            return Ok(RefundGateOutcome::NoRefund);
        }

        let user_latest_e9s = self
            .get_user_latest_stripe_payment(input.requester_pubkey)
            .await?
            .unwrap_or(0);
        let cap_exceeded = input.refund_e9s > user_latest_e9s;
        let now_ns = crate::now_ns()?;

        let idempotency_key = build_idempotency_key(
            input.reason,
            input.contract_id,
            input.stripe_dispute_id,
            now_ns,
        );

        let status = if cap_exceeded { "pending" } else { "auto_issued" };

        let refund_request_id: i64 = sqlx::query_scalar(
            r#"INSERT INTO refund_requests (
                   contract_id, requester_pubkey, refund_amount_e9s, reason,
                   status, user_latest_payment_e9s, cap_exceeded,
                   payment_intent_id, currency, stripe_dispute_id,
                   idempotency_key, created_at_ns
               ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
               ON CONFLICT (contract_id, reason)
               DO UPDATE SET contract_id = EXCLUDED.contract_id
               RETURNING id"#,
        )
        .bind(input.contract_id)
        .bind(input.requester_pubkey)
        .bind(input.refund_e9s)
        .bind(input.reason)
        .bind(status)
        .bind(user_latest_e9s)
        .bind(cap_exceeded)
        .bind(input.payment_intent_id)
        .bind(input.currency)
        .bind(input.stripe_dispute_id)
        .bind(&idempotency_key)
        .bind(now_ns)
        .fetch_one(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to create refund_request for contract {} reason {}",
                hex::encode(input.contract_id),
                input.reason
            )
        })?;

        let refund_cents = input.refund_e9s / 10_000_000;

        if cap_exceeded {
            crate::notifications::telegram::send_ops_alert(&format!(
                "💰 Refund PENDING APPROVAL: contract {} | {} e9s ({} cents) | reason: {} | \
                 exceeds user's latest payment of {} e9s — admin review required",
                hex::encode(input.contract_id),
                input.refund_e9s,
                refund_cents,
                input.reason,
                user_latest_e9s
            ))
            .await;

            tracing::warn!(
                contract_id = %hex::encode(input.contract_id),
                refund_e9s = input.refund_e9s,
                user_latest_e9s,
                reason = input.reason,
                "Refund held for admin approval: exceeds user's latest payment"
            );

            return Ok(RefundGateOutcome::PendingApproval {
                refund_amount_e9s: input.refund_e9s,
                user_latest_payment_e9s: user_latest_e9s,
            });
        }

        // Cap passed — auto-issue via the existing audited refund path.
        let stripe_refund_id = self
            .issue_audited_refund(crate::database::refund_audit::AuditedRefundInput {
                contract_id: input.contract_id,
                idempotency_key: &idempotency_key,
                payment_intent_id: input.payment_intent_id,
                refund_cents,
                currency: input.currency,
                reason: input.reason,
                stripe_dispute_id: input.stripe_dispute_id,
                stripe_client: input.stripe_client,
            })
            .await?;

        if let Some(ref rid) = stripe_refund_id {
            self.mark_refund_request_issued(refund_request_id, rid)
                .await?;
        }

        crate::notifications::telegram::send_ops_alert(&format!(
            "💸 Refund auto-issued: contract {} | {} e9s ({} cents) | reason: {} | refund_id: {}",
            hex::encode(input.contract_id),
            input.refund_e9s,
            refund_cents,
            input.reason,
            stripe_refund_id.as_deref().unwrap_or("(dry-run — no Stripe client)")
        ))
        .await;

        tracing::info!(
            contract_id = %hex::encode(input.contract_id),
            refund_e9s = input.refund_e9s,
            reason = input.reason,
            stripe_refund_id = ?stripe_refund_id,
            "Refund auto-issued (cap passed)"
        );

        Ok(RefundGateOutcome::AutoIssued {
            refund_amount_e9s: input.refund_e9s,
            stripe_refund_id,
        })
    }

    /// Count refund requests, optionally filtered by status. Used for pagination.
    pub async fn count_refund_requests(&self, status_filter: Option<&str>) -> Result<i64> {
        let count: i64 = if let Some(status) = status_filter {
            sqlx::query_scalar(
                r#"SELECT COUNT(*) FROM refund_requests WHERE status = $1"#,
            )
            .bind(status)
            .fetch_one(&self.pool)
            .await
        } else {
            sqlx::query_scalar(r#"SELECT COUNT(*) FROM refund_requests"#)
                .fetch_one(&self.pool)
                .await
        }
        .with_context(|| {
            format!(
                "Failed to count refund_requests (status={:?})",
                status_filter
            )
        })?;
        Ok(count)
    }

    /// List refund requests, optionally filtered by status. Ordered newest-first.
    pub async fn list_refund_requests(
        &self,
        status_filter: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<RefundRequest>> {
        let rows = if let Some(status) = status_filter {
            sqlx::query_as(
                r#"SELECT id, contract_id, requester_pubkey, refund_amount_e9s,
                          reason, status, user_latest_payment_e9s, cap_exceeded,
                          payment_intent_id, currency, stripe_dispute_id,
                          stripe_refund_id, idempotency_key, created_at_ns,
                          reviewed_at_ns, reviewed_by, review_note
                     FROM refund_requests
                    WHERE status = $1
                    ORDER BY created_at_ns DESC
                    LIMIT $2 OFFSET $3"#,
            )
            .bind(status)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as(
                r#"SELECT id, contract_id, requester_pubkey, refund_amount_e9s,
                          reason, status, user_latest_payment_e9s, cap_exceeded,
                          payment_intent_id, currency, stripe_dispute_id,
                          stripe_refund_id, idempotency_key, created_at_ns,
                          reviewed_at_ns, reviewed_by, review_note
                     FROM refund_requests
                    ORDER BY created_at_ns DESC
                    LIMIT $1 OFFSET $2"#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
        }
        .with_context(|| format!("Failed to list refund_requests (status={:?})", status_filter))?;

        Ok(rows)
    }

    /// Approve a pending refund request: marks it approved, issues the Stripe
    /// refund, records the refund id. Returns the updated row.
    ///
    /// This is the admin-triggered completion of a gate-held refund.
    pub async fn approve_refund_request(
        &self,
        request_id: i64,
        admin_pubkey: &[u8],
        review_note: Option<&str>,
        stripe_client: Option<&crate::stripe_client::StripeClient>,
    ) -> Result<RefundRequest> {
        let now_ns = crate::now_ns()?;

        // Atomically flip status pending → approved so two admins clicking
        // simultaneously cannot both issue the refund.
        let row: RefundRequest = sqlx::query_as(
            r#"UPDATE refund_requests
                  SET status = 'approved',
                      reviewed_at_ns = $1,
                      reviewed_by = $2,
                      review_note = $3
                WHERE id = $4 AND status = 'pending'
            RETURNING id, contract_id, requester_pubkey, refund_amount_e9s,
                      reason, status, user_latest_payment_e9s, cap_exceeded,
                      payment_intent_id, currency, stripe_dispute_id,
                      stripe_refund_id, idempotency_key, created_at_ns,
                      reviewed_at_ns, reviewed_by, review_note"#,
        )
        .bind(now_ns)
        .bind(admin_pubkey)
        .bind(review_note)
        .bind(request_id)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to approve refund_request {}", request_id))?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Refund request {} not found or not in 'pending' status",
                request_id
            )
        })?;

        // Issue the actual Stripe refund.
        let refund_cents = row.refund_amount_e9s / 10_000_000;
        let stripe_refund_id = self
            .issue_audited_refund(crate::database::refund_audit::AuditedRefundInput {
                contract_id: &row.contract_id,
                idempotency_key: &row.idempotency_key,
                payment_intent_id: &row.payment_intent_id,
                refund_cents,
                currency: &row.currency,
                reason: &row.reason,
                stripe_dispute_id: row.stripe_dispute_id.as_deref(),
                stripe_client,
            })
            .await?;

        if let Some(ref rid) = stripe_refund_id {
            self.mark_refund_request_issued(row.id, rid).await?;

            // Update the contract row with refund details. The contract may
            // have been deleted (GDPR / account deletion); best-effort, not
            // a failure.
            sqlx::query(
                r#"UPDATE contract_sign_requests
                      SET stripe_refund_id = $1,
                          refund_amount_e9s = $2,
                          refund_created_at_ns = $3,
                          payment_status = 'refunded'
                    WHERE contract_id = $4"#,
            )
            .bind(rid)
            .bind(row.refund_amount_e9s)
            .bind(now_ns)
            .bind(&row.contract_id)
            .execute(&self.pool)
            .await
            .with_context(|| {
                format!(
                    "Failed to update contract {} after approved refund",
                    hex::encode(&row.contract_id)
                )
            })?;
        }

        crate::notifications::telegram::send_ops_alert(&format!(
            "✅ Refund APPROVED & ISSUED: request #{} | contract {} | {} cents | reason: {} | refund_id: {}",
            row.id,
            hex::encode(&row.contract_id),
            refund_cents,
            row.reason,
            stripe_refund_id.as_deref().unwrap_or("(dry-run)")
        ))
        .await;

        tracing::info!(
            request_id = row.id,
            contract_id = %hex::encode(&row.contract_id),
            refund_cents,
            "Admin approved refund request and issued Stripe refund"
        );

        // Re-fetch to include the updated stripe_refund_id
        self.get_refund_request(row.id).await
    }

    /// Decline a pending refund request. No Stripe refund is issued.
    pub async fn decline_refund_request(
        &self,
        request_id: i64,
        admin_pubkey: &[u8],
        review_note: Option<&str>,
    ) -> Result<RefundRequest> {
        let now_ns = crate::now_ns()?;

        let row: RefundRequest = sqlx::query_as(
            r#"UPDATE refund_requests
                  SET status = 'declined',
                      reviewed_at_ns = $1,
                      reviewed_by = $2,
                      review_note = $3
                WHERE id = $4 AND status = 'pending'
            RETURNING id, contract_id, requester_pubkey, refund_amount_e9s,
                      reason, status, user_latest_payment_e9s, cap_exceeded,
                      payment_intent_id, currency, stripe_dispute_id,
                      stripe_refund_id, idempotency_key, created_at_ns,
                      reviewed_at_ns, reviewed_by, review_note"#,
        )
        .bind(now_ns)
        .bind(admin_pubkey)
        .bind(review_note)
        .bind(request_id)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("Failed to decline refund_request {}", request_id))?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Refund request {} not found or not in 'pending' status",
                request_id
            )
        })?;

        crate::notifications::telegram::send_ops_alert(&format!(
            "❌ Refund DECLINED: request #{} | contract {} | {} cents | reason: {}",
            row.id,
            hex::encode(&row.contract_id),
            row.refund_amount_e9s / 10_000_000,
            row.reason
        ))
        .await;

        tracing::info!(
            request_id = row.id,
            contract_id = %hex::encode(&row.contract_id),
            "Admin declined refund request"
        );

        Ok(row)
    }

    /// Fetch a single refund request by id.
    pub async fn get_refund_request(&self, request_id: i64) -> Result<RefundRequest> {
        sqlx::query_as(
            r#"SELECT id, contract_id, requester_pubkey, refund_amount_e9s,
                      reason, status, user_latest_payment_e9s, cap_exceeded,
                      payment_intent_id, currency, stripe_dispute_id,
                      stripe_refund_id, idempotency_key, created_at_ns,
                      reviewed_at_ns, reviewed_by, review_note
                 FROM refund_requests
                WHERE id = $1"#,
        )
        .bind(request_id)
        .fetch_one(&self.pool)
        .await
        .with_context(|| format!("Failed to fetch refund_request {}", request_id))
    }

    /// Record the Stripe refund id on a refund_requests row after a successful
    /// Stripe call (auto-issued or admin-approved).
    async fn mark_refund_request_issued(
        &self,
        request_id: i64,
        stripe_refund_id: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"UPDATE refund_requests
                  SET stripe_refund_id = $1
                WHERE id = $2"#,
        )
        .bind(stripe_refund_id)
        .bind(request_id)
        .execute(&self.pool)
        .await
        .with_context(|| {
            format!(
                "Failed to mark refund_request {} issued with stripe_refund_id {}",
                request_id, stripe_refund_id
            )
        })?;
        Ok(())
    }
}

/// Build the idempotency key for a refund request. Matches the layout from
/// `crate::refund::refund_idempotency_key` so Stripe retries collapse onto one
/// Refund record.
fn build_idempotency_key(
    reason: &str,
    contract_id: &[u8],
    stripe_dispute_id: Option<&str>,
    now_ns: i64,
) -> String {
    if reason == "dispute_lost" {
        if let Some(d) = stripe_dispute_id {
            return format!("dispute:{}", d);
        }
    }
    let token = format!("{}:{}", reason, now_ns);
    crate::refund::refund_idempotency_key(reason, contract_id, &token)
}
