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

// Stripe `charge.dispute.*` handlers live in a sibling module (issue #444
// large-file split). They are plain async fns invoked by the four dispute
// arms of `stripe_webhook` below.
use super::webhooks_disputes::{
    handle_dispute_closed, handle_dispute_created, handle_dispute_funds_withdrawn,
    handle_dispute_updated,
};

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
                        // #447: replay the dispute-lifecycle actions that the
                        // normal `charge.dispute.created` + `charge.dispute.closed`
                        // handlers apply but were missed while the dispute was
                        // orphaned. Money-safe (pause is status-only; terminate
                        // short-circuits on terminal state; refund uses the fixed
                        // `dispute:<id>` Stripe idempotency key). Best-effort:
                        // a replay failure MUST NOT fail the webhook (payment
                        // already succeeded), mirroring the relink above.
                        let stripe_client = crate::stripe_client::stripe_client_or_warn();
                        match db.replay_orphan_dispute_lifecycle(&contract_id_bytes, pi, stripe_client.as_ref()).await {
                            Ok(outcome) => {
                                if outcome.paused > 0 {
                                    tracing::info!(
                                        contract_id = %contract_id_hex,
                                        payment_intent = %pi,
                                        paused = outcome.paused,
                                        "Replayed missed dispute pause for re-linked orphan(s)"
                                    );
                                }
                                if outcome.terminated > 0 || outcome.refunded > 0 {
                                    tracing::info!(
                                        contract_id = %contract_id_hex,
                                        payment_intent = %pi,
                                        terminated = outcome.terminated,
                                        refunded = outcome.refunded,
                                        "Replayed missed dispute-lost terminate+refund for re-linked orphan(s)"
                                    );
                                    crate::notifications::telegram::send_ops_alert(&format!(
                                        "Stripe dispute re-linked after late checkout for contract {} \
                                         replayed {} terminate(s) + {} refund(s) for orphan dispute(s) closed LOST.",
                                        contract_id_hex, outcome.terminated, outcome.refunded
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

}
