use crate::database::Database;
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

            db.update_payment_status(payment_intent_id, "succeeded")
                .await
                .map_err(|e| {
                    tracing::error!("Failed to update payment status to succeeded: {}", e);
                    PoemError::from_string(
                        format!("Database error: {}", e),
                        poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                    )
                })?;
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
}
