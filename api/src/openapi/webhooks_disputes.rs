//! Stripe `charge.dispute.*` webhook handlers (Phase 2), extracted from
//! `webhooks.rs` as part of issue #444's large-file split.
//!
//! These are plain async functions invoked by `stripe_webhook` (in
//! `webhooks.rs`) for the four `charge.dispute.*` event arms. They persist
//! every dispute event, pause the matching contract while a dispute is open,
//! and resume/terminate when it closes. Replays MUST be idempotent: 2xx is the
//! signal Stripe uses to stop retrying, and the DB primitives in
//! `database/contracts/dispute.rs` are designed for exactly that.
//!
//! These handlers are NOT `#[OpenApi]` endpoints — inbound webhooks are raw
//! `Route::at(..)` registrations in `main.rs` and do not appear in the OpenAPI
//! spec, so this split has zero spec impact (guarded by
//! `openapi::spec_snapshot`).

use crate::database::Database;
use poem::Error as PoemError;
use serde::Deserialize;

// =============================================================================
// Stripe charge.dispute.* webhook types (Phase 2)
// =============================================================================
//
// Stripe sends these events whenever a customer files a chargeback. The
// server must persist every event (so we have an audit trail), pause the
// matching contract while the dispute is open, and either resume or
// terminate when the dispute closes. Replays MUST be idempotent: 2xx is the
// signal Stripe uses to stop retrying, and the DB primitives in
// `contracts/dispute.rs` are designed for exactly that.

#[derive(Debug, Deserialize)]
struct StripeDispute {
    id: String,
    /// Stripe charge ID (`ch_*`). Always present on a `charge.dispute.*` event.
    charge: String,
    /// PaymentIntent ID (`pi_*`). Stripe omits this on legacy charges that
    /// were not created via PaymentIntents -- contracts older than the
    /// session/PI split therefore fall back to charge-id lookup.
    #[serde(default)]
    payment_intent: Option<String>,
    /// Disputed amount in the smallest currency unit (cents for USD/EUR).
    amount: i64,
    currency: String,
    /// Free-form Stripe-provided dispute reason (e.g. "fraudulent",
    /// "product_not_received"). Persisted verbatim.
    #[serde(default)]
    reason: Option<String>,
    /// Stripe-side dispute status: e.g. `needs_response`, `under_review`,
    /// `won`, `lost`, `warning_closed`. We forward the raw value to the DB.
    status: String,
    #[serde(default)]
    evidence_details: Option<StripeDisputeEvidenceDetails>,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct StripeDisputeEvidenceDetails {
    /// Unix seconds; Stripe's deadline for evidence submission.
    #[serde(default)]
    due_by: Option<i64>,
}

// =============================================================================
// Dispute handler implementations (Phase 2)
// =============================================================================

fn parse_dispute(object: &serde_json::Value) -> Result<StripeDispute, PoemError> {
    serde_json::from_value(object.clone()).map_err(|e| {
        tracing::error!("Failed to parse dispute payload: {:#}", e);
        PoemError::from_string(
            format!("Invalid dispute data: {}", e),
            poem::http::StatusCode::BAD_REQUEST,
        )
    })
}

fn map_db_err(context: &'static str, e: anyhow::Error) -> PoemError {
    tracing::error!("{}: {:#}", context, e);
    PoemError::from_string(
        format!("{}: {}", context, e),
        poem::http::StatusCode::INTERNAL_SERVER_ERROR,
    )
}

/// Resolve a Stripe dispute to one of our contracts.
///
/// Lookup chain (most-specific first):
///  1. `metadata.contract_id` (set by us when creating the checkout session).
///  2. `payment_intent` -> `stripe_payment_intent_id` (canonical post-rename).
///  3. `payment_intent` against the legacy `stripe_checkout_session_id`
///     column (covered by the same DB helper for legacy rows).
///  4. `charge` -> previously-seen dispute row (so a `dispute.updated` for
///     a known dispute still finds the contract).
///
/// Returns `None` for charges we never issued -- the caller logs and pages
/// ops but does NOT 500 (Stripe would retry forever).
async fn lookup_contract_for_charge(
    db: &Database,
    dispute: &StripeDispute,
) -> Option<Vec<u8>> {
    if let Some(meta) = dispute.metadata.as_ref() {
        if let Some(hex_id) = meta.get("contract_id").and_then(|v| v.as_str()) {
            match hex::decode(hex_id) {
                Ok(bytes) => return Some(bytes),
                Err(e) => tracing::warn!(
                    dispute_id = %dispute.id,
                    charge = %dispute.charge,
                    contract_id_meta = %hex_id,
                    error = %e,
                    "dispute metadata contract_id is not valid hex; falling through to payment_intent lookup"
                ),
            }
        }
    }
    if let Some(pi) = dispute.payment_intent.as_deref() {
        match db.get_contract_id_by_stripe_payment_intent(pi).await {
            Ok(Some(id)) => return Some(id),
            Ok(None) => {}
            Err(e) => tracing::warn!(
                payment_intent = %pi,
                error = %format!("{:#}", e),
                "DB lookup by stripe_payment_intent failed; continuing fallback chain"
            ),
        }
    }
    match db.get_contract_id_by_stripe_charge(&dispute.charge).await {
        Ok(Some(id)) => Some(id),
        Ok(None) => None,
        Err(e) => {
            tracing::warn!(
                charge = %dispute.charge,
                error = %format!("{:#}", e),
                "DB lookup by stripe_charge failed; treating dispute as orphan"
            );
            None
        }
    }
}

fn evidence_due_by_ns(d: &StripeDispute) -> Option<i64> {
    d.evidence_details
        .as_ref()
        .and_then(|e| e.due_by)
        .map(|s| s * 1_000_000_000)
}

fn upsert_input<'a>(
    contract_id: Option<&'a [u8]>,
    d: &'a StripeDispute,
    raw: &'a serde_json::Value,
    funds_withdrawn_at_ns: Option<i64>,
    closed_at_ns: Option<i64>,
) -> crate::database::ContractDisputeUpsert<'a> {
    crate::database::ContractDisputeUpsert {
        contract_id,
        stripe_dispute_id: &d.id,
        stripe_charge_id: &d.charge,
        stripe_payment_intent_id: d.payment_intent.as_deref(),
        reason: d.reason.as_deref(),
        status: &d.status,
        amount_cents: d.amount,
        currency: &d.currency,
        evidence_due_by_ns: evidence_due_by_ns(d),
        funds_withdrawn_at_ns,
        closed_at_ns,
        raw_event: raw,
    }
}

