use crate::chatwoot::ChatwootClient;
use crate::database::Database;
use crate::notifications::telegram::{TelegramClient, TelegramUpdate};
use crate::support_bot::handler::handle_customer_message;
use crate::support_bot::notifications::{dispatch_notification, SupportNotification};
use anyhow::{Context, Result};
use poem::{handler, web::Data, Body, Error as PoemError, Response};
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
    object: StripePaymentIntent,
}

#[derive(Debug, Deserialize)]
struct StripePaymentIntent {
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
        tracing::error!("Webhook signature verification failed: {}", e);
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
        "payment_intent.succeeded" => {
            let payment_intent_id = &event.data.object.id;
            tracing::info!("Payment succeeded: {}", payment_intent_id);

            // Update payment status to succeeded
            db.update_payment_status(payment_intent_id, "succeeded")
                .await
                .map_err(|e| {
                    tracing::error!("Failed to update payment status to succeeded: {}", e);
                    PoemError::from_string(
                        format!("Database error: {}", e),
                        poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                    )
                })?;

            // Auto-accept contract for Stripe payments
            // Get contract by payment_intent_id to find contract_id
            if let Ok(Some(contract)) = db.get_contract_by_payment_intent(payment_intent_id).await {
                if contract.payment_method == "stripe" {
                    if let Ok(contract_id_bytes) = hex::decode(&contract.contract_id) {
                        match db.accept_contract(&contract_id_bytes).await {
                            Ok(_) => {
                                tracing::info!(
                                    "Auto-accepted contract {} after successful Stripe payment",
                                    &contract.contract_id
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to auto-accept contract {}: {}",
                                    &contract.contract_id,
                                    e
                                );
                                // Don't fail the webhook - payment status is already updated
                            }
                        }
                    }
                }
            } else {
                tracing::warn!(
                    "Contract not found for payment_intent_id: {}",
                    payment_intent_id
                );
            }
        }
        "payment_intent.payment_failed" => {
            let payment_intent_id = &event.data.object.id;
            tracing::warn!("Payment failed: {}", payment_intent_id);

            db.update_payment_status(payment_intent_id, "failed")
                .await
                .map_err(|e| {
                    tracing::error!("Failed to update payment status to failed: {}", e);
                    PoemError::from_string(
                        format!("Database error: {}", e),
                        poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                    )
                })?;
        }
        _ => {
            tracing::debug!("Unhandled event type: {}", event.event_type);
        }
    }

    Ok(Response::builder()
        .status(poem::http::StatusCode::OK)
        .body(""))
}

// Chatwoot webhook types
#[derive(Debug, Deserialize)]
struct ChatwootWebhookPayload {
    event: String,
    conversation: Option<ChatwootConversation>,
    message: Option<ChatwootMessage>,
}

