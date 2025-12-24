use crate::chatwoot::ChatwootClient;
use crate::database::Database;
use crate::notifications::telegram::{TelegramClient, TelegramUpdate};
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

// Subscription webhook types
#[derive(Debug, Deserialize)]
struct StripeSubscription {
    id: String,
    customer: String,
    status: String,
    current_period_end: i64,
    cancel_at_period_end: bool,
    items: StripeSubscriptionItems,
    #[allow(dead_code)]
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct StripeSubscriptionItems {
    data: Vec<StripeSubscriptionItem>,
}

#[derive(Debug, Deserialize)]
struct StripeSubscriptionItem {
    price: StripePrice,
}

#[derive(Debug, Deserialize)]
struct StripePrice {
    id: String,
}

/// Verify Stripe webhook signature
fn verify_signature(payload: &str, signature: &str, secret: &str) -> Result<()> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    // Parse signature header (format: "t=timestamp,v1=signature")
    let parts: Vec<&str> = signature.split(',').collect();
    let mut timestamp = None;
    let mut sig_hash = None;

    for part in parts {
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

    // Construct signed payload
    let signed_payload = format!("{}.{}", timestamp, payload);

    // Compute HMAC
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).context("Invalid webhook secret")?;
    mac.update(signed_payload.as_bytes());
    let result = mac.finalize();
    let computed_hash = hex::encode(result.into_bytes());

    // Compare signatures
    if computed_hash != sig_hash {
        return Err(anyhow::anyhow!("Invalid signature"));
    }

    Ok(())
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

