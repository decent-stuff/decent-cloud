use crate::chatwoot::ChatwootClient;
use crate::database::Database;
use crate::notifications::telegram::{TelegramClient, TelegramUpdate};
use crate::openapi::common::decode_hex_path;
use crate::support_bot::handler::handle_customer_message;
use anyhow::{Context, Result};
use email_utils::EmailService;
use poem::{handler, http::header::HeaderMap, web::Data, Body, Error as PoemError, Response};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct StripeEvent {
    #[serde(rename = "type")]
    event_type: String,
    data: StripeEventData,
}

#[derive(Debug, Deserialize)]
struct StripeEventData {
    object: serde_json::Value, // Can be PaymentIntent or CheckoutSession
}

#[derive(Debug, Deserialize)]
struct StripeCheckoutSession {
    id: String,
    invoice: Option<String>,
    /// Real PaymentIntent ID (`pi_*`) attached by Stripe at session completion.
    /// Stripe sends this as the PI string in webhook payloads.
    payment_intent: Option<String>,
    metadata: Option<serde_json::Value>,
    total_details: Option<StripeTotalDetails>,
    customer_details: Option<StripeCustomerDetails>,
}

#[derive(Debug, Deserialize)]
struct StripeTotalDetails {
    amount_tax: Option<i64>, // Tax amount in cents
}

#[derive(Debug, Deserialize)]
struct StripeCustomerDetails {
    tax_ids: Option<Vec<StripeTaxId>>,
}

