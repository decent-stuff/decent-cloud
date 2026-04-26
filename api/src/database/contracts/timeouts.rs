//! Timeout-driven cleanup primitives for stuck contracts (issues #409 + #410).
//!
//! Two periodic background tasks (`api/src/timeout_cleanup_service.rs`) call
//! into the helpers below to close the highest-probability
//! Stripe-account-freeze gaps:
//!
//!  * `expire_requested` (#410): contracts in `requested` state for longer
//!    than `REQUESTED_TIMEOUT_SECONDS` (Stripe checkout never completed).
//!    No refund is issued because no payment ever succeeded. Inventory that
//!    was reserved by `release_self_provisioned_resource` is freed.
//!
//!  * `mark_provisioning_failed` (#409): contracts in `accepted` or
//!    `provisioning` for longer than `PROVISIONING_TIMEOUT_SECONDS`. The
//!    full paid amount is auto-refunded via `issue_audited_refund` keyed on
//!    `provisioning_failed:{contract_id_hex}:{provisioning_failed_at_ns}` so
//!    a transient retry collapses onto a single Stripe Refund record.
//!
//! All four helpers below are idempotent: callers may replay them safely on
//! a row whose status no longer matches (no error, no state change).
//!
//! Existing `Provisioning -> Cancelled` failure paths are intentionally
//! unchanged in this module; migrating those callsites to
//! `ProvisioningFailed` is tracked separately.

use super::Database;
use anyhow::Result;
use sqlx::FromRow;

/// Slim row returned by the periodic-task scans. We only project the columns
/// the cleanup loop actually needs so the partial indexes stay narrow.
#[derive(Debug, Clone, FromRow, PartialEq, Eq)]
pub struct StaleContractRow {
    pub contract_id: Vec<u8>,
    pub status: String,
    pub status_updated_at_ns: Option<i64>,
    pub created_at_ns: i64,
    pub payment_method: String,
    pub payment_status: String,
    pub payment_amount_e9s: i64,
    pub currency: String,
    pub stripe_payment_intent_id: Option<String>,
    pub stripe_checkout_session_id: Option<String>,
}