            let contract_id_bytes = hex::decode(contract_id_hex).map_err(|e| {
                tracing::error!("Invalid contract_id hex: {:#}", e);
                PoemError::from_string(
                    format!("Invalid contract_id: {}", e),
                    poem::http::StatusCode::BAD_REQUEST,
                )
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

            // Update contract with tax info and set payment status to succeeded
            if let Err(e) = db
                .update_checkout_session_payment(
                    &contract_id_bytes,
                    &session.id,
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

            // Notify provider about new rental request
            if let Ok(Some(contract)) = db.get_contract(&contract_id_bytes).await {
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
                    // Don't fail the webhook - payment succeeded
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
                match hex::decode(contract_id_hex) {
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
                        let _ = db.remove_pending_stripe_receipt(&contract_id_bytes).await;

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
                        tracing::warn!("Invalid contract_id hex in invoice metadata: {:#}", e);
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
        // Subscription lifecycle events
        "customer.subscription.created" | "customer.subscription.updated" => {
            let subscription: StripeSubscription =
                serde_json::from_value(event.data.object).map_err(|e| {
                    tracing::error!("Failed to parse subscription: {:#}", e);
                    PoemError::from_string(
                        format!("Invalid subscription data: {}", e),
                        poem::http::StatusCode::BAD_REQUEST,
                    )
                })?;

            tracing::info!(
                "Subscription {}: {} (status: {}, customer: {})",
                event.event_type,
                subscription.id,
                subscription.status,
                subscription.customer
            );

            // Find account by Stripe customer ID
            let account_id = match db
                .get_account_id_by_stripe_customer(&subscription.customer)
                .await
            {
                Ok(Some(id)) => id,
                Ok(None) => {
                    tracing::warn!(
                        "No account found for Stripe customer {}, skipping subscription update",
                        subscription.customer
                    );
                    return Ok(Response::builder()
                        .status(poem::http::StatusCode::OK)
                        .body(""));
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to lookup account for customer {}: {}",
                        subscription.customer,
                        e
                    );
                    return Err(PoemError::from_string(
                        format!("Database error: {}", e),
                        poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                    ));
                }
            };

            // Get price ID from subscription items
            let price_id = subscription
                .items
                .data
                .first()
                .map(|item| item.price.id.as_str());

            // Find plan by Stripe price ID
            let plan_id = if let Some(price_id) = price_id {
                match db.get_subscription_plan_by_stripe_price(price_id).await {
                    Ok(Some(plan)) => plan.id,
                    Ok(None) => {
                        tracing::warn!(
                            "No plan found for Stripe price {}, using 'pro' as default",
                            price_id
                        );
                        "pro".to_string()
                    }
                    Err(e) => {
                        tracing::error!("Failed to lookup plan by price {}: {}", price_id, e);
                        "pro".to_string()
                    }
                }
            } else {
                tracing::warn!("Subscription {} has no price items", subscription.id);
                "pro".to_string()
            };

            // Map Stripe status to our status
            let status = match subscription.status.as_str() {
                "active" => "active",
                "trialing" => "trialing",
                "past_due" => "past_due",
                "canceled" | "unpaid" | "incomplete_expired" => "canceled",
                "incomplete" | "paused" => "past_due",
                other => {
                    tracing::warn!("Unknown subscription status: {}", other);
                    "active"
                }
            };

            // Convert current_period_end to nanoseconds
            let period_end_ns = subscription.current_period_end * 1_000_000_000;

            // Update account subscription
            if let Err(e) = db
                .update_account_subscription(
                    &account_id,
                    &plan_id,
                    status,
                    Some(&subscription.id),
                    Some(period_end_ns),
                    subscription.cancel_at_period_end,
                )
                .await
            {
                tracing::error!(
                    "Failed to update account subscription for customer {}: {}",
                    subscription.customer,
                    e
                );
                return Err(PoemError::from_string(
                    format!("Database error: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }

            // Record event for audit trail
            let _ = db
                .insert_subscription_event(
                    &account_id,
                    crate::database::SubscriptionEventInput {
                        event_type: &event.event_type,
                        new_plan_id: Some(&plan_id),
                        stripe_subscription_id: Some(&subscription.id),
                        ..Default::default()
                    },
                )
                .await;

            tracing::info!(
                "Updated subscription for customer {}: plan={}, status={}",
                subscription.customer,
                plan_id,
                status
            );
        }

        "customer.subscription.deleted" => {
            let subscription: StripeSubscription =
                serde_json::from_value(event.data.object).map_err(|e| {
                    tracing::error!("Failed to parse subscription: {:#}", e);
                    PoemError::from_string(
                        format!("Invalid subscription data: {}", e),
                        poem::http::StatusCode::BAD_REQUEST,
                    )
                })?;

            tracing::info!(
                "Subscription deleted: {} (customer: {})",
                subscription.id,
                subscription.customer
            );

            // Find account by Stripe customer ID
            let account_id = match db
                .get_account_id_by_stripe_customer(&subscription.customer)
                .await
            {
                Ok(Some(id)) => id,
                Ok(None) => {
                    tracing::warn!(
                        "No account found for Stripe customer {}, skipping subscription delete",
                        subscription.customer
                    );
                    return Ok(Response::builder()
                        .status(poem::http::StatusCode::OK)
                        .body(""));
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to lookup account for customer {}: {}",
                        subscription.customer,
                        e
                    );
                    return Err(PoemError::from_string(
                        format!("Database error: {}", e),
                        poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                    ));
                }
            };

            // Reset to free tier
            if let Err(e) = db
                .update_account_subscription(&account_id, "free", "active", None, None, false)
                .await
            {
                tracing::error!(
                    "Failed to reset subscription to free for customer {}: {}",
                    subscription.customer,
                    e
                );
                return Err(PoemError::from_string(
                    format!("Database error: {}", e),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }

            // Record event for audit trail
            let _ = db
                .insert_subscription_event(
                    &account_id,
                    crate::database::SubscriptionEventInput {
                        event_type: "deleted",
                        new_plan_id: Some("free"),
                        stripe_subscription_id: Some(&subscription.id),
                        ..Default::default()
                    },
                )
                .await;

            tracing::info!(
                "Subscription deleted for customer {}, reset to free tier",
                subscription.customer
            );
        }

        "invoice.payment_failed" => {
            // Check if this is a subscription invoice
            let invoice: StripeInvoice =
                serde_json::from_value(event.data.object.clone()).map_err(|e| {
                    tracing::error!("Failed to parse invoice: {:#}", e);
                    PoemError::from_string(
                        format!("Invalid invoice data: {}", e),
                        poem::http::StatusCode::BAD_REQUEST,
                    )
                })?;

            tracing::warn!("Invoice payment failed: {}", invoice.id);

            // Check if there's a subscription field in the raw data
            if let Some(subscription_id) = event
                .data
                .object
                .get("subscription")
                .and_then(|v| v.as_str())
            {
                tracing::warn!(
                    "Subscription {} invoice payment failed - user should update payment method",
                    subscription_id
                );
                // The subscription status will be updated by customer.subscription.updated webhook
                // which Stripe sends when a subscription enters past_due status
            }
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

// ICPay webhook types
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct IcpayWebhookEvent {
    id: String,
    #[serde(rename = "type")]
    event_type: String,
    data: IcpayWebhookData,
}

#[derive(Debug, Deserialize)]
struct IcpayWebhookData {
    object: IcpayPaymentObject,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct IcpayPaymentObject {
    id: String,
    status: String,
    metadata: Option<serde_json::Value>,
}

/// Verify ICPay webhook signature (same format as Stripe)
fn verify_icpay_signature(payload: &str, signature: &str, secret: &str) -> Result<()> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    // Parse signature header (format: "t=timestamp,v1=signature")
    let parts: Vec<&str> = signature.split(',').collect();
    let mut timestamp = None;
    let mut sig_hash = None;

    for part in parts {
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

    // Check timestamp tolerance (300 seconds)
    let ts = timestamp.parse::<i64>().context("Invalid timestamp")?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .context("System time error")?
        .as_secs() as i64;
    if (now - ts).abs() > 300 {
        return Err(anyhow::anyhow!("Timestamp outside tolerance window"));
    }

    // Construct signed payload
    let signed_payload = format!("{}.{}", timestamp, payload);

    // Compute HMAC
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).context("Invalid webhook secret")?;
    mac.update(signed_payload.as_bytes());
    let result = mac.finalize();
    let computed_hash = hex::encode(result.into_bytes());

    // Compare signatures
    if computed_hash != sig_hash {
        return Err(anyhow::anyhow!("Invalid signature"));
    }

    Ok(())
}

/// Handle ICPay webhook events
#[handler]
pub async fn icpay_webhook(
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
        .get("x-icpay-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            PoemError::from_string(
                "Missing x-icpay-signature header",
                poem::http::StatusCode::BAD_REQUEST,
            )
        })?;

    // Get webhook secret from environment
    let webhook_secret = std::env::var("ICPAY_WEBHOOK_SECRET").map_err(|_| {
        PoemError::from_string(
            "ICPAY_WEBHOOK_SECRET not configured",
            poem::http::StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;

    // Verify signature
    verify_icpay_signature(&payload, signature, &webhook_secret).map_err(|e| {
        tracing::error!("ICPay webhook signature verification failed: {:#}", e);
        PoemError::from_string("Invalid signature", poem::http::StatusCode::UNAUTHORIZED)
    })?;

    // Parse event
    let event: IcpayWebhookEvent = serde_json::from_slice(&body_bytes).map_err(|e| {
        PoemError::from_string(
            format!("Invalid JSON: {}", e),
            poem::http::StatusCode::BAD_REQUEST,
        )
    })?;

    tracing::info!("Received ICPay webhook: {}", event.event_type);

    // Handle event types
    match event.event_type.as_str() {
        "payment.completed" => {
            let payment_id = &event.data.object.id;
            tracing::info!("ICPay payment completed: {}", payment_id);

            // Extract contract_id from metadata
            let contract_id_hex = event
                .data
                .object
                .metadata
                .as_ref()
                .and_then(|m| m.get("contract_id"))
                .and_then(|v| v.as_str());

            if let Some(contract_id_hex) = contract_id_hex {
                match hex::decode(contract_id_hex) {
                    Ok(contract_id_bytes) => {
                        // Update contract with ICPay payment ID and set payment status to succeeded
                        if let Err(e) = db
                            .update_icpay_payment_confirmed(&contract_id_bytes, payment_id)
                            .await
                        {
                            tracing::error!(
                                "Failed to update ICPay payment confirmation for contract {}: {}",
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
                            "Contract {} ICPay payment succeeded, awaiting provider review",
                            contract_id_hex
                        );

                        // Notify provider about new rental request
                        if let Ok(Some(contract)) = db.get_contract(&contract_id_bytes).await {
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
                                // Don't fail the webhook - payment succeeded
                            }
                        }

                        // Send payment receipt with invoice attachment
                        match crate::receipts::send_payment_receipt(
                            db.as_ref(),
                            &contract_id_bytes,
                            email_service.as_ref(),
                        )
                        .await
                        {
                            Ok(receipt_num) => {
                                tracing::info!(
                                    "Sent receipt #{} for contract {} after ICPay payment",
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
                                // Don't fail the webhook - payment was successful
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Invalid contract_id hex in ICPay webhook metadata: {:#}",
                            e
                        );
                    }
                }
            } else {
                tracing::warn!(
                    "Missing contract_id in ICPay webhook metadata for payment {}",
                    payment_id
                );
            }
        }
        "payment.failed" => {
            let payment_id = &event.data.object.id;
            tracing::warn!("ICPay payment failed: {}", payment_id);

            // Extract contract_id from metadata
            let contract_id_hex = event
                .data
                .object
                .metadata
                .as_ref()
                .and_then(|m| m.get("contract_id"))
                .and_then(|v| v.as_str());

            if let Some(contract_id_hex) = contract_id_hex {
                match hex::decode(contract_id_hex) {
                    Ok(contract_id_bytes) => {
                        // Update payment status to failed
                        if let Err(e) = db
                            .update_icpay_payment_status(&contract_id_bytes, "failed")
                            .await
                        {
                            tracing::error!(
                                "Failed to update ICPay payment status to failed for contract {}: {}",
                                contract_id_hex,
                                e
                            );
                            return Err(PoemError::from_string(
                                format!("Database error: {}", e),
                                poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                            ));
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Invalid contract_id hex in ICPay webhook metadata: {:#}",
                            e
                        );
                    }
                }
            }
        }
        "payment.refunded" => {
            let payment_id = &event.data.object.id;
            tracing::info!(
                "ICPay payment refunded (webhook confirmation): {}",
                payment_id
            );
            // Refund already processed via API call, this is just confirmation
        }
        _ => {
            tracing::debug!("Unhandled ICPay event type: {}", event.event_type);
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
        tracing::warn!(
            "TELEGRAM_WEBHOOK_SECRET not set - webhook requests are NOT verified! \
             Set this env var and use it when calling setWebhook API."
        );
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

    // ICPay webhook tests
    #[test]
    fn test_verify_icpay_signature_valid() {
        let payload = r#"{"test":"data"}"#;
        let secret = "whsec_test_secret";

        // Generate valid signature with current timestamp
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();
        let signed_payload = format!("{}.{}", timestamp, payload);
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signed_payload.as_bytes());
        let sig_hash = hex::encode(mac.finalize().into_bytes());

        let signature = format!("t={},v1={}", timestamp, sig_hash);

        let result = verify_icpay_signature(payload, &signature, secret);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_icpay_signature_invalid() {
        let payload = r#"{"test":"data"}"#;
        let secret = "whsec_test_secret";
        // Use current timestamp but invalid signature hash
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let signature = format!("t={},v1=invalid_signature", timestamp);

        let result = verify_icpay_signature(payload, &signature, secret);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid signature"));
    }

    #[test]
    fn test_verify_icpay_signature_expired_timestamp() {
        let payload = r#"{"test":"data"}"#;
        let secret = "whsec_test_secret";

        // Use an old timestamp (more than 300 seconds ago)
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        let old_timestamp = "1000000000"; // Very old timestamp
        let signed_payload = format!("{}.{}", old_timestamp, payload);
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signed_payload.as_bytes());
        let sig_hash = hex::encode(mac.finalize().into_bytes());

        let signature = format!("t={},v1={}", old_timestamp, sig_hash);

        let result = verify_icpay_signature(payload, &signature, secret);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Timestamp outside tolerance"));
    }

    #[test]
    fn test_verify_icpay_signature_missing_timestamp() {
        let payload = r#"{"test":"data"}"#;
        let secret = "whsec_test_secret";
        let signature = "v1=somehash";

        let result = verify_icpay_signature(payload, signature, secret);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timestamp"));
    }

    #[test]
    fn test_verify_icpay_signature_missing_v1() {
        let payload = r#"{"test":"data"}"#;
        let secret = "whsec_test_secret";
        let signature = "t=1234567890";

        let result = verify_icpay_signature(payload, signature, secret);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("v1"));
    }

    #[test]
    fn test_icpay_webhook_event_deserialization() {
        let json = r#"{
            "id": "evt_123",
            "type": "payment.completed",
            "data": {
                "object": {
                    "id": "pay_456",
                    "status": "succeeded",
                    "metadata": {
                        "contract_id": "abc123"
                    }
                }
            }
        }"#;

        let event: IcpayWebhookEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.id, "evt_123");
        assert_eq!(event.event_type, "payment.completed");
        assert_eq!(event.data.object.id, "pay_456");
        assert_eq!(event.data.object.status, "succeeded");
        assert!(event.data.object.metadata.is_some());
    }

    #[test]
    fn test_icpay_webhook_event_without_metadata() {
        let json = r#"{
            "id": "evt_789",
            "type": "payment.failed",
            "data": {
                "object": {
                    "id": "pay_999",
                    "status": "failed"
                }
            }
        }"#;

        let event: IcpayWebhookEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "payment.failed");
        assert!(event.data.object.metadata.is_none());
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