pub(super) async fn handle_dispute_created(
    db: &Database,
    object: &serde_json::Value,
) -> Result<(), PoemError> {
    let dispute = parse_dispute(object)?;
    tracing::warn!(
        stripe_dispute_id = %dispute.id,
        charge = %dispute.charge,
        amount = dispute.amount,
        currency = %dispute.currency,
        reason = %dispute.reason.as_deref().unwrap_or(""),
        status = %dispute.status,
        "Stripe dispute opened"
    );

    let contract_id = lookup_contract_for_charge(db, &dispute).await;

    db.upsert_contract_dispute(upsert_input(
        contract_id.as_deref(),
        &dispute,
        object,
        None,
        None,
    ))
    .await
    .map_err(|e| map_db_err("upsert_contract_dispute", e))?;

    if let Some(cid) = contract_id {
        let pause_reason = format!("stripe_dispute:{}", dispute.id);
        if let Err(e) = db.pause_contract(&cid, &pause_reason).await {
            // Pause failure is operator-relevant but MUST NOT 500: the
            // dispute row is already persisted, and Stripe replays would
            // produce no new state. Page ops, return Ok.
            tracing::error!(
                contract_id = %hex::encode(&cid),
                stripe_dispute_id = %dispute.id,
                error = %format!("{:#}", e),
                "Failed to pause contract for dispute; row persisted, manual intervention may be required"
            );
            crate::notifications::telegram::send_ops_alert(&format!(
                "Stripe dispute OPENED but pause FAILED for contract {}: id={} err={:#}",
                hex::encode(&cid),
                dispute.id,
                e
            ))
            .await;
        } else {
            crate::notifications::telegram::send_ops_alert(&format!(
                "Stripe dispute OPENED for contract {}: id={} reason={} amount={} {}",
                hex::encode(&cid),
                dispute.id,
                dispute.reason.as_deref().unwrap_or(""),
                dispute.amount,
                dispute.currency
            ))
            .await;
        }
    } else {
        tracing::warn!(
            stripe_dispute_id = %dispute.id,
            charge = %dispute.charge,
            "Stripe dispute has no matching contract (orphan); persisted with NULL contract_id"
        );
        crate::notifications::telegram::send_ops_alert(&format!(
            "Stripe dispute OPENED with NO matching contract: id={} charge={} amount={} {}",
            dispute.id, dispute.charge, dispute.amount, dispute.currency
        ))
        .await;
    }
    Ok(())
}

pub(super) async fn handle_dispute_updated(
    db: &Database,
    object: &serde_json::Value,
) -> Result<(), PoemError> {
    let dispute = parse_dispute(object)?;
    tracing::info!(
        stripe_dispute_id = %dispute.id,
        status = %dispute.status,
        "Stripe dispute updated"
    );
    let contract_id = lookup_contract_for_charge(db, &dispute).await;
    db.upsert_contract_dispute(upsert_input(
        contract_id.as_deref(),
        &dispute,
        object,
        None,
        None,
    ))
    .await
    .map_err(|e| map_db_err("upsert_contract_dispute", e))?;
    Ok(())
}