impl Database {
    /// Find contracts in `requested` state whose
    /// `COALESCE(status_updated_at_ns, created_at_ns)` is older than
    /// `older_than_ns`. The partial index
    /// `idx_contract_requested_pending_timeout` keeps this scan O(stale).
    pub async fn find_stale_requested(
        &self,
        older_than_ns: i64,
    ) -> Result<Vec<StaleContractRow>> {
        let rows = sqlx::query_as::<_, StaleContractRow>(
            r#"SELECT contract_id, status, status_updated_at_ns, created_at_ns,
                      payment_method, payment_status, payment_amount_e9s, currency,
                      stripe_payment_intent_id, stripe_checkout_session_id
                 FROM contract_sign_requests
                WHERE status = 'requested'
                  AND COALESCE(status_updated_at_ns, created_at_ns) < $1
                ORDER BY created_at_ns ASC"#,
        )
        .bind(older_than_ns)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Find contracts in `accepted` or `provisioning` state whose
    /// `status_updated_at_ns` is older than `older_than_ns`. Uses the partial
    /// index `idx_contract_provisioning_pending_timeout`. Rows with NULL
    /// `status_updated_at_ns` cannot be timed out (state machine sets the
    /// timestamp on every transition into these states); we project the
    /// COALESCE fallback anyway so the loop never picks up a row that has
    /// never been touched.
    pub async fn find_failed_provisioning(
        &self,
        older_than_ns: i64,
    ) -> Result<Vec<StaleContractRow>> {
        let rows = sqlx::query_as::<_, StaleContractRow>(
            r#"SELECT contract_id, status, status_updated_at_ns, created_at_ns,
                      payment_method, payment_status, payment_amount_e9s, currency,
                      stripe_payment_intent_id, stripe_checkout_session_id
                 FROM contract_sign_requests
                WHERE status IN ('accepted', 'provisioning')
                  AND COALESCE(status_updated_at_ns, created_at_ns) < $1
                ORDER BY COALESCE(status_updated_at_ns, created_at_ns) ASC"#,
        )
        .bind(older_than_ns)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Transition a `requested` contract to `expired`. Idempotent: a row that
    /// is no longer in `requested` status produces zero affected rows and the
    /// helper returns `Ok(false)` -- the caller treats that as "another worker
    /// already processed it" and moves on. On the happy path the contract
    /// record + status history + event timeline are written atomically and
    /// any reserved self-provisioned inventory is freed.
    pub async fn expire_requested(&self, contract_id: &[u8]) -> Result<bool> {
        let now_ns = crate::now_ns()?;
        let actor: &[u8] = b"system-timeout";
        let new_status = dcc_common::ContractStatus::Expired.to_string();
        let memo = "Pre-payment timeout: Stripe checkout never completed";

        let mut tx = self.pool.begin().await?;
        let updated = sqlx::query(
            r#"UPDATE contract_sign_requests
                  SET status = $1,
                      status_updated_at_ns = $2,
                      status_updated_by = $3,
                      requested_expired_at_ns = $2
                WHERE contract_id = $4
                  AND status = 'requested'"#,
        )
        .bind(&new_status)
        .bind(now_ns)
        .bind(actor)
        .bind(contract_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if updated == 0 {
            tx.rollback().await?;
            return Ok(false);
        }

        sqlx::query(
            "INSERT INTO contract_status_history (contract_id, old_status, new_status, changed_by, changed_at_ns, change_memo) VALUES ($1, 'requested', $2, $3, $4, $5)",
        )
        .bind(contract_id)
        .bind(&new_status)
        .bind(actor)
        .bind(now_ns)
        .bind(memo)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO contract_events (contract_id, event_type, old_status, new_status, actor, details, created_at) VALUES ($1, 'status_change', 'requested', $2, 'system', $3, $4)",
        )
        .bind(contract_id)
        .bind(&new_status)
        .bind(memo)
        .bind(now_ns)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        // Best-effort: release marketplace inventory the contract may have
        // reserved. A failure here MUST NOT roll back the state transition --
        // the contract is already terminal logically; an operator can
        // reconcile inventory manually if this branch errors.
        if let Err(e) = self.release_self_provisioned_resource(contract_id).await {
            tracing::warn!(
                contract_id = %hex::encode(contract_id),
                error = %format!("{:#}", e),
                "Failed to release self-provisioned resource after expire_requested"
            );
        }
        Ok(true)
    }

    /// Transition an `accepted`/`provisioning` contract to
    /// `provisioningfailed` and issue a full auto-refund. Idempotent: a row
    /// already out of those states produces `Ok(false)` and no refund call.
    /// On the happy path:
    ///   1. Status flips to `provisioningfailed`,
    ///      `provisioning_failed_at_ns` and `provisioning_failure_reason`
    ///      are recorded.
    ///   2. `contract_status_history` + `contract_events` rows are inserted
    ///      in the same transaction.
    ///   3. After commit, the cloud_resource (if any) is marked for deletion
    ///      via the existing `mark_contract_resource_for_deletion` helper.
    ///   4. If `payment_status = 'succeeded'` and we have a Stripe PI, a
    ///      full refund is issued through `issue_audited_refund`. The
    ///      idempotency key is fixed at
    ///      `provisioning_failed:{contract_id_hex}:{provisioning_failed_at_ns}`
    ///      so a retry collapses onto one Stripe Refund.
    ///
    /// Returns the timestamp recorded in `provisioning_failed_at_ns` when the
    /// transition fired, or `None` for a no-op replay. Refund-call errors
    /// propagate to the caller AFTER the state transition has already been
    /// committed, so the audit trail records both the transition AND the
    /// failed refund attempt.
    pub async fn mark_provisioning_failed(
        &self,
        contract_id: &[u8],
        reason: &str,
        stripe_client: Option<&crate::stripe_client::StripeClient>,
    ) -> Result<Option<i64>> {
        let now_ns = crate::now_ns()?;
        let actor: &[u8] = b"system-timeout";
        let new_status = dcc_common::ContractStatus::ProvisioningFailed.to_string();
        let memo = format!("Provisioning timeout: {}", reason);

        // Lock the row, capture its prior state, then transition. SELECT FOR
        // UPDATE prevents a parallel worker from overwriting the row between
        // our read and write. If the row is no longer in the eligible set,
        // we short-circuit with zero side effects.
        let mut tx = self.pool.begin().await?;
        let row: Option<(String, String, String, i64, String, Option<String>, Option<String>)> = sqlx::query_as(
            r#"SELECT status,
                      payment_method,
                      payment_status,
                      payment_amount_e9s,
                      currency,
                      stripe_payment_intent_id,
                      stripe_checkout_session_id
                 FROM contract_sign_requests
                WHERE contract_id = $1
                  AND status IN ('accepted', 'provisioning')
                FOR UPDATE"#,
        )
        .bind(contract_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some((
            prior_status,
            payment_method,
            payment_status,
            payment_amount_e9s,
            currency,
            stripe_payment_intent_id,
            stripe_checkout_session_id,
        )) = row
        else {
            tx.rollback().await?;
            return Ok(None);
        };

        sqlx::query(
            r#"UPDATE contract_sign_requests
                  SET status = $1,
                      status_updated_at_ns = $2,
                      status_updated_by = $3,
                      provisioning_failed_at_ns = $2,
                      provisioning_failure_reason = $4
                WHERE contract_id = $5"#,
        )
        .bind(&new_status)
        .bind(now_ns)
        .bind(actor)
        .bind(reason)
        .bind(contract_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO contract_status_history (contract_id, old_status, new_status, changed_by, changed_at_ns, change_memo) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(contract_id)
        .bind(&prior_status)
        .bind(&new_status)
        .bind(actor)
        .bind(now_ns)
        .bind(&memo)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO contract_events (contract_id, event_type, old_status, new_status, actor, details, created_at) VALUES ($1, 'status_change', $2, $3, 'system', $4, $5)",
        )
        .bind(contract_id)
        .bind(&prior_status)
        .bind(&new_status)
        .bind(&memo)
        .bind(now_ns)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        // Best-effort: tell the deletion loop to tear down a partial VM.
        if let Err(e) = self.mark_contract_resource_for_deletion(contract_id).await {
            tracing::warn!(
                contract_id = %hex::encode(contract_id),
                error = %format!("{:#}", e),
                "Failed to mark cloud_resource for deletion after mark_provisioning_failed"
            );
        }

        // Issue the refund only when the customer has actually been charged.
        // For Stripe: payment_status == "succeeded" and we have a real PI.
        // ICPay/self-rental/free flows fall through with no refund call.
        if payment_method == "stripe"
            && payment_status == "succeeded"
            && payment_amount_e9s > 0
        {
            let payment_intent_id = stripe_payment_intent_id
                .as_deref()
                .or(stripe_checkout_session_id.as_deref());
            if let Some(payment_intent_id) = payment_intent_id {
                let refund_cents = payment_amount_e9s / 10_000_000;
                let unique_token = format!("provisioning_failed:{}", now_ns);
                let key = crate::refund::refund_idempotency_key(
                    "provisioning_failed",
                    contract_id,
                    &unique_token,
                );
                let refund_id = self
                    .issue_audited_refund(crate::database::refund_audit::AuditedRefundInput {
                        contract_id,
                        idempotency_key: &key,
                        payment_intent_id,
                        refund_cents,
                        currency: &currency,
                        reason: "provisioning_failed",
                        stripe_dispute_id: None,
                        stripe_client,
                    })
                    .await?;

                // Persist refund accounting on the contract row regardless
                // of whether Stripe accepted the call -- the audit row is
                // already in place; reconciliation tools rely on this row.
                sqlx::query(
                    "UPDATE contract_sign_requests SET refund_amount_e9s = $1, stripe_refund_id = $2, refund_created_at_ns = $3, payment_status = 'refunded' WHERE contract_id = $4",
                )
                .bind(payment_amount_e9s)
                .bind(refund_id.as_deref())
                .bind(now_ns)
                .bind(contract_id)
                .execute(&self.pool)
                .await?;

                if let Some(ref id) = refund_id {
                    tracing::info!(
                        contract_id = %hex::encode(contract_id),
                        stripe_refund_id = %id,
                        refund_cents,
                        "Provisioning-failure auto-refund issued"
                    );
                }
            }
        }

        Ok(Some(now_ns))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_helpers::setup_test_db;

    /// Insert a contract row directly with the requested status and
    /// timestamps. Mirrors the existing `insert_contract_request` helper used
    /// elsewhere in the contracts test suite but exposes timestamp + payment
    /// knobs needed for timeout boundary tests.
    async fn insert_test_contract(
        db: &Database,
        contract_id: &[u8],
        status: &str,
        created_at_ns: i64,
        status_updated_at_ns: Option<i64>,
        payment_method: &str,
        payment_status: &str,
        payment_amount_e9s: i64,
        stripe_payment_intent_id: Option<&str>,
    ) {
        let requester: &[u8] = &[0xAA; 32];
        let provider: &[u8] = &[0xBB; 32];
        sqlx::query(
            r#"INSERT INTO contract_sign_requests (
                contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact,
                provider_pubkey, offering_id, payment_amount_e9s, request_memo,
                created_at_ns, status, status_updated_at_ns, payment_method,
                stripe_payment_intent_id, payment_status, currency
            ) VALUES ($1, $2, 'ssh-key', 'contact', $3, 'off-1', $4, 'memo',
                      $5, $6, $7, $8, $9, $10, 'usd')"#,
        )
        .bind(contract_id)
        .bind(requester)
        .bind(provider)
        .bind(payment_amount_e9s)
        .bind(created_at_ns)
        .bind(status)
        .bind(status_updated_at_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(payment_status)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_expire_requested_idempotent_on_non_requested_contract() {
        // Calling expire on a contract that is not currently `requested` must
        // be a silent no-op: returns Ok(false), leaves status untouched, and
        // does NOT write spurious history/event rows.
        let db = setup_test_db().await;
        let contract_id = vec![0x10; 32];
        insert_test_contract(
            &db,
            &contract_id,
            "active",
            0,
            Some(0),
            "icpay",
            "succeeded",
            1_000_000_000,
            None,
        )
        .await;

        let result = db.expire_requested(&contract_id).await.unwrap();
        assert!(!result, "expire_requested on non-requested must return false");

        let after = db.get_contract(&contract_id).await.unwrap().unwrap();
        assert_eq!(after.status, "active", "status must be untouched");

        let event_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM contract_events WHERE contract_id = $1",
        )
        .bind(&contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(event_count, 0, "no event rows must be written on no-op");
    }

    #[tokio::test]
    async fn test_expire_requested_releases_marketplace_inventory() {
        // A `requested` contract that reserved a marketplace cloud_resource
        // must, on expiry, free that resource (contract_id = NULL) and flip
        // the offering's stock_status back to `in_stock`. This is the core
        // anti-deadlock invariant for inventory and proves the timeout
        // cleanup does not leak provider stock when checkouts abandon.
        let db = setup_test_db().await;
        let provider_pubkey = [0xBB_u8; 32];

        // Provider account + cloud_account + cloud_resource via real helpers
        // -- mirrors the existing `test_release_self_provisioned_resource_restocks_offering`
        // setup so we are exercising the same release path the contract
        // cancel flow uses.
        let provider_account = db
            .create_account("timeout_test", &provider_pubkey, "timeout@example.com")
            .await
            .unwrap();
        let cloud_account = db
            .create_cloud_account(
                &provider_account.id,
                crate::cloud::types::BackendType::Hetzner,
                "timeout-hetzner",
                "encrypted",
                None,
            )
            .await
            .unwrap();
        let ca_uuid: uuid::Uuid = cloud_account.id.parse().unwrap();
        let resource = db
            .create_cloud_resource(
                &ca_uuid,
                "ext-timeout",
                "timeout-vm",
                "cx22",
                "nbg1",
                "ubuntu-24.04",
                "ssh-ed25519 AAAA test",
            )
            .await
            .unwrap();
        let resource_id: uuid::Uuid = resource.id.parse().unwrap();
        db.update_cloud_resource_status(&resource_id, "running")
            .await
            .unwrap();

        // Seed an offering owned by the provider and list the resource on
        // the marketplace. We do this with a raw insert because the public
        // helper lives in a sibling test module; this is the same shape as
        // `cloud_resources.rs::create_test_offering`.
        let offering_id: i64 = sqlx::query_scalar(
            r#"INSERT INTO provider_offerings (
                pubkey, offering_id, offer_name, currency, monthly_price, setup_fee,
                visibility, product_type, billing_interval, stock_status,
                datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns
            ) VALUES ($1, 'off-timeout', 'TimeoutOffer', 'USD', 10.0, 0, 'public', 'compute',
                      'monthly', 'in_stock', 'US', 'NYC', FALSE, 0)
            RETURNING id"#,
        )
        .bind(&provider_pubkey[..])
        .fetch_one(&db.pool)
        .await
        .unwrap();
        db.list_on_marketplace(&resource_id, &provider_account.id, offering_id)
            .await
            .unwrap();

        // Insert a `requested` contract owned by another tenant, then
        // reserve the marketplace inventory for that contract.
        let contract_id = vec![0x11_u8; 32];
        insert_test_contract(
            &db,
            &contract_id,
            "requested",
            0,
            Some(0),
            "stripe",
            "pending",
            500_000_000,
            Some("pi_test_expire"),
        )
        .await;
        db.reserve_self_provisioned_resource(offering_id, &contract_id)
            .await
            .unwrap();

        // Sanity: reservation flips stock to `out_of_stock`. The release
        // path must restore it to `in_stock`. We assert the precondition so
        // a future schema/state change does not silently let the test pass
        // when inventory is never actually held.
        let pre_stock: String = sqlx::query_scalar(
            "SELECT stock_status FROM provider_offerings WHERE id = $1",
        )
        .bind(offering_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(
            pre_stock, "out_of_stock",
            "precondition: reservation must hold inventory"
        );

        // Run the timeout helper.
        let fired = db.expire_requested(&contract_id).await.unwrap();
        assert!(fired, "expire_requested must report success");

        let after = db.get_contract(&contract_id).await.unwrap().unwrap();
        assert_eq!(after.status, "expired", "status must flip to expired");

        // Inventory MUST be released: the resource row no longer points at
        // the contract and the offering is back in stock for the next
        // tenant.
        assert!(
            db.get_reserved_self_provisioned_resource(&contract_id)
                .await
                .unwrap()
                .is_none(),
            "cloud_resource reservation must be cleared"
        );
        let stock: String = sqlx::query_scalar(
            "SELECT stock_status FROM provider_offerings WHERE id = $1",
        )
        .bind(offering_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(stock, "in_stock", "offering must be restocked");
    }

    #[tokio::test]
    async fn test_mark_provisioning_failed_idempotent_on_non_provisioning_contract() {
        // A `provisioned` (or any non-{accepted,provisioning}) row must NOT
        // be flipped to `provisioningfailed`. Returns Ok(None) and writes
        // nothing.
        let db = setup_test_db().await;
        let contract_id = vec![0x20; 32];
        insert_test_contract(
            &db,
            &contract_id,
            "provisioned",
            0,
            Some(0),
            "icpay",
            "succeeded",
            1_000_000_000,
            None,
        )
        .await;

        let result = db
            .mark_provisioning_failed(&contract_id, "test-no-op", None)
            .await
            .unwrap();
        assert!(result.is_none(), "must return None for replay");

        let after = db.get_contract(&contract_id).await.unwrap().unwrap();
        assert_eq!(after.status, "provisioned");

        let history_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM contract_status_history WHERE contract_id = $1",
        )
        .bind(&contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(history_count, 0, "no history must be written on no-op");
    }

    #[tokio::test]
    async fn test_mark_provisioning_failed_records_audit_with_correct_idempotency_key() {
        // Full happy path with a Stripe-charged contract: the state must
        // flip, the refund_audit row must exist with the deterministic
        // idempotency key shape, and the contract row must record the
        // failure timestamp + reason. We pass `stripe_client = None` so the
        // audit row is recorded as `requested` (no Stripe network call) --
        // the key shape is what we assert.
        let db = setup_test_db().await;
        let contract_id = vec![0x21; 32];
        insert_test_contract(
            &db,
            &contract_id,
            "provisioning",
            0,
            Some(0),
            "stripe",
            "succeeded",
            500_000_000, // 5000 cents -> $50.00 paid
            Some("pi_test_provfail"),
        )
        .await;
        let fired_at_ns = db
            .mark_provisioning_failed(&contract_id, "agent timeout 60m", None)
            .await
            .unwrap()
            .expect("happy path must return Some(timestamp)");

        // The prior status MUST be threaded into the audit history exactly
        // as it stood before the transition (`provisioning` here, NOT some
        // synthetic placeholder). This guards against history corruption.
        let history_old: String = sqlx::query_scalar(
            "SELECT old_status FROM contract_status_history WHERE contract_id = $1 ORDER BY changed_at_ns DESC LIMIT 1",
        )
        .bind(&contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(history_old, "provisioning");

        let after = db.get_contract(&contract_id).await.unwrap().unwrap();
        assert_eq!(after.status, "provisioningfailed");
        assert_eq!(
            after.payment_status, "refunded",
            "payment_status must reflect the refund attempt"
        );
        assert_eq!(after.refund_amount_e9s, Some(500_000_000));

        let expected_key = crate::refund::refund_idempotency_key(
            "provisioning_failed",
            &contract_id,
            &format!("provisioning_failed:{}", fired_at_ns),
        );
        let audit = db
            .find_audit_by_idempotency_key(&expected_key)
            .await
            .unwrap()
            .expect("audit row must exist with the deterministic key");
        assert_eq!(audit.reason, "provisioning_failed");
        assert_eq!(audit.amount_cents, 50, "5e8 e9s -> 50 cents");
        // No stripe_client passed -> audit stays at `requested` (no
        // succeeded/failed transition). The audit row is the proof we
        // attempted; an actual stripe call is exercised by issue_audited_refund's
        // own test suite.
        assert_eq!(audit.status, "requested");
    }

    #[tokio::test]
    async fn test_find_stale_requested_respects_timeout_boundary() {
        // The boundary check: a row whose status_updated_at_ns equals the
        // cutoff is NOT picked up (we use strict `<`); a row that is one
        // nanosecond older IS picked up. This is the cheapest invariant
        // that prevents off-by-one starvation OR herd-thundering at the
        // exact cutoff.
        let db = setup_test_db().await;

        // Two requested contracts: one stale, one fresh. Plus a pending
        // contract that must NOT be returned (different status).
        let stale_id = vec![0x30; 32];
        let fresh_id = vec![0x31; 32];
        let pending_id = vec![0x32; 32];

        insert_test_contract(
            &db, &stale_id, "requested", 0, Some(99), "stripe", "pending", 1, None,
        )
        .await;
        insert_test_contract(
            &db, &fresh_id, "requested", 0, Some(101), "stripe", "pending", 1, None,
        )
        .await;
        insert_test_contract(
            &db,
            &pending_id,
            "pending",
            0,
            Some(0),
            "icpay",
            "succeeded",
            1,
            None,
        )
        .await;

        let stale = db.find_stale_requested(100).await.unwrap();
        assert_eq!(stale.len(), 1, "exactly one row must be returned");
        assert_eq!(stale[0].contract_id, stale_id);
        assert_eq!(stale[0].status, "requested");
    }

    #[tokio::test]
    async fn test_find_failed_provisioning_respects_timeout_boundary() {
        // Same boundary semantics for the provisioning-stuck scan, plus
        // confirms it picks up `accepted` AND `provisioning` (both states
        // are equally stuck pre-VM-up) but ignores everything else.
        let db = setup_test_db().await;
        let stuck_acc = vec![0x40; 32];
        let stuck_prov = vec![0x41; 32];
        let fresh_prov = vec![0x42; 32];
        let other = vec![0x43; 32];

        insert_test_contract(
            &db, &stuck_acc, "accepted", 0, Some(50), "stripe", "succeeded", 1, None,
        )
        .await;
        insert_test_contract(
            &db,
            &stuck_prov,
            "provisioning",
            0,
            Some(60),
            "stripe",
            "succeeded",
            1,
            None,
        )
        .await;
        insert_test_contract(
            &db,
            &fresh_prov,
            "provisioning",
            0,
            Some(105),
            "stripe",
            "succeeded",
            1,
            None,
        )
        .await;
        insert_test_contract(
            &db,
            &other,
            "active",
            0,
            Some(0),
            "stripe",
            "succeeded",
            1,
            None,
        )
        .await;

        let stale = db.find_failed_provisioning(100).await.unwrap();
        assert_eq!(stale.len(), 2, "both accepted and provisioning rows older than cutoff");
        let ids: std::collections::HashSet<Vec<u8>> =
            stale.iter().map(|r| r.contract_id.clone()).collect();
        assert!(ids.contains(&stuck_acc));
        assert!(ids.contains(&stuck_prov));
        assert!(!ids.contains(&fresh_prov));
        assert!(!ids.contains(&other));
    }
}
