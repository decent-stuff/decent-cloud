use anyhow::Result;
use dcc_common::DccIdentity;
use poem::{error::ResponseError, http::StatusCode, FromRequest, Request, RequestBody};
use std::fmt;

/// Authenticated user with verified public key
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub pubkey_hash: Vec<u8>,
}

/// Authentication error types
#[derive(Debug)]
pub enum AuthError {
    MissingHeader(String),
    InvalidFormat(String),
    InvalidSignature(String),
    TimestampExpired,
    InternalError(String),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AuthError::MissingHeader(h) => write!(f, "Missing required header: {}", h),
            AuthError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            AuthError::InvalidSignature(msg) => write!(f, "Invalid signature: {}", msg),
            AuthError::TimestampExpired => write!(f, "Request timestamp expired (max 5 minutes)"),
            AuthError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for AuthError {}

impl ResponseError for AuthError {
    fn status(&self) -> StatusCode {
        match self {
            AuthError::MissingHeader(_) => StatusCode::UNAUTHORIZED,
            AuthError::InvalidFormat(_) => StatusCode::BAD_REQUEST,
            AuthError::InvalidSignature(_) => StatusCode::UNAUTHORIZED,
            AuthError::TimestampExpired => StatusCode::UNAUTHORIZED,
            AuthError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// Verify request signature
///
/// Message format: timestamp + method + path + body
/// NOTE: Path excludes query string for robustness (query params are typically non-critical)
///
/// Headers required:
/// - X-Public-Key: hex-encoded public key (32 bytes)
/// - X-Signature: hex-encoded signature (64 bytes)
/// - X-Timestamp: unix timestamp in nanoseconds
pub fn verify_request_signature(
    pubkey_hex: &str,
    signature_hex: &str,
    timestamp_str: &str,
    method: &str,
    path: &str,
    body: &[u8],
) -> Result<Vec<u8>, AuthError> {
    // Decode public key
    let pubkey = hex::decode(pubkey_hex)
        .map_err(|e| AuthError::InvalidFormat(format!("Invalid public key hex encoding: {}", e)))?;

    if pubkey.len() != 32 {
        return Err(AuthError::InvalidFormat(format!(
            "Public key must be 32 bytes, got {}",
            pubkey.len()
        )));
    }

    // Decode signature
    let signature = hex::decode(signature_hex)
        .map_err(|e| AuthError::InvalidFormat(format!("Invalid signature hex encoding: {}", e)))?;

    if signature.len() != 64 {
        return Err(AuthError::InvalidFormat(format!(
            "Signature must be 64 bytes, got {}",
            signature.len()
        )));
    }

    // Parse timestamp
    let timestamp = timestamp_str
        .parse::<i64>()
        .map_err(|e| AuthError::InvalidFormat(format!("Invalid timestamp: {}", e)))?;

    // Verify timestamp freshness (within 5 minutes)
    let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let max_age_ns = 5 * 60 * 1_000_000_000; // 5 minutes
    let age = now - timestamp;

    if age > max_age_ns || age < -max_age_ns {
        return Err(AuthError::TimestampExpired);
    }

    // Construct message: timestamp + method + path + body
    let mut message = timestamp_str.as_bytes().to_vec();
    message.extend_from_slice(method.as_bytes());
    message.extend_from_slice(path.as_bytes());
    message.extend_from_slice(body);

    // Verify signature
    let identity = DccIdentity::new_verifying_from_bytes(&pubkey)
        .map_err(|e| AuthError::InternalError(format!("Failed to create identity: {}", e)))?;

    identity.verify_bytes(&message, &signature).map_err(|e| {
        AuthError::InvalidSignature(format!("Signature verification failed: {}", e))
    })?;

    Ok(pubkey)
}

/// Poem extractor for authenticated requests
impl FromRequest<'_> for AuthenticatedUser {
    async fn from_request(req: &Request, body: &mut RequestBody) -> poem::Result<Self> {
        // Extract headers
        let headers = req.headers();

        let pubkey_hex = headers
            .get("X-Public-Key")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AuthError::MissingHeader("X-Public-Key".to_string()))?;

        let signature_hex = headers
            .get("X-Signature")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AuthError::MissingHeader("X-Signature".to_string()))?;

        let timestamp = headers
            .get("X-Timestamp")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AuthError::MissingHeader("X-Timestamp".to_string()))?;