pub(super) async fn handle_dispute_closed(
    db: &Database,
    object: &serde_json::Value,
) -> Result<(), PoemError> {
    let dispute = parse_dispute(object)?;
    let now_ns = crate::now_ns().map_err(|e| map_db_err("now_ns", e))?;
    let outcome = dispute.status.as_str();
    match outcome {
        "won" => tracing::info!(
            stripe_dispute_id = %dispute.id,
            "Stripe dispute WON"
        ),
        "lost" => tracing::warn!(
            stripe_dispute_id = %dispute.id,
            amount = dispute.amount,
            currency = %dispute.currency,
            "Stripe dispute LOST"
        ),
        other => tracing::info!(
            stripe_dispute_id = %dispute.id,
            status = other,
            "Stripe dispute closed (non-binary outcome)"
        ),
    }

    let contract_id = lookup_contract_for_charge(db, &dispute).await;

    db.upsert_contract_dispute(upsert_input(
        contract_id.as_deref(),
        &dispute,
        object,
        None,
        Some(now_ns),
    ))
    .await
    .map_err(|e| map_db_err("upsert_contract_dispute", e))?;

    let Some(cid) = contract_id else {
        // Closed dispute with no matching contract -- still operator-relevant.
        crate::notifications::telegram::send_ops_alert(&format!(
            "Stripe dispute CLOSED ({}) with NO matching contract: id={} charge={}",
            outcome, dispute.id, dispute.charge
        ))
        .await;
        return Ok(());
    };

    match outcome {
        "won" => {
            if let Err(e) = db.resume_contract(&cid).await {
                tracing::error!(
                    contract_id = %hex::encode(&cid),
                    stripe_dispute_id = %dispute.id,
                    error = %format!("{:#}", e),
                    "Failed to resume contract after dispute won; manual intervention may be required"
                );
                crate::notifications::telegram::send_ops_alert(&format!(
                    "Stripe dispute WON but resume FAILED for contract {}: id={} err={:#}",
                    hex::encode(&cid),
                    dispute.id,
                    e
                ))
                .await;
            }
        }
        "lost" => {
            // Order matters: terminate FIRST (sets payment_status='disputed',
            // emits the audit event, marks the resource for deletion). Refund
            // SECOND so it sees the final paused interval -- terminate does
            // not call resume so total_paused_ns reflects the full pause
            // window.
            if let Err(e) = db
                .terminate_contract_for_dispute_lost(&cid, &dispute.id)
                .await
            {
                tracing::error!(
                    contract_id = %hex::encode(&cid),
                    stripe_dispute_id = %dispute.id,
                    error = %format!("{:#}", e),
                    "Failed to terminate contract for dispute_lost; manual intervention required"
                );
            }

            // Best-effort prorated refund. The Phase 1 helper handles
            // idempotency (key = `dispute:<id>`) so replays collapse onto
            // the same Stripe Refund record.
            let stripe_client = crate::stripe_client::stripe_client_or_warn();
            if let Err(e) = db
                .process_dispute_lost_refund(&cid, &dispute.id, stripe_client.as_ref())
                .await
            {
                tracing::error!(
                    contract_id = %hex::encode(&cid),
                    stripe_dispute_id = %dispute.id,
                    error = %format!("{:#}", e),
                    "Failed to compute/issue dispute-lost refund"
                );
            }

            crate::notifications::telegram::send_ops_alert(&format!(
                "Stripe dispute LOST for contract {}: id={} amount={} {}",
                hex::encode(&cid),
                dispute.id,
                dispute.amount,
                dispute.currency
            ))
            .await;
        }
        // warning_closed and other non-binary statuses: row updated, no transition.
        _ => {}
    }
    Ok(())
}