#[derive(Debug, Deserialize)]
struct StripeTaxId {
    #[serde(rename = "type")]
    tax_type: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct StripeInvoice {
    id: String,
    metadata: Option<serde_json::Value>,
}

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

/// Verify Stripe webhook signature.
///
/// Constant-time HMAC comparison via [`super::signature::verify_hmac_sha256_hex`]
/// (see #428 for the timing-attack rationale).
fn verify_signature(payload: &str, signature: &str, secret: &str) -> Result<()> {
    // Parse signature header (format: "t=timestamp,v1=signature")
    let mut timestamp = None;
    let mut sig_hash = None;
    for part in signature.split(',') {
        let kv: Vec<&str> = part.splitn(2, '=').collect();
        if kv.len() == 2 {
            match kv[0] {
                "t" => timestamp = Some(kv[1]),
                "v1" => sig_hash = Some(kv[1]),
                _ => {}
            }
        }
    }
    let timestamp = timestamp.context("Missing timestamp in signature header")?;
    let sig_hash = sig_hash.context("Missing v1 signature in signature header")?;

    let signed_payload = format!("{}.{}", timestamp, payload);
    super::signature::verify_hmac_sha256_hex(
        signed_payload.as_bytes(),
        secret.as_bytes(),
        sig_hash,
    )
}

/// Handle Stripe webhook events
#[handler]
pub async fn stripe_webhook(
    db: Data<&Arc<Database>>,
    email_service: Data<&Option<Arc<EmailService>>>,
    body: Body,
    req: &poem::Request,
) -> Result<Response, PoemError> {
    // Get raw body for signature verification
    let body_bytes = body.into_vec().await.map_err(|e| {
        PoemError::from_string(
            format!("Failed to read body: {}", e),
            poem::http::StatusCode::BAD_REQUEST,
        )
    })?;

    let payload = String::from_utf8(body_bytes.clone()).map_err(|e| {
        PoemError::from_string(
            format!("Invalid UTF-8 in payload: {}", e),
            poem::http::StatusCode::BAD_REQUEST,
        )
    })?;

    // Get signature from header
    let signature = req
        .headers()
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            PoemError::from_string(
                "Missing stripe-signature header",
                poem::http::StatusCode::BAD_REQUEST,
            )
        })?;

    // Get webhook secret from environment
    let webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET").map_err(|_| {
        PoemError::from_string(
            "STRIPE_WEBHOOK_SECRET not configured",
            poem::http::StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;

    // Verify signature
    verify_signature(&payload, signature, &webhook_secret).map_err(|e| {
        tracing::error!("Webhook signature verification failed: {:#}", e);
        PoemError::from_string("Invalid signature", poem::http::StatusCode::UNAUTHORIZED)
    })?;

    // Parse event
    let event: StripeEvent = serde_json::from_slice(&body_bytes).map_err(|e| {
        PoemError::from_string(
            format!("Invalid JSON: {}", e),
            poem::http::StatusCode::BAD_REQUEST,
        )
    })?;

    tracing::info!("Received Stripe webhook: {}", event.event_type);

    // Handle event types
    match event.event_type.as_str() {
        "checkout.session.completed" => {
            // Parse checkout session from event data
            let session: StripeCheckoutSession = serde_json::from_value(event.data.object)
                .map_err(|e| {
                    tracing::error!("Failed to parse checkout session: {:#}", e);
                    PoemError::from_string(
                        format!("Invalid session data: {}", e),
                        poem::http::StatusCode::BAD_REQUEST,
                    )
                })?;

            tracing::info!("Checkout session completed: {}", session.id);

            // Extract contract_id from metadata
            let contract_id_hex = session
                .metadata
                .as_ref()
                .and_then(|m| m.get("contract_id"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    tracing::error!("Missing contract_id in session metadata");
                    PoemError::from_string(
                        "Missing contract_id in metadata",
                        poem::http::StatusCode::BAD_REQUEST,
                    )
                })?;

            let contract_id_bytes = decode_hex_path(contract_id_hex, "contract_id")
                .map_err(|e| {
                    tracing::error!("{e}");
                    PoemError::from_string(e, poem::http::StatusCode::BAD_REQUEST)
                })?;

            // Extract tax information
            let tax_amount_cents = session.total_details.as_ref().and_then(|td| td.amount_tax);

            let tax_amount_e9s = tax_amount_cents.map(|cents| cents * 10_000_000);

            let customer_tax_id = session
                .customer_details
                .as_ref()
                .and_then(|cd| cd.tax_ids.as_ref())
                .and_then(|ids| ids.first())
                .map(|tax_id| format!("{}: {}", tax_id.tax_type, tax_id.value));

            // Detect reverse charge: 0% VAT with valid EU VAT ID
            // Stripe Tax automatically applies reverse charge for B2B cross-border EU
            let reverse_charge = customer_tax_id.is_some() && tax_amount_cents.unwrap_or(1) == 0;

            // Update contract with tax info and set payment status to succeeded.
            // `session.payment_intent` is the real PaymentIntent ID (`pi_*`) that we
            // need for downstream refund and dispute lookups.
            if let Err(e) = db
                .update_checkout_session_payment(
                    &contract_id_bytes,
                    &session.id,
                    session.payment_intent.as_deref(),
                    tax_amount_e9s,
                    customer_tax_id.as_deref(),
                    reverse_charge,
                    session.invoice.as_deref(),
                )
                .await
            {
                tracing::error!(
                    "Failed to update checkout session payment for contract {}: {}",
                    contract_id_hex,
                    e
                );
                return Err(PoemError::from_string(
                    format!("Database error: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }

            // Contract stays in 'requested' status - provider must explicitly accept/reject
            // If rejected, user gets full refund via reject_contract()
            tracing::info!(
                "Contract {} payment succeeded, awaiting provider review",
                contract_id_hex
            );

            // Out-of-order delivery reconciliation (#426): if Stripe delivered
            // `charge.dispute.created` BEFORE this checkout completion, the
            // dispute was persisted as an orphan (NULL contract_id) because the
            // contract's PI was not yet known. Now that we know the PI, backfill
            // the FK so the dispute is visible on the contract. Best-effort +
            // idempotent: failure here MUST NOT fail the webhook (payment already
            // succeeded) and replays affect each orphan at most once. This only
            // links the row -- the lifecycle replay is #447, below.
            if let Some(pi) = session.payment_intent.as_deref() {
                let linked = db.relink_orphan_disputes_for_payment_intent(&contract_id_bytes, pi).await;
                match linked {
                    Ok(n) if n > 0 => {
                        tracing::info!(
                            contract_id = %contract_id_hex,
                            payment_intent = %pi,
                            linked = n,
                            "Reconciled orphan dispute(s) to contract after late checkout completion"
                        );
                        // #447: replay the dispute-lifecycle pause that the
                        // normal `charge.dispute.created` handler applies but
                        // was missed while the dispute was orphaned. Money-safe
                        // (pause is a status change only; refund columns are NOT
                        // touched) and idempotent (pause_contract with the same
                        // reason is a no-op). Closed-lost orphans are detected
                        // and ops is paged -- their terminate+refund replay is a
                        // money-path decision, intentionally deferred. Best-effort:
                        // a replay failure MUST NOT fail the webhook (payment
                        // already succeeded), mirroring the relink above.
                        match db.replay_orphan_dispute_pause(&contract_id_bytes, pi).await {
                            Ok(outcome) => {
                                if outcome.paused > 0 {
                                    tracing::info!(
                                        contract_id = %contract_id_hex,
                                        payment_intent = %pi,
                                        paused = outcome.paused,
                                        "Replayed missed dispute pause for re-linked orphan(s)"
                                    );
                                }
                                if outcome.closed_lost_detected > 0 {
                                    tracing::warn!(
                                        contract_id = %contract_id_hex,
                                        payment_intent = %pi,
                                        closed_lost_detected = outcome.closed_lost_detected,
                                        "Re-linked orphan dispute(s) closed LOST while orphaned; \
                                         refund replay deferred (money-path) -- operator review required"
                                    );
                                    crate::notifications::telegram::send_ops_alert(&format!(
                                        "Stripe dispute re-linked after late checkout for contract {} \
                                         had {} dispute(s) CLOSED LOST while orphaned. \
                                         Terminate+refund replay is deferred (money-path). \
                                         REVIEW + issue the prorated refund manually.",
                                        contract_id_hex, outcome.closed_lost_detected
                                    ))
                                    .await;
                                }
                            }
                            Err(e) => tracing::warn!(
                                contract_id = %contract_id_hex,
                                payment_intent = %pi,
                                error = %format!("{:#}", e),
                                "Failed to replay orphan dispute pause; payment still succeeded"
                            ),
                        }
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!(
                        contract_id = %contract_id_hex,
                        payment_intent = %pi,
                        error = %format!("{:#}", e),
                        "Failed to reconcile orphan disputes for contract; payment still succeeded"
                    ),
                }
            }

            // Notify provider about new rental request
            match db.get_contract(&contract_id_bytes).await {
                Ok(Some(contract)) => {
                    if let Err(e) = crate::rental_notifications::notify_provider_new_rental(
                        db.as_ref(),
                        email_service.as_ref(),
                        &contract,
                    )
                    .await
                    {
                        tracing::warn!(
                            "Failed to notify provider for contract {}: {}",
                            contract_id_hex,
                            e
                        );
                    }
                }
                Ok(None) => {
                    tracing::warn!(
                        "Contract {} not found after payment succeeded",
                        contract_id_hex
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to fetch contract {} for provider notification after payment: {:#}",
                        contract_id_hex,
                        e
                    );
                }
            }

            // Schedule delayed receipt sending - wait for Stripe invoice to be ready
            // Background processor will retry 5 times at 1-minute intervals before falling back to Typst
            if let Err(e) = db.schedule_pending_stripe_receipt(&contract_id_bytes).await {
                tracing::error!(
                    "Failed to schedule pending receipt for contract {}: {}",
                    contract_id_hex,
                    e
                );
                // Don't fail the webhook - payment was successful
            } else {
                tracing::info!(
                    "Scheduled pending receipt for contract {} (will wait for Stripe invoice)",
                    contract_id_hex
                );
            }
        }
        // invoice.paid is fired when the invoice is finalized and paid
        // This happens asynchronously after checkout.session.completed when invoice_creation is enabled
        "invoice.paid" => {
            let invoice: StripeInvoice =
                serde_json::from_value(event.data.object).map_err(|e| {
                    tracing::error!("Failed to parse invoice: {:#}", e);
                    PoemError::from_string(
                        format!("Invalid invoice data: {}", e),
                        poem::http::StatusCode::BAD_REQUEST,
                    )
                })?;

            tracing::info!("Invoice paid: {}", invoice.id);

            // Extract contract_id from invoice metadata (passed via invoice_data.metadata)
            let contract_id_hex = invoice
                .metadata
                .as_ref()
                .and_then(|m| m.get("contract_id"))
                .and_then(|v| v.as_str());

            if let Some(contract_id_hex) = contract_id_hex {
                match decode_hex_path(contract_id_hex, "contract_id") {
                    Ok(contract_id_bytes) => {
                        // Update contract with the invoice ID
                        if let Err(e) = db
                            .update_stripe_invoice_id(&contract_id_bytes, &invoice.id)
                            .await
                        {
                            tracing::error!(
                                "Failed to update stripe_invoice_id for contract {}: {}",
                                contract_id_hex,
                                e
                            );
                            // Don't fail webhook - invoice was created successfully
                        } else {
                            tracing::info!(
                                "Updated contract {} with invoice ID {}",
                                contract_id_hex,
                                invoice.id
                            );
                        }

                        // Cancel any pending receipt - we'll send immediately with Stripe invoice
                        if let Err(e) = db.remove_pending_stripe_receipt(&contract_id_bytes).await {
                            tracing::warn!(
                                "Failed to remove pending receipt for contract {}: {}",
                                contract_id_hex,
                                e
                            );
                            // Don't fail webhook - invoice was created successfully
                        }

                        // Send receipt with Stripe invoice PDF attached
                        // This is idempotent - skips if receipt already sent
                        match crate::receipts::send_payment_receipt(
                            db.as_ref(),
                            &contract_id_bytes,
                            email_service.as_ref(),
                        )
                        .await
                        {
                            Ok(0) => {
                                tracing::debug!(
                                    "Receipt already sent for contract {}, skipping",
                                    contract_id_hex
                                );
                            }
                            Ok(receipt_num) => {
                                tracing::info!(
                                    "Sent receipt #{} with Stripe invoice for contract {} via invoice.paid",
                                    receipt_num,
                                    contract_id_hex
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to send receipt for contract {}: {}",
                                    contract_id_hex,
                                    e
                                );
                                // Don't fail the webhook - invoice was created successfully
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("invoice.paid: invalid contract_id in metadata: {e}");
                    }
                }
            } else {
                // This is fine - could be an invoice from a subscription or other source
                tracing::debug!(
                    "Invoice {} has no contract_id in metadata, skipping",
                    invoice.id
                );
            }
        }
        // Invoice payment failure: log for ops visibility. Per-contract
        // recurring-billing state changes are driven by the checkout/invoice
        // arms above and Stripe's own dunning; no account-level action here.
        "invoice.payment_failed" => {
            let invoice: StripeInvoice = serde_json::from_value(event.data.object.clone())
                .map_err(|e| {
                    tracing::error!("Failed to parse invoice: {:#}", e);
                    PoemError::from_string(
                        format!("Invalid invoice data: {}", e),
                        poem::http::StatusCode::BAD_REQUEST,
                    )
                })?;

            tracing::warn!("Invoice payment failed: {}", invoice.id);
        }

        // Stripe dispute lifecycle (Phase 2). Each handler wraps a Phase 1 DB
        // primitive in `contracts/dispute.rs`. All four handlers MUST be 2xx
        // even if the dispute can't be matched to a contract -- a 5xx puts
        // Stripe into an indefinite retry loop while the operator paging is
        // already happening via `send_ops_alert` in the orphan path.
        "charge.dispute.created" => {
            handle_dispute_created(db.as_ref(), &event.data.object).await?;
        }
        "charge.dispute.updated" => {
            handle_dispute_updated(db.as_ref(), &event.data.object).await?;
        }
        "charge.dispute.closed" => {
            handle_dispute_closed(db.as_ref(), &event.data.object).await?;
        }
        "charge.dispute.funds_withdrawn" => {
            handle_dispute_funds_withdrawn(db.as_ref(), &event.data.object).await?;
        }
        // Note: payment_intent.succeeded and payment_intent.payment_failed webhooks are NOT used.
        // We use checkout.session.completed which already sets payment_status and has the contract_id.
        // Stripe Checkout generates its own PaymentIntent internally, but we link contracts by
        // checkout session ID, not payment intent ID.
        _ => {
            tracing::debug!("Unhandled event type: {}", event.event_type);
        }
    }

    Ok(Response::builder()
        .status(poem::http::StatusCode::OK)
        .body(""))
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

async fn handle_dispute_created(
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

async fn handle_dispute_updated(
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

async fn handle_dispute_closed(
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

async fn handle_dispute_funds_withdrawn(
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

// Chatwoot webhook types
// For message_created: message data is at top level with nested conversation
// For other events: conversation is at top level
#[derive(Debug, Deserialize)]
struct ChatwootWebhookPayload {
    event: String,
    // For conversation events (conversation_status_changed, etc.)
    #[serde(default)]
    conversation: Option<ChatwootConversation>,
    // For message events - message fields are at top level
    id: Option<i64>,
    message_type: Option<serde_json::Value>, // Can be int or string
    created_at: Option<serde_json::Value>,   // Timestamp
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatwootConversation {
    id: i64,
    #[allow(dead_code)] // Part of API response, kept for future use
    status: Option<String>,
    custom_attributes: Option<serde_json::Value>,
}

/// Handle Chatwoot webhook events for response time tracking and AI bot
#[handler]
pub async fn chatwoot_webhook(
    db: Data<&Arc<Database>>,
    email_service: Data<&Option<Arc<email_utils::EmailService>>>,
    body: Body,
) -> Result<Response, PoemError> {
    let body_bytes = body.into_vec().await.map_err(|e| {
        PoemError::from_string(
            format!("Failed to read body: {}", e),
            poem::http::StatusCode::BAD_REQUEST,
        )
    })?;

    // Log raw payload for debugging
    if let Ok(raw) = String::from_utf8(body_bytes.clone()) {
        tracing::debug!("Chatwoot webhook raw payload: {}", raw);
    }

    let payload: ChatwootWebhookPayload = serde_json::from_slice(&body_bytes).map_err(|e| {
        tracing::error!("Failed to parse Chatwoot webhook: {:#}", e);
        PoemError::from_string(
            format!("Invalid JSON: {}", e),
            poem::http::StatusCode::BAD_REQUEST,
        )
    })?;

    tracing::info!(
        "Received Chatwoot webhook: {} (conversation: {:?}, message_id: {:?})",
        payload.event,
        payload.conversation.as_ref().map(|c| c.id),
        payload.id
    );

    // Notifications are sent directly by the bot handler on escalation.
    // No need to handle conversation_status_changed here.

    if payload.event == "message_created" {
        // For message_created, message fields are at top level with conversation nested
        let Some(conv) = payload.conversation else {
            tracing::warn!("message_created webhook missing conversation data");
            return Ok(Response::builder()
                .status(poem::http::StatusCode::OK)
                .body(""));
        };

        let Some(message_id) = payload.id else {
            tracing::warn!("message_created webhook missing message id");
            return Ok(Response::builder()
                .status(poem::http::StatusCode::OK)
                .body(""));
        };

        // message_type can be int (0=incoming, 1=outgoing) or string
        let sender_type = match &payload.message_type {
            Some(v) if v.as_i64() == Some(0) || v.as_str() == Some("incoming") => "customer",
            Some(v) if v.as_i64() == Some(1) || v.as_str() == Some("outgoing") => "provider",
            other => {
                tracing::debug!(
                    "Ignoring Chatwoot message {} with type {:?} (not incoming/outgoing)",
                    message_id,
                    other
                );
                return Ok(Response::builder()
                    .status(poem::http::StatusCode::OK)
                    .body(""));
            }
        };

        tracing::info!(
            "Processing Chatwoot message {} from {}",
            message_id,
            sender_type
        );

        // Extract contract_id for response time tracking only (optional)
        let contract_id = conv
            .custom_attributes
            .as_ref()
            .and_then(|attrs| attrs.get("contract_id"))
            .and_then(|v| v.as_str());

        // Track message for response time (only if contract_id is present)
        if let Some(cid) = contract_id {
            // Extract created_at timestamp
            let created_at = payload
                .created_at
                .as_ref()
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            if let Err(e) = db
                .insert_chatwoot_message_event(cid, conv.id, message_id, sender_type, created_at)
                .await
            {
                tracing::warn!("Failed to insert Chatwoot message event: {:#}", e);
                // Don't fail webhook - event may be duplicate
            }
        }

        // If this is an incoming customer message, trigger bot response
        if sender_type == "customer" {
            let Some(content) = payload.content.as_ref() else {
                tracing::debug!(
                    "Chatwoot message {} has no content, skipping bot",
                    message_id
                );
                return Ok(Response::builder()
                    .status(poem::http::StatusCode::OK)
                    .body(""));
            };

            if content.trim().is_empty() {
                tracing::debug!(
                    "Chatwoot message {} has empty content, skipping bot",
                    message_id
                );
                return Ok(Response::builder()
                    .status(poem::http::StatusCode::OK)
                    .body(""));
            }

            // Try to create Chatwoot client and handle message
            let chatwoot = match ChatwootClient::from_env() {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(
                        "Chatwoot client not configured, skipping bot response: {}",
                        e
                    );
                    return Ok(Response::builder()
                        .status(poem::http::StatusCode::OK)
                        .body(""));
                }
            };

            tracing::info!(
                "Invoking AI bot for conversation {} (message: '{}...')",
                conv.id,
                content.chars().take(50).collect::<String>()
            );

            if let Err(e) = handle_customer_message(
                &db,
                &chatwoot,
                email_service.as_ref(),
                conv.id as u64,
                content,
            )
            .await
            {
                tracing::error!(
                    "Failed to handle customer message for conversation {}: {}",
                    conv.id,
                    e
                );
                // Don't fail webhook - log error and continue
            }
        }
    }

    Ok(Response::builder()
        .status(poem::http::StatusCode::OK)
        .body(""))
}

/// Handle Telegram webhook updates for provider replies and /start command
#[handler]
pub async fn telegram_webhook(
    db: Data<&Arc<Database>>,
    headers: &HeaderMap,
    body: Body,
) -> Result<Response, PoemError> {
    // Verify Telegram webhook secret if configured
    // When setWebhook is called with secret_token, Telegram sends it in this header
    if let Ok(expected_secret) = std::env::var("TELEGRAM_WEBHOOK_SECRET") {
        let provided_secret = headers
            .get("x-telegram-bot-api-secret-token")
            .and_then(|v| v.to_str().ok());

        match provided_secret {
            Some(secret) if secret == expected_secret => {
                // Secret verified
            }
            Some(_) => {
                tracing::warn!("Telegram webhook: invalid secret token");
                return Err(PoemError::from_string(
                    "Invalid secret token",
                    poem::http::StatusCode::UNAUTHORIZED,
                ));
            }
            None => {
                tracing::warn!("Telegram webhook: missing secret token header");
                return Err(PoemError::from_string(
                    "Missing secret token",
                    poem::http::StatusCode::UNAUTHORIZED,
                ));
            }
        }
    } else {
        tracing::error!(
            "TELEGRAM_WEBHOOK_SECRET not set - rejecting webhook request! \
             Set this env var and use it when calling Telegram's setWebhook API."
        );
        return Err(PoemError::from_string(
            "Webhook secret not configured",
            poem::http::StatusCode::SERVICE_UNAVAILABLE,
        ));
    }

    let body_bytes = body.into_vec().await.map_err(|e| {
        PoemError::from_string(
            format!("Failed to read body: {}", e),
            poem::http::StatusCode::BAD_REQUEST,
        )
    })?;

    let update: TelegramUpdate = serde_json::from_slice(&body_bytes).map_err(|e| {
        PoemError::from_string(
            format!("Invalid JSON: {}", e),
            poem::http::StatusCode::BAD_REQUEST,
        )
    })?;

    tracing::info!("Received Telegram update: {}", update.update_id);

    // Check if this is a message
    if let Some(msg) = update.message {
        let chat_id = msg.chat.id.to_string();

        // Check for /start command - respond with chat_id for notification setup
        if let Some(text) = &msg.text {
            if text.trim() == "/start" || text.starts_with("/start ") {
                tracing::info!("Received /start command from chat_id: {}", chat_id);

                let telegram = TelegramClient::from_env().map_err(|e| {
                    tracing::error!("TELEGRAM_BOT_TOKEN not configured: {:#}", e);
                    PoemError::from_string(
                        "Telegram not configured",
                        poem::http::StatusCode::SERVICE_UNAVAILABLE,
                    )
                })?;

                let response_text = format!(
                    "Welcome! Your Telegram Chat ID is:\n\n`{}`\n\n\
                    Copy this ID and paste it in your notification settings at:\n\
                    Dashboard → Account → Notifications → Telegram Chat ID\n\n\
                    Once configured, you'll receive support escalation alerts here.",
                    chat_id
                );

                telegram
                    .send_message(&chat_id, &response_text)
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to send /start response: {:#}", e);
                        PoemError::from_string(
                            format!("Failed to send response: {}", e),
                            poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                        )
                    })?;

                return Ok(Response::builder()
                    .status(poem::http::StatusCode::OK)
                    .body(""));
            }
        }

        // Check if this is a reply to a notification
        if let Some(reply_to) = msg.reply_to_message {
            // This is a reply - lookup the conversation from DB
            let conversation_id = db
                .lookup_telegram_conversation(reply_to.message_id)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to lookup Telegram conversation: {:#}", e);
                    PoemError::from_string(
                        "Database error",
                        poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                    )
                })?;

            if let Some(conversation_id) = conversation_id {
                // Extract reply text
                if let Some(reply_text) = msg.text {
                    if !reply_text.trim().is_empty() {
                        // Post reply to Chatwoot
                        match ChatwootClient::from_env() {
                            Ok(chatwoot) => {
                                if let Err(e) = chatwoot
                                    .send_message(conversation_id as u64, &reply_text)
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to post Telegram reply to Chatwoot conversation {}: {}",
                                        conversation_id,
                                        e
                                    );
                                    return Err(PoemError::from_string(
                                        format!("Failed to post reply: {}", e),
                                        poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                                    ));
                                }
                                tracing::info!(
                                    "Posted provider reply to Chatwoot conversation {}",
                                    conversation_id
                                );
                            }
                            Err(e) => {
                                tracing::error!("Chatwoot client not configured: {:#}", e);
                                return Err(PoemError::from_string(
                                    "Chatwoot not configured",
                                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                                ));
                            }
                        }
                    }
                }
            } else {
                tracing::warn!(
                    "Received reply to unknown Telegram message {}",
                    reply_to.message_id
                );
            }
        }
    }

    Ok(Response::builder()
        .status(poem::http::StatusCode::OK)
        .body(""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_signature_valid() {
        let payload = r#"{"test":"data"}"#;
        let secret = "whsec_test_secret";

        // Generate valid signature
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        let timestamp = "1234567890";
        let signed_payload = format!("{}.{}", timestamp, payload);
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signed_payload.as_bytes());
        let sig_hash = hex::encode(mac.finalize().into_bytes());

        let signature = format!("t={},v1={}", timestamp, sig_hash);

        let result = verify_signature(payload, &signature, secret);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_signature_invalid() {
        let payload = r#"{"test":"data"}"#;
        let secret = "whsec_test_secret";
        let signature = "t=1234567890,v1=invalid_signature";

        let result = verify_signature(payload, signature, secret);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid signature"));
    }

    /// Regression test for #428: a near-correct signature (correct length,
    /// correct hex prefix, one byte off at the end) must be rejected. The
    /// constant-time comparison is what makes this safe against timing
    /// side channels; the assertion here is that the rejection still
    /// happens on the value level.
    #[test]
    fn test_verify_signature_constant_time_reject_one_byte_off() {
        let payload = r#"{"test":"data"}"#;
        let secret = "whsec_test_secret";

        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        let timestamp = "1234567890";
        let signed_payload = format!("{}.{}", timestamp, payload);
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signed_payload.as_bytes());
        let valid_hex = hex::encode(mac.finalize().into_bytes());

        // Flip the final hex nibble: same length, same prefix, one byte off.
        let mut tampered = valid_hex.clone();
        let last = tampered.pop().unwrap();
        let flipped = if last == '0' { '1' } else { '0' };
        tampered.push(flipped);
        assert_eq!(tampered.len(), valid_hex.len());
        assert_ne!(tampered, valid_hex);
        assert_eq!(&tampered[..tampered.len() - 1], &valid_hex[..valid_hex.len() - 1]);

        let signature = format!("t={},v1={}", timestamp, tampered);
        let err = verify_signature(payload, &signature, secret)
            .expect_err("one-byte-off signature must be rejected");
        assert!(
            err.to_string().contains("Invalid signature"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn test_verify_signature_missing_timestamp() {
        let payload = r#"{"test":"data"}"#;
        let secret = "whsec_test_secret";
        let signature = "v1=somehash";

        let result = verify_signature(payload, signature, secret);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timestamp"));
    }

    #[test]
    fn test_verify_signature_missing_v1() {
        let payload = r#"{"test":"data"}"#;
        let secret = "whsec_test_secret";
        let signature = "t=1234567890";

        let result = verify_signature(payload, signature, secret);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("v1"));
    }

    #[test]
    fn test_telegram_update_deserialization_with_reply() {
        let json = r#"{
            "update_id": 123,
            "message": {
                "message_id": 789,
                "chat": {
                    "id": 456,
                    "type": "private"
                },
                "text": "This is a reply from provider",
                "reply_to_message": {
                    "message_id": 321
                }
            }
        }"#;

        let update: TelegramUpdate = serde_json::from_str(json).unwrap();
        assert_eq!(update.update_id, 123);
        assert!(update.message.is_some());

        let msg = update.message.unwrap();
        assert_eq!(msg.message_id, 789);
        assert_eq!(msg.text, Some("This is a reply from provider".to_string()));
        assert!(msg.reply_to_message.is_some());
        assert_eq!(msg.reply_to_message.unwrap().message_id, 321);
    }

    #[test]
    fn test_telegram_update_deserialization_without_reply() {
        let json = r#"{
            "update_id": 124,
            "message": {
                "message_id": 790,
                "chat": {
                    "id": 456,
                    "type": "private"
                },
                "text": "Just a regular message"
            }
        }"#;

        let update: TelegramUpdate = serde_json::from_str(json).unwrap();
        let msg = update.message.unwrap();
        assert!(msg.reply_to_message.is_none());
    }

    #[test]
    fn test_telegram_update_no_message() {
        let json = r#"{
            "update_id": 125
        }"#;

        let update: TelegramUpdate = serde_json::from_str(json).unwrap();
        assert!(update.message.is_none());
    }

    // Stripe checkout session webhook tests
    #[test]
    fn test_checkout_session_deserialization_with_tax() {
        let json = r#"{
            "id": "cs_test_123",
            "metadata": {
                "contract_id": "abc123def456"
            },
            "total_details": {
                "amount_tax": 250
            },
            "customer_details": {
                "tax_ids": [
                    {
                        "type": "eu_vat",
                        "value": "DE123456789"
                    }
                ]
            }
        }"#;

        let session: StripeCheckoutSession = serde_json::from_str(json).unwrap();
        assert_eq!(session.id, "cs_test_123");
        assert!(session.metadata.is_some());
        assert!(session.total_details.is_some());
        assert_eq!(session.total_details.unwrap().amount_tax, Some(250));
        assert!(session.customer_details.is_some());
        let tax_ids = session.customer_details.unwrap().tax_ids.unwrap();
        assert_eq!(tax_ids.len(), 1);
        assert_eq!(tax_ids[0].tax_type, "eu_vat");
        assert_eq!(tax_ids[0].value, "DE123456789");
    }

    #[test]
    fn test_checkout_session_deserialization_without_tax() {
        let json = r#"{
            "id": "cs_test_456",
            "metadata": {
                "contract_id": "789abc012def"
            },
            "total_details": {
                "amount_tax": null
            },
            "customer_details": {
                "tax_ids": null
            }
        }"#;

        let session: StripeCheckoutSession = serde_json::from_str(json).unwrap();
        assert_eq!(session.id, "cs_test_456");
        assert!(session.metadata.is_some());
        assert!(session.total_details.is_some());
        assert_eq!(session.total_details.unwrap().amount_tax, None);
        assert!(session.customer_details.is_some());
        assert!(session.customer_details.unwrap().tax_ids.is_none());
    }

    #[test]
    fn test_checkout_session_event_deserialization() {
        let json = r#"{
            "type": "checkout.session.completed",
            "data": {
                "object": {
                    "id": "cs_test_789",
                    "metadata": {
                        "contract_id": "abc123"
                    },
                    "total_details": {
                        "amount_tax": 150
                    },
                    "customer_details": {
                        "tax_ids": [
                            {
                                "type": "eu_vat",
                                "value": "FR12345678901"
                            }
                        ]
                    }
                }
            }
        }"#;

        let event: StripeEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "checkout.session.completed");

        let session: StripeCheckoutSession = serde_json::from_value(event.data.object).unwrap();
        assert_eq!(session.id, "cs_test_789");

        let contract_id = session
            .metadata
            .as_ref()
            .and_then(|m| m.get("contract_id"))
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(contract_id, "abc123");

        let tax_amount = session
            .total_details
            .as_ref()
            .and_then(|td| td.amount_tax)
            .unwrap();
        assert_eq!(tax_amount, 150);

        let tax_id = session
            .customer_details
            .as_ref()
            .and_then(|cd| cd.tax_ids.as_ref())
            .and_then(|ids| ids.first())
            .unwrap();
        assert_eq!(tax_id.tax_type, "eu_vat");
        assert_eq!(tax_id.value, "FR12345678901");
    }

    #[test]
    fn test_tax_amount_conversion() {
        // Test that cents are correctly converted to e9s
        // 250 cents = $2.50
        // e9s = cents * 10_000_000
        let cents: i64 = 250;
        let e9s = cents * 10_000_000;
        assert_eq!(e9s, 2_500_000_000);
    }

    #[test]
    fn test_reverse_charge_detection_with_vat_id_and_zero_tax() {
        // Reverse charge applies when: VAT ID present AND tax amount is 0
        let customer_tax_id = Some("eu_vat: DE123456789".to_string());
        let tax_amount_cents: Option<i64> = Some(0);

        let reverse_charge = customer_tax_id.is_some() && tax_amount_cents == Some(0);

        assert!(
            reverse_charge,
            "Reverse charge should be true with VAT ID and 0 tax"
        );
    }

    #[test]
    fn test_reverse_charge_detection_without_vat_id() {
        // No reverse charge if VAT ID is missing
        let customer_tax_id: Option<String> = None;
        let tax_amount_cents: Option<i64> = Some(0);

        let reverse_charge = customer_tax_id.is_some() && tax_amount_cents == Some(0);

        assert!(
            !reverse_charge,
            "Reverse charge should be false without VAT ID"
        );
    }

    #[test]
    fn test_reverse_charge_detection_with_vat_id_and_nonzero_tax() {
        // No reverse charge if tax is applied (domestic transaction)
        let customer_tax_id = Some("eu_vat: FR12345678901".to_string());
        let tax_amount_cents: Option<i64> = Some(250); // 19% VAT on €13.16

        let reverse_charge = customer_tax_id.is_some() && tax_amount_cents == Some(0);

        assert!(
            !reverse_charge,
            "Reverse charge should be false with VAT ID but non-zero tax"
        );
    }

    #[test]
    fn test_checkout_session_with_reverse_charge() {
        // Full session with reverse charge scenario
        let json = r#"{
            "id": "cs_test_reverse_charge",
            "metadata": {
                "contract_id": "abc123def456"
            },
            "total_details": {
                "amount_tax": 0
            },
            "customer_details": {
                "tax_ids": [
                    {
                        "type": "eu_vat",
                        "value": "DE123456789"
                    }
                ]
            }
        }"#;

        let session: StripeCheckoutSession = serde_json::from_str(json).unwrap();
        assert_eq!(session.id, "cs_test_reverse_charge");

        let tax_amount = session.total_details.as_ref().and_then(|td| td.amount_tax);
        let has_vat_id = session
            .customer_details
            .as_ref()
            .and_then(|cd| cd.tax_ids.as_ref())
            .map(|ids| !ids.is_empty())
            .unwrap_or(false);

        assert_eq!(tax_amount, Some(0));
        assert!(has_vat_id);

        // This would trigger reverse charge
        let reverse_charge = has_vat_id && tax_amount.unwrap_or(1) == 0;
        assert!(reverse_charge);
    }

    // Invoice webhook tests
    #[test]
    fn test_invoice_deserialization_with_metadata() {
        let json = r#"{
            "id": "in_test_123",
            "metadata": {
                "contract_id": "abc123def456"
            }
        }"#;

        let invoice: StripeInvoice = serde_json::from_str(json).unwrap();
        assert_eq!(invoice.id, "in_test_123");
        assert!(invoice.metadata.is_some());
        let contract_id = invoice
            .metadata
            .as_ref()
            .and_then(|m| m.get("contract_id"))
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(contract_id, "abc123def456");
    }

    #[test]
    fn test_invoice_deserialization_without_metadata() {
        let json = r#"{
            "id": "in_test_456"
        }"#;

        let invoice: StripeInvoice = serde_json::from_str(json).unwrap();
        assert_eq!(invoice.id, "in_test_456");
        assert!(invoice.metadata.is_none());
    }

    #[test]
    fn test_invoice_paid_event_deserialization() {
        let json = r#"{
            "type": "invoice.paid",
            "data": {
                "object": {
                    "id": "in_test_789",
                    "metadata": {
                        "contract_id": "abc123"
                    }
                }
            }
        }"#;

        let event: StripeEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "invoice.paid");

        let invoice: StripeInvoice = serde_json::from_value(event.data.object).unwrap();
        assert_eq!(invoice.id, "in_test_789");

        let contract_id = invoice
            .metadata
            .as_ref()
            .and_then(|m| m.get("contract_id"))
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(contract_id, "abc123");
    }

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
            .replay_orphan_dispute_pause(&contract_id, "pi_447_open")
            .await
            .expect("replay must not error");
        assert_eq!(outcome.paused, 1, "exactly one open dispute must be paused");
        assert_eq!(
            outcome.closed_lost_detected, 0,
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
            .replay_orphan_dispute_pause(&contract_id, "pi_447_open")
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
    async fn test_orphan_dispute_closed_lost_is_detected_not_auto_refunded() {
        // #447 money-path guard: if a dispute CLOSED as `lost` while it was
        // orphaned, the terminate+refund replay is a MONEY decision and is
        // intentionally NOT auto-applied (would need the idempotency-key path
        // + Stripe client + operator sign-off). The replay MUST:
        //  * detect the closed-lost orphan and surface it (so the webhook can
        //    page ops to review), and
        //  * NOT pause the contract (a lost dispute should terminate, not
        //    pause), and
        //  * NOT touch any refund column (no auto-refund -> no double-refund).
        let db = setup_test_db().await;
        let contract_id = vec![0xD2; 32];
        insert_active_contract(&db, &contract_id, None).await;

        // Orphan dispute that has already CLOSED LOST while orphaned.
        // Runtime query (not `query!`) to avoid churning `.sqlx` for a
        // test-only insert; the dispute schema is stable.
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
            .replay_orphan_dispute_pause(&contract_id, "pi_447_lost")
            .await
            .expect("replay must not error on closed-lost orphan");

        // The closed-lost orphan is surfaced for operator review.
        assert_eq!(
            outcome.closed_lost_detected, 1,
            "closed-lost orphan MUST be detected so ops can review the deferred refund"
        );
        assert_eq!(
            outcome.paused, 0,
            "a lost dispute MUST NOT be paused (terminate is the correct action, deferred)"
        );

        // Contract stays active: we neither paused nor terminated.
        assert_eq!(
            read_status(&db, &contract_id).await,
            "active",
            "closed-lost orphan replay MUST NOT transition the contract"
        );

        // MONEY-SAFETY: no refund columns mutated (the money replay is deferred).
        let money: (Option<i64>, Option<String>) = sqlx::query_as(
            "SELECT refund_amount_e9s, stripe_refund_id \
             FROM contract_sign_requests WHERE contract_id = $1",
        )
        .bind(&contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(
            money.0, None,
            "NO auto-refund -- the closed-lost money replay is deferred"
        );
        assert_eq!(money.1, None, "NO stripe_refund_id written");
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
            .replay_orphan_dispute_pause(&contract_id, "pi_447_req")
            .await
            .expect("replay MUST NOT error when contract is not pausable");
        assert_eq!(
            outcome.paused, 0,
            "pause MUST NOT be counted as applied when the contract cannot transition to paused"
        );
        assert_eq!(outcome.closed_lost_detected, 0);

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
            .replay_orphan_dispute_pause(&contract_id, "pi_447_req")
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