        // Read body
        let body_bytes = body.take()?.into_vec().await?;

        // Verify signature
        // NOTE: Query strings are intentionally excluded from signature for robustness
        // Security trade-off: query params could be manipulated, but they're typically
        // non-critical (filters, pagination, options). Body and path integrity maintained.
        let pubkey_hash = verify_request_signature(
            pubkey_hex,
            signature_hex,
            timestamp,
            req.method().as_str(),
            req.uri().path(), // Path only, no query string
            &body_bytes,
        )?;

        // Restore body for downstream handlers
        *body = RequestBody::new(poem::Body::from(body_bytes));

        Ok(AuthenticatedUser { pubkey_hash })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcc_common::DccIdentity;

    fn create_test_identity() -> (DccIdentity, Vec<u8>) {
        let seed = [42u8; 32];
        let identity = DccIdentity::new_from_seed(&seed).unwrap();
        let pubkey = identity.to_bytes_verifying();
        (identity, pubkey)
    }

    #[test]
    fn test_verify_valid_signature() {
        let (identity, pubkey) = create_test_identity();
        let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap();
        let method = "POST";
        let path = "/api/v1/users/abc123/profile";
        let body = b"{\"display_name\":\"Test User\"}";

        // Construct message
        let timestamp_str = timestamp.to_string();
        let mut message = timestamp_str.as_bytes().to_vec();
        message.extend_from_slice(method.as_bytes());
        message.extend_from_slice(path.as_bytes());
        message.extend_from_slice(body);

        // Sign message
        let signature = identity.sign(&message).unwrap();

        // Verify
        let result = verify_request_signature(
            &hex::encode(&pubkey),
            &hex::encode(signature.to_bytes()),
            &timestamp_str,
            method,
            path,
            body,
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), pubkey);
    }

    #[test]
    fn test_verify_invalid_signature() {
        let (identity, pubkey) = create_test_identity();
        let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap();
        let method = "POST";
        let path = "/api/v1/users/abc123/profile";
        let body = b"{\"display_name\":\"Test User\"}";

        // Construct message
        let timestamp_str = timestamp.to_string();
        let mut message = timestamp_str.as_bytes().to_vec();
        message.extend_from_slice(method.as_bytes());
        message.extend_from_slice(path.as_bytes());
        message.extend_from_slice(body);

        // Sign message
        let signature = identity.sign(&message).unwrap();

        // Tamper with body
        let tampered_body = b"{\"display_name\":\"Hacker\"}";

        // Verify should fail
        let result = verify_request_signature(
            &hex::encode(&pubkey),
            &hex::encode(signature.to_bytes()),
            &timestamp_str,
            method,
            path,
            tampered_body,
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AuthError::InvalidSignature(_)
        ));
    }

    #[test]
    fn test_verify_expired_timestamp() {
        let (identity, pubkey) = create_test_identity();
        // Timestamp from 10 minutes ago
        let timestamp =
            chrono::Utc::now().timestamp_nanos_opt().unwrap() - (10 * 60 * 1_000_000_000);
        let method = "POST";
        let path = "/api/v1/users/abc123/profile";
        let body = b"{}";

        let timestamp_str = timestamp.to_string();
        let mut message = timestamp_str.as_bytes().to_vec();
        message.extend_from_slice(method.as_bytes());
        message.extend_from_slice(path.as_bytes());
        message.extend_from_slice(body);

        let signature = identity.sign(&message).unwrap();

        let result = verify_request_signature(
            &hex::encode(&pubkey),
            &hex::encode(signature.to_bytes()),
            &timestamp_str,
            method,
            path,
            body,
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::TimestampExpired));
    }

    #[test]
    fn test_verify_invalid_pubkey_length() {
        let result = verify_request_signature(
            "deadbeef", // Too short
            &hex::encode([0u8; 64]),
            "1234567890",
            "POST",
            "/test",
            b"{}",
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::InvalidFormat(_)));
    }

    #[test]
    fn test_verify_invalid_signature_length() {
        let (_, pubkey) = create_test_identity();
        let result = verify_request_signature(
            &hex::encode(&pubkey),
            "deadbeef", // Too short
            "1234567890",
            "POST",
            "/test",
            b"{}",
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::InvalidFormat(_)));
    }
}