pub(super) async fn handle_dispute_funds_withdrawn(
    db: &Database,
    object: &serde_json::Value,
) -> Result<(), PoemError> {
    let dispute = parse_dispute(object)?;
    let now_ns = crate::now_ns().map_err(|e| map_db_err("now_ns", e))?;
    tracing::warn!(
        stripe_dispute_id = %dispute.id,
        charge = %dispute.charge,
        amount = dispute.amount,
        currency = %dispute.currency,
        "Stripe dispute funds withdrawn"
    );
    let contract_id = lookup_contract_for_charge(db, &dispute).await;
    db.upsert_contract_dispute(upsert_input(
        contract_id.as_deref(),
        &dispute,
        object,
        Some(now_ns),
        None,
    ))
    .await
    .map_err(|e| map_db_err("upsert_contract_dispute", e))?;

    crate::notifications::telegram::send_ops_alert(&format!(
        "Stripe dispute FUNDS WITHDRAWN: id={} charge={} contract={} amount={} {}",
        dispute.id,
        dispute.charge,
        contract_id
            .as_ref()
            .map(hex::encode)
            .unwrap_or_else(|| "<none>".to_string()),
        dispute.amount,
        dispute.currency
    ))
    .await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Stripe charge.dispute.* end-to-end handler tests (Phase 2).
    //
    // These exercise the dispatch logic directly against a real test DB so we
    // assert the full pause-resume / terminate-refund flow, including
    // idempotent replay (Stripe retries forever on non-2xx responses).
    // The signature-verification path is unit-tested above; here we focus on
    // the handler-side invariants the spec mandates in section 6.
    // =========================================================================

    use crate::database::contracts::dispute::dispute_refund_idempotency_key;
    use crate::database::test_helpers::setup_test_db;

    async fn insert_active_contract(db: &Database, contract_id: &[u8], pi_id: Option<&str>) {
        // Contract started 1 minute ago, ends 1 day from now -> mostly-unused
        // billable window so the prorated lost-dispute refund is large enough
        // (>> 1 cent) to be observably positive in the DB row.
        let now_ns = crate::now_ns().expect("now_ns");
        let one_min_ns: i64 = 60 * 1_000_000_000;
        let one_day_ns: i64 = 24 * 60 * 60 * 1_000_000_000;
        let provisioning_completed_at_ns = now_ns - one_min_ns;
        let end_timestamp_ns = now_ns + one_day_ns;
        sqlx::query!(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, payment_status, currency, provisioning_completed_at_ns, end_timestamp_ns) \
             VALUES ($1, $2, 'ssh-key', 'contact', $3, 'off-1', 100000000000, 'memo', 0, 'active', 'stripe', $4, NULL, 'succeeded', 'usd', $5, $6)",
            contract_id,
            &[1u8; 32][..],
            &[2u8; 32][..],
            pi_id,
            provisioning_completed_at_ns,
            end_timestamp_ns,
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }

    fn dispute_event(
        event_type: &str,
        dispute_id: &str,
        charge: &str,
        payment_intent: Option<&str>,
        status: &str,
        contract_id_hex: Option<&str>,
    ) -> serde_json::Value {
        let mut metadata = serde_json::Map::new();
        if let Some(cid) = contract_id_hex {
            metadata.insert("contract_id".into(), serde_json::json!(cid));
        }
        serde_json::json!({
            "type": event_type,
            "data": {
                "object": {
                    "id": dispute_id,
                    "charge": charge,
                    "payment_intent": payment_intent,
                    "amount": 5_000,
                    "currency": "usd",
                    "reason": "fraudulent",
                    "status": status,
                    "metadata": serde_json::Value::Object(metadata),
                }
            }
        })
    }

    fn unwrap_object(event: &serde_json::Value) -> serde_json::Value {
        event["data"]["object"].clone()
    }

    async fn count_disputes(db: &Database, dispute_id: &str) -> i64 {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM contract_disputes WHERE stripe_dispute_id = $1",
        )
        .bind(dispute_id)
        .fetch_one(&db.pool)
        .await
        .unwrap()
    }

    async fn count_history_to(db: &Database, contract_id: &[u8], new_status: &str) -> i64 {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM contract_status_history WHERE contract_id = $1 AND new_status = $2",
        )
        .bind(contract_id)
        .bind(new_status)
        .fetch_one(&db.pool)
        .await
        .unwrap()
    }

    async fn read_status(db: &Database, contract_id: &[u8]) -> String {
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM contract_sign_requests WHERE contract_id = $1",
        )
        .bind(contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn test_dispute_created_pauses_contract() {
        // dispute.created on a contract we own MUST: insert a row in
        // contract_disputes, transition the contract to `paused`, set
        // paused_at_ns, and emit a single 'paused' history row + event.
        let db = setup_test_db().await;
        let contract_id = vec![0xC1; 32];
        insert_active_contract(&db, &contract_id, Some("pi_test_c1")).await;

        let event = dispute_event(
            "charge.dispute.created",
            "du_c1",
            "ch_c1",
            Some("pi_test_c1"),
            "needs_response",
            None,
        );
        handle_dispute_created(&db, &unwrap_object(&event))
            .await
            .expect("dispute.created handler must succeed against a known contract");

        assert_eq!(count_disputes(&db, "du_c1").await, 1);
        assert_eq!(read_status(&db, &contract_id).await, "paused");
        let paused_at: Option<i64> = sqlx::query_scalar(
            "SELECT paused_at_ns FROM contract_sign_requests WHERE contract_id = $1",
        )
        .bind(&contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert!(
            paused_at.is_some(),
            "paused_at_ns MUST be populated by pause_contract"
        );
        assert_eq!(count_history_to(&db, &contract_id, "paused").await, 1);
    }

    #[tokio::test]
    async fn test_dispute_created_idempotent_on_replay() {
        // Stripe replays the SAME dispute.created event indefinitely until
        // the server returns 2xx. Replays MUST collapse: one dispute row,
        // one transition (one paused history row), no extra audit noise.
        let db = setup_test_db().await;
        let contract_id = vec![0xC2; 32];
        insert_active_contract(&db, &contract_id, Some("pi_test_c2")).await;

        let event = dispute_event(
            "charge.dispute.created",
            "du_c2",
            "ch_c2",
            Some("pi_test_c2"),
            "needs_response",
            None,
        );
        handle_dispute_created(&db, &unwrap_object(&event))
            .await
            .unwrap();
        handle_dispute_created(&db, &unwrap_object(&event))
            .await
            .expect("replay must NOT 5xx");

        assert_eq!(
            count_disputes(&db, "du_c2").await,
            1,
            "exactly one dispute row across replays"
        );
        assert_eq!(
            count_history_to(&db, &contract_id, "paused").await,
            1,
            "exactly one paused history row across replays"
        );
        let paused_events: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM contract_events WHERE contract_id = $1 AND event_type = 'paused'",
        )
        .bind(&contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(
            paused_events, 1,
            "replay MUST NOT emit a second 'paused' audit event"
        );
    }

    #[tokio::test]
    async fn test_dispute_closed_won_resumes_contract() {
        // Pause via dispute.created, sleep, then close=won. Contract must
        // return to `active`, total_paused_ns must reflect the pause window,
        // and the dispute row's status MUST be 'won' with closed_at_ns set.
        let db = setup_test_db().await;
        let contract_id = vec![0xC3; 32];
        insert_active_contract(&db, &contract_id, Some("pi_test_c3")).await;

        let created = dispute_event(
            "charge.dispute.created",
            "du_c3",
            "ch_c3",
            Some("pi_test_c3"),
            "needs_response",
            None,
        );
        handle_dispute_created(&db, &unwrap_object(&created))
            .await
            .unwrap();
        // Sleep enough for ns-resolution to register a positive credit.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let closed = dispute_event(
            "charge.dispute.closed",
            "du_c3",
            "ch_c3",
            Some("pi_test_c3"),
            "won",
            None,
        );
        handle_dispute_closed(&db, &unwrap_object(&closed))
            .await
            .unwrap();

        assert_eq!(read_status(&db, &contract_id).await, "active");
        let total_paused: i64 = sqlx::query_scalar(
            "SELECT total_paused_ns FROM contract_sign_requests WHERE contract_id = $1",
        )
        .bind(&contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert!(
            total_paused >= 10_000_000,
            "total_paused_ns must reflect the pause interval; got {}",
            total_paused
        );
        let row: (String, Option<i64>) = sqlx::query_as(
            "SELECT status, closed_at_ns FROM contract_disputes WHERE stripe_dispute_id = 'du_c3'",
        )
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(row.0, "won");
        assert!(row.1.is_some(), "closed_at_ns must be set on close");
    }

    #[tokio::test]
    async fn test_dispute_closed_lost_terminates_and_records_refund() {
        // Pause + close=lost MUST: terminate (cancelled, payment_status=disputed),
        // emit dispute_lost event with the dispute id, and record a positive
        // refund_amount_e9s on the contract row using the deterministic
        // idempotency key `dispute:<id>`. We cannot live-call Stripe in a
        // unit test, so we pass `stripe_client=None` via process_dispute_lost_refund;
        // the handler swallows that and we assert the DB-side accounting
        // (refund_amount_e9s) plus the idempotency-key construction
        // (which is what Stripe-side replay collapsing relies on).
        let db = setup_test_db().await;
        let contract_id = vec![0xC4; 32];
        insert_active_contract(&db, &contract_id, Some("pi_test_c4")).await;

        // Pause first so total_paused_ns is non-zero by the time we refund.
        let created = dispute_event(
            "charge.dispute.created",
            "du_c4",
            "ch_c4",
            Some("pi_test_c4"),
            "needs_response",
            None,
        );
        handle_dispute_created(&db, &unwrap_object(&created))
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // The handler attempts to construct StripeClient::new(); without
        // STRIPE_SECRET_KEY it returns None and the refund is "calculated but
        // not pushed", which is still observable on the contract row.
        let was_set = std::env::var("STRIPE_SECRET_KEY").ok();
        std::env::remove_var("STRIPE_SECRET_KEY");
        let closed = dispute_event(
            "charge.dispute.closed",
            "du_c4",
            "ch_c4",
            Some("pi_test_c4"),
            "lost",
            None,
        );
        handle_dispute_closed(&db, &unwrap_object(&closed))
            .await
            .unwrap();
        if let Some(v) = was_set {
            std::env::set_var("STRIPE_SECRET_KEY", v);
        }

        let row: (String, String, Option<i64>) = sqlx::query_as(
            "SELECT status, payment_status, refund_amount_e9s FROM contract_sign_requests WHERE contract_id = $1",
        )
        .bind(&contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(row.0, "cancelled");
        assert_eq!(row.1, "disputed");
        assert!(
            row.2.unwrap_or(0) > 0,
            "refund_amount_e9s must be positive on lost-dispute path"
        );

        let dispute_lost_events: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM contract_events WHERE contract_id = $1 AND event_type = 'dispute_lost'",
        )
        .bind(&contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(dispute_lost_events, 1);

        // Idempotency-key construction is the contract Stripe relies on; assert
        // the exact value here so a refactor that breaks it surfaces loudly.
        assert_eq!(dispute_refund_idempotency_key("du_c4"), "dispute:du_c4");
    }

    #[tokio::test]
    async fn test_orphan_dispute_persists_and_does_not_5xx() {
        // dispute.created for a charge we never issued (no metadata, no PI
        // we recognise) MUST NOT 5xx -- Stripe would retry forever. Instead:
        // upsert the dispute row with NULL contract_id, log + page ops.
        let db = setup_test_db().await;
        let event = dispute_event(
            "charge.dispute.created",
            "du_orphan",
            "ch_orphan",
            Some("pi_unknown"),
            "needs_response",
            None,
        );
        handle_dispute_created(&db, &unwrap_object(&event))
            .await
            .expect("orphan dispute MUST return Ok (Stripe retries on 5xx)");

        assert_eq!(count_disputes(&db, "du_orphan").await, 1);
        let contract_id: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT contract_id FROM contract_disputes WHERE stripe_dispute_id = 'du_orphan'",
        )
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert!(
            contract_id.is_none(),
            "orphan dispute row MUST have NULL contract_id"
        );
    }

    #[tokio::test]
    async fn test_orphan_dispute_relinks_on_late_checkout_completion() {
        // Out-of-order delivery (#426): Stripe delivers `charge.dispute.created`
        // BEFORE `checkout.session.completed` for the same payment. At dispute
        // time the contract has no `stripe_payment_intent_id` yet, so the
        // dispute is persisted as an orphan (NULL contract_id). When checkout
        // completion later sets the PI on the contract, the reconciliation MUST
        // backfill the FK so the dispute is visible on the contract.
        //
        // Money-path invariants asserted below:
        //  * relink is idempotent (second call affects 0 rows);
        //  * relink touches ONLY contract_id -- it does not mutate the
        //    contract's status, payment_status, or refund_amount_e9s, so it
        //    cannot trigger a double-refund / double-pause / spurious state flip.
        let db = setup_test_db().await;
        let contract_id = vec![0xC6; 32];

        // Contract exists with NO stripe_payment_intent_id yet (checkout not
        // yet delivered). insert_active_contract takes Option<pi>; pass None.
        insert_active_contract(&db, &contract_id, None).await;

        // 1. Dispute arrives first. Stripe knows the PI; the contract does not.
        //    No metadata.contract_id -> lookup falls through -> orphan.
        let dispute = dispute_event(
            "charge.dispute.created",
            "du_oop",
            "ch_oop",
            Some("pi_oop"),
            "needs_response",
            None,
        );
        handle_dispute_created(&db, &unwrap_object(&dispute))
            .await
            .expect("orphan dispute MUST return Ok (Stripe retries on 5xx)");

        let orphan_cid: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT contract_id FROM contract_disputes WHERE stripe_dispute_id = 'du_oop'",
        )
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert!(
            orphan_cid.is_none(),
            "dispute MUST be orphaned before checkout completion"
        );

        // 2. checkout.session.completed arrives: learn the PI on the contract.
        db.update_checkout_session_payment(
            &contract_id,
            "cs_oop",
            Some("pi_oop"),
            None,
            None,
            false,
            None,
        )
        .await
        .expect("update_checkout_session_payment must succeed");

        // Capture money-path columns AFTER checkout, BEFORE relink.
        let before: (String, String, Option<i64>) = sqlx::query_as(
            "SELECT status, payment_status, refund_amount_e9s \
             FROM contract_sign_requests WHERE contract_id = $1",
        )
        .bind(&contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();

        // 3. Reconciliation runs (as the webhook handler does post-payment).
        let linked = db
            .relink_orphan_disputes_for_payment_intent(&contract_id, "pi_oop")
            .await
            .expect("relink must not error");
        assert_eq!(linked, 1, "exactly one orphan dispute should have been linked");

        // The orphan row now carries the contract_id FK.
        let relinked_cid: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT contract_id FROM contract_disputes WHERE stripe_dispute_id = 'du_oop'",
        )
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(
            relinked_cid.as_deref(),
            Some(contract_id.as_slice()),
            "orphan dispute MUST be linked to the contract after checkout completion"
        );

        // 4. Idempotency: replaying the reconciliation affects 0 rows.
        let linked_again = db
            .relink_orphan_disputes_for_payment_intent(&contract_id, "pi_oop")
            .await
            .expect("idempotent relink must not error");
        assert_eq!(
            linked_again, 0,
            "relink MUST be idempotent -- second call affects zero rows"
        );

        // 5. Money-safety: relink touched ONLY contract_id on the dispute row.
        //    Contract status / payment_status / refund_amount_e9s are unchanged.
        let after: (String, String, Option<i64>) = sqlx::query_as(
            "SELECT status, payment_status, refund_amount_e9s \
             FROM contract_sign_requests WHERE contract_id = $1",
        )
        .bind(&contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(
            before, after,
            "relink MUST NOT mutate contract status / payment_status / refund_amount_e9s"
        );
        assert_eq!(
            after.1, "succeeded",
            "payment_status is set by update_checkout_session_payment, not by relink"
        );
    }

    #[tokio::test]
    async fn test_orphan_dispute_pause_replayed_on_late_checkout_completion() {
        // #447: when an orphan dispute (open) is re-linked after late checkout
        // completion, the `pause_contract` effect that the normal
        // `charge.dispute.created` handler applies was MISSED while the dispute
        // was orphaned. The replay MUST apply that pause idempotently and MUST
        // NOT touch any money column (pause is a status change only; the
        // refund path is a separate money concern that is deferred here).
        let db = setup_test_db().await;
        let contract_id = vec![0xD1; 32];

        // Contract exists with NO stripe_payment_intent_id yet (checkout not
        // yet delivered).
        insert_active_contract(&db, &contract_id, None).await;

        // 1. Dispute arrives first as an orphan: contract has no PI, so all
        //    lookups fail and the row persists with NULL contract_id. No pause
        //    is attempted (there is no contract to pause).
        let dispute = dispute_event(
            "charge.dispute.created",
            "du_447_open",
            "ch_447_open",
            Some("pi_447_open"),
            "needs_response",
            None,
        );
        handle_dispute_created(&db, &unwrap_object(&dispute))
            .await
            .expect("orphan dispute MUST return Ok");

        // 2. checkout.session.completed learns the PI on the contract.
        db.update_checkout_session_payment(
            &contract_id,
            "cs_447_open",
            Some("pi_447_open"),
            None,
            None,
            false,
            None,
        )
        .await
        .expect("update_checkout_session_payment must succeed");

        // 3. Relink (the #426 reconciliation) backfills the FK.
        let linked = db
            .relink_orphan_disputes_for_payment_intent(&contract_id, "pi_447_open")
            .await
            .expect("relink must not error");
        assert_eq!(linked, 1);

        // GAP PROOF: after relink, the contract is STILL active (the pause
        // effect was missed while the dispute was orphaned).
        assert_eq!(
            read_status(&db, &contract_id).await,
            "active",
            "gap: contract MUST still be active immediately after relink (pause not yet replayed)"
        );

        // 4. The #447 replay applies the missed pause.
        let outcome = db
            .replay_orphan_dispute_lifecycle(&contract_id, "pi_447_open", None)
            .await
            .expect("replay must not error");
        assert_eq!(outcome.paused, 1, "exactly one open dispute must be paused");
        assert_eq!(
            outcome.terminated, 0,
            "no closed-lost dispute in this scenario"
        );

        // The pause effect is now applied, mirroring the normal handler.
        assert_eq!(read_status(&db, &contract_id).await, "paused");
        let pause_reason: String = sqlx::query_scalar(
            "SELECT pause_reason FROM contract_sign_requests WHERE contract_id = $1",
        )
        .bind(&contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(pause_reason, "stripe_dispute:du_447_open");
        assert_eq!(
            count_history_to(&db, &contract_id, "paused").await,
            1,
            "exactly one paused history row"
        );

        // 5. IDEMPOTENCY: replaying the pause is a no-op (pause_contract with
        //    the same reason returns Ok without a second transition).
        let outcome_again = db
            .replay_orphan_dispute_lifecycle(&contract_id, "pi_447_open", None)
            .await
            .expect("idempotent replay must not error");
        assert_eq!(outcome_again.paused, 1, "replay re-confirms the pause");
        assert_eq!(
            count_history_to(&db, &contract_id, "paused").await,
            1,
            "idempotent replay MUST NOT emit a second paused history row"
        );

        // 6. MONEY-SAFETY: the replay touched NO money column. Refund columns
        //    stay NULL/zero and payment_status stays 'succeeded'. The deferred
        //    refund path (closed-lost) is a separate money concern.
        let money: (Option<i64>, Option<String>, Option<i64>, String) = sqlx::query_as(
            "SELECT refund_amount_e9s, stripe_refund_id, refund_created_at_ns, payment_status \
             FROM contract_sign_requests WHERE contract_id = $1",
        )
        .bind(&contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(money.0, None, "refund_amount_e9s MUST stay NULL (no refund)");
        assert_eq!(money.1, None, "stripe_refund_id MUST stay NULL (no refund)");
        assert_eq!(
            money.2, None,
            "refund_created_at_ns MUST stay NULL (no refund)"
        );
        assert_eq!(
            money.3, "succeeded",
            "payment_status MUST be unchanged by the pause replay"
        );
    }

    #[tokio::test]
    async fn test_orphan_dispute_closed_lost_is_terminated_and_refunded() {
        // #447 full replay: if a dispute CLOSED as `lost` while it was
        // orphaned, the replay MUST apply the same terminate+refund sequence
        // as `handle_dispute_closed`. Money-safety is guaranteed by:
        //  * `terminate_contract_for_dispute_lost` short-circuits on terminal state
        //  * `process_dispute_lost_refund` uses `dispute:<id>` idempotency key
        //  * If the normal handler already processed this, the dispute would NOT
        //    be orphaned — so the replay is the FIRST and ONLY processing.
        let db = setup_test_db().await;
        let contract_id = vec![0xD2; 32];
        insert_active_contract(&db, &contract_id, None).await;

        // Orphan dispute that has already CLOSED LOST while orphaned.
        let now_ns = crate::now_ns().unwrap();
        sqlx::query(
            "INSERT INTO contract_disputes \
             (contract_id, stripe_dispute_id, stripe_charge_id, stripe_payment_intent_id, \
              reason, status, amount_cents, currency, raw_event, created_at_ns, updated_at_ns, closed_at_ns) \
             VALUES (NULL, 'du_447_lost', 'ch_447_lost', 'pi_447_lost', \
                     'fraudulent', 'lost', 5000, 'usd', '{}'::jsonb, $1, $1, $1)",
        )
        .bind(now_ns)
        .execute(&db.pool)
        .await
        .unwrap();

        db.update_checkout_session_payment(
            &contract_id,
            "cs_447_lost",
            Some("pi_447_lost"),
            None,
            None,
            false,
            None,
        )
        .await
        .unwrap();
        db.relink_orphan_disputes_for_payment_intent(&contract_id, "pi_447_lost")
            .await
            .unwrap();

        let outcome = db
            .replay_orphan_dispute_lifecycle(&contract_id, "pi_447_lost", None)
            .await
            .expect("replay must not error on closed-lost orphan");

        // The closed-lost orphan is terminated (not paused).
        assert_eq!(
            outcome.terminated, 1,
            "closed-lost orphan MUST be terminated"
        );
        assert_eq!(
            outcome.paused, 0,
            "a lost dispute MUST NOT be paused (terminate is the correct action)"
        );

        // Contract is now cancelled (terminated).
        assert_eq!(
            read_status(&db, &contract_id).await,
            "cancelled",
            "closed-lost orphan replay MUST terminate the contract"
        );

        // Refund: terminate set payment_status='disputed', so
        // process_dispute_lost_refund computes a prorated amount and writes
        // refund_amount_e9s to the DB. No Stripe client in the test, so no
        // actual Stripe Refund is issued (stripe_refund_id is NULL) — but the
        // accounting is correct, which is what matters.
        assert_eq!(
            outcome.refunded, 1,
            "prorated refund computed and recorded for closed-lost orphan"
        );
        let payment_status: String = sqlx::query_scalar(
            "SELECT payment_status FROM contract_sign_requests WHERE contract_id = $1",
        )
        .bind(&contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(
            payment_status, "disputed",
            "terminate sets payment_status='disputed'"
        );
    }

    #[tokio::test]
    async fn test_orphan_dispute_replay_graceful_when_contract_not_pausable() {
        // #447 realistic case: at checkout.session.completed the contract is
        // still `requested` (provider has not accepted yet). A `requested`
        // contract cannot transition to `paused` (state machine only allows
        // Active/Provisioned -> Paused), so the replayed pause MUST fail
        // gracefully -- no crash, no money movement, dispute stays linked and
        // visible. This mirrors exactly what the normal `handle_dispute_created`
        // does when a dispute arrives on a pre-active contract: it logs + pages
        // ops but does not 500. The deeper "pause-on-activation" gap is a
        // separate concern (needs state-machine + dc-agent coordination).
        let db = setup_test_db().await;
        let contract_id = vec![0xD3; 32];

        // Insert a `requested` contract (NOT active) with NO PI yet.
        // Runtime query (not `query!`) to avoid churning `.sqlx` for a
        // test-only insert with different column literals than the helpers.
        let now_ns = crate::now_ns().unwrap();
        sqlx::query(
            "INSERT INTO contract_sign_requests \
             (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, \
              provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, \
              status, payment_method, stripe_payment_intent_id, payment_status, currency, \
              provisioning_completed_at_ns, end_timestamp_ns) \
             VALUES ($1, $2, 'ssh-key', 'contact', $3, 'off-447', 100000000000, 'poc', 0, \
                     'requested', 'stripe', NULL, 'pending', 'usd', $4, $5)",
        )
        .bind(&contract_id[..])
        .bind(&[1u8; 32][..])
        .bind(&[2u8; 32][..])
        .bind(now_ns - 60_000_000_000)
        .bind(now_ns + 86_400_000_000_000i64)
        .execute(&db.pool)
        .await
        .unwrap();

        // Orphan open dispute.
        let dispute = dispute_event(
            "charge.dispute.created",
            "du_447_req",
            "ch_447_req",
            Some("pi_447_req"),
            "needs_response",
            None,
        );
        handle_dispute_created(&db, &unwrap_object(&dispute))
            .await
            .unwrap();
        db.update_checkout_session_payment(
            &contract_id,
            "cs_447_req",
            Some("pi_447_req"),
            None,
            None,
            false,
            None,
        )
        .await
        .unwrap();
        db.relink_orphan_disputes_for_payment_intent(&contract_id, "pi_447_req")
            .await
            .unwrap();

        // Replay MUST NOT error even though the contract is not pausable.
        let outcome = db
            .replay_orphan_dispute_lifecycle(&contract_id, "pi_447_req", None)
            .await
            .expect("replay MUST NOT error when contract is not pausable");
        assert_eq!(
            outcome.paused, 0,
            "pause MUST NOT be counted as applied when the contract cannot transition to paused"
        );
        assert_eq!(outcome.terminated, 0);

        // Contract stays `requested`; pause_reason stays NULL (nothing to pause).
        assert_eq!(read_status(&db, &contract_id).await, "requested");
        let pause_reason: Option<String> = sqlx::query_scalar(
            "SELECT pause_reason FROM contract_sign_requests WHERE contract_id = $1",
        )
        .bind(&contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert!(pause_reason.is_none(), "no pause_reason written");

        // Dispute IS linked (the relink worked) -- the row stays visible.
        let linked: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT contract_id FROM contract_disputes WHERE stripe_dispute_id = 'du_447_req'",
        )
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(
            linked.as_deref(),
            Some(contract_id.as_slice()),
            "dispute MUST remain linked even when pause could not be applied"
        );

        // MONEY-SAFETY: no refund columns mutated.
        let refund: Option<i64> = sqlx::query_scalar(
            "SELECT refund_amount_e9s FROM contract_sign_requests WHERE contract_id = $1",
        )
        .bind(&contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert!(refund.is_none(), "no refund column touched");

        // IDEMPOTENT: re-running the replay is still a no-op.
        let outcome_again = db
            .replay_orphan_dispute_lifecycle(&contract_id, "pi_447_req", None)
            .await
            .unwrap();
        assert_eq!(outcome_again.paused, 0);
        assert_eq!(read_status(&db, &contract_id).await, "requested");
    }

    #[tokio::test]
    async fn test_dispute_funds_withdrawn_sets_timestamp_no_state_change() {
        // funds_withdrawn is informational: persist the row with
        // funds_withdrawn_at_ns and DO NOT touch the contract status. Active
        // contracts stay active; paused contracts stay paused.
        let db = setup_test_db().await;
        let contract_id = vec![0xC5; 32];
        insert_active_contract(&db, &contract_id, Some("pi_test_c5")).await;

        let event = dispute_event(
            "charge.dispute.funds_withdrawn",
            "du_c5",
            "ch_c5",
            Some("pi_test_c5"),
            "needs_response",
            None,
        );
        handle_dispute_funds_withdrawn(&db, &unwrap_object(&event))
            .await
            .unwrap();

        let funds_withdrawn_at: Option<i64> = sqlx::query_scalar(
            "SELECT funds_withdrawn_at_ns FROM contract_disputes WHERE stripe_dispute_id = 'du_c5'",
        )
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert!(
            funds_withdrawn_at.is_some(),
            "funds_withdrawn handler MUST set funds_withdrawn_at_ns"
        );
        assert_eq!(
            read_status(&db, &contract_id).await,
            "active",
            "funds_withdrawn MUST NOT mutate contract status"
        );
    }
}