#[derive(Debug, Deserialize)]
struct ChatwootConversation {
    id: i64,
    status: Option<String>,
    custom_attributes: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ChatwootMessage {
    id: i64,
    message_type: String,
    created_at: i64,
    content: Option<String>,
}

/// Handle Chatwoot webhook events for response time tracking and AI bot
#[handler]
pub async fn chatwoot_webhook(db: Data<&Arc<Database>>, body: Body) -> Result<Response, PoemError> {
    let body_bytes = body.into_vec().await.map_err(|e| {
        PoemError::from_string(
            format!("Failed to read body: {}", e),
            poem::http::StatusCode::BAD_REQUEST,
        )
    })?;

    let payload: ChatwootWebhookPayload = serde_json::from_slice(&body_bytes).map_err(|e| {
        PoemError::from_string(
            format!("Invalid JSON: {}", e),
            poem::http::StatusCode::BAD_REQUEST,
        )
    })?;

    tracing::info!("Received Chatwoot webhook: {}", payload.event);

    if payload.event == "conversation_status_changed" {
        if let Some(conv) = payload.conversation {
            // Check if status changed to "open" (human handoff)
            if let Some(status) = conv.status {
                if status == "open" {
                    // Extract contract_id from custom_attributes
                    let contract_id = conv
                        .custom_attributes
                        .as_ref()
                        .and_then(|attrs| attrs.get("contract_id"))
                        .and_then(|v| v.as_str());

                    if let Some(contract_id) = contract_id {
                        // Lookup contract to get provider pubkey
                        match hex::decode(contract_id) {
                            Ok(contract_id_bytes) => {
                                match db.get_contract(&contract_id_bytes).await {
                                    Ok(Some(contract)) => {
                                        // Decode provider pubkey from hex
                                        match hex::decode(&contract.provider_pubkey) {
                                            Ok(provider_pubkey_bytes) => {
                                                // Get Chatwoot base URL for notification link
                                                let chatwoot_url = std::env::var(
                                                    "CHATWOOT_FRONTEND_URL",
                                                )
                                                .expect("CHATWOOT_FRONTEND_URL must be set");

                                                let notification = SupportNotification::new(
                                                    provider_pubkey_bytes,
                                                    conv.id,
                                                    contract_id.to_string(),
                                                    "Customer conversation escalated to human support"
                                                        .to_string(),
                                                    &chatwoot_url,
                                                );

                                                if let Err(e) =
                                                    dispatch_notification(&db, &notification).await
                                                {
                                                    tracing::error!(
                                                        "Failed to dispatch notification for conversation {}: {}",
                                                        conv.id,
                                                        e
                                                    );
                                                    // Don't fail webhook - notification failure shouldn't block event processing
                                                }
                                            }
                                            Err(e) => {
                                                tracing::warn!(
                                                    "Invalid provider_pubkey hex for conversation {}: {}",
                                                    conv.id,
                                                    e
                                                );
                                            }
                                        }
                                    }
                                    Ok(None) => {
                                        tracing::warn!(
                                            "Contract not found for conversation {} (contract_id: {})",
                                            conv.id,
                                            contract_id
                                        );
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            "Failed to lookup contract for conversation {}: {}",
                                            conv.id,
                                            e
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Invalid contract_id hex for conversation {}: {}",
                                    conv.id,
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }
    } else if payload.event == "message_created" {
        if let (Some(conv), Some(msg)) = (payload.conversation, payload.message) {
            // Extract contract_id from custom_attributes
            let contract_id = conv
                .custom_attributes
                .as_ref()
                .and_then(|attrs| attrs.get("contract_id"))
                .and_then(|v| v.as_str());

            if let Some(contract_id) = contract_id {
                let sender_type = match msg.message_type.as_str() {
                    "incoming" => "customer",
                    "outgoing" => "provider",
                    _ => {
                        return Ok(Response::builder()
                            .status(poem::http::StatusCode::OK)
                            .body(""))
                    }
                };

                // Track message for response time
                if let Err(e) = db
                    .insert_chatwoot_message_event(
                        contract_id,
                        conv.id,
                        msg.id,
                        sender_type,
                        msg.created_at,
                    )
                    .await
                {
                    tracing::warn!("Failed to insert Chatwoot message event: {}", e);
                    // Don't fail webhook - event may be duplicate
                }

                // If this is an incoming customer message, trigger bot response
                if sender_type == "customer" {
                    if let Some(content) = msg.content {
                        if !content.trim().is_empty() {
                            // Try to create Chatwoot client and handle message
                            match ChatwootClient::from_env() {
                                Ok(chatwoot) => {
                                    if let Err(e) = handle_customer_message(
                                        &db,
                                        &chatwoot,
                                        conv.id as u64,
                                        contract_id,
                                        &content,
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
                                Err(e) => {
                                    tracing::warn!(
                                        "Chatwoot client not configured, skipping bot response: {}",
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(Response::builder()
        .status(poem::http::StatusCode::OK)
        .body(""))
}

// ICPay webhook types
#[derive(Debug, Deserialize)]
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
        tracing::error!("ICPay webhook signature verification failed: {}", e);
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

                        // Auto-accept contract for ICPay payments
                        match db.accept_contract(&contract_id_bytes).await {
                            Ok(_) => {
                                tracing::info!(
                                    "Auto-accepted contract {} after successful ICPay payment",
                                    contract_id_hex
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to auto-accept contract {}: {}",
                                    contract_id_hex,
                                    e
                                );
                                // Don't fail the webhook - payment status is already updated
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Invalid contract_id hex in ICPay webhook metadata: {}",
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
                            "Invalid contract_id hex in ICPay webhook metadata: {}",
                            e
                        );
                    }
                }
            }
        }
        "payment.refunded" => {
            let payment_id = &event.data.object.id;
            tracing::info!("ICPay payment refunded (webhook confirmation): {}", payment_id);
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
pub async fn telegram_webhook(db: Data<&Arc<Database>>, body: Body) -> Result<Response, PoemError> {
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

                if let Ok(telegram) = TelegramClient::from_env() {
                    let response_text = format!(
                        "Welcome! Your Telegram Chat ID is:\n\n`{}`\n\n\
                        Copy this ID and paste it in your notification settings at:\n\
                        Dashboard → Account → Notifications → Telegram Chat ID\n\n\
                        Once configured, you'll receive support escalation alerts here.",
                        chat_id
                    );

                    if let Err(e) = telegram.send_message(&chat_id, &response_text).await {
                        tracing::error!("Failed to send /start response: {}", e);
                    }
                }

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
                    tracing::error!("Failed to lookup Telegram conversation: {}", e);
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
                                tracing::error!("Chatwoot client not configured: {}", e);
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

        let result = verify_icpay_signature(payload, &signature, secret);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_icpay_signature_invalid() {
        let payload = r#"{"test":"data"}"#;
        let secret = "whsec_test_secret";
        let signature = "t=1234567890,v1=invalid_signature";

        let result = verify_icpay_signature(payload, signature, secret);
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
}
