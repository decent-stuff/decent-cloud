use anyhow::Result;
use dcc_common::DccIdentity;
use poem::{error::ResponseError, http::StatusCode, FromRequest, Request, RequestBody};
use poem_openapi::registry::MetaSecurityScheme;
use serde::{Deserialize, Serialize};
use std::fmt;
use ts_rs::TS;

/// Authenticated user with verified public key
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub pubkey: Vec<u8>,
}

/// Headers for signed API requests
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
pub struct SignedRequestHeaders {
    #[serde(rename = "X-Public-Key")]
    #[ts(rename = "X-Public-Key")]
    pub x_public_key: String,
    #[serde(rename = "X-Signature")]
    #[ts(rename = "X-Signature")]
    pub x_signature: String,
    #[serde(rename = "X-Timestamp")]
    #[ts(rename = "X-Timestamp")]
    pub x_timestamp: String,
    #[serde(rename = "X-Nonce")]
    #[ts(rename = "X-Nonce")]
    pub x_nonce: String,
    #[serde(rename = "Content-Type")]
    #[ts(rename = "Content-Type")]
    pub content_type: String,
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
/// Message format: timestamp + nonce + method + path + body
/// NOTE: Path excludes query string for robustness (query params are typically non-critical)
///
/// Headers required:
/// - X-Public-Key: hex-encoded public key (32 bytes)
/// - X-Signature: hex-encoded signature (64 bytes)
/// - X-Timestamp: unix timestamp in nanoseconds
/// - X-Nonce: UUID v4 for replay prevention
pub fn verify_request_signature(
    pubkey_hex: &str,
    signature_hex: &str,
    timestamp_str: &str,
    nonce_str: &str,
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

    // Validate nonce format (UUID v4)
    uuid::Uuid::parse_str(nonce_str)
        .map_err(|e| AuthError::InvalidFormat(format!("Invalid nonce format (must be UUID v4): {}", e)))?;

    // Construct message: timestamp + nonce + method + path + body
    let mut message = timestamp_str.as_bytes().to_vec();
    message.extend_from_slice(nonce_str.as_bytes());
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

        let nonce = headers
            .get("X-Nonce")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AuthError::MissingHeader("X-Nonce".to_string()))?;

        // Read body
        let body_bytes = body.take()?.into_vec().await?;

        // Verify signature
        // NOTE: Query strings are intentionally excluded from signature for robustness
        // Security trade-off: query params could be manipulated, but they're typically
        // non-critical (filters, pagination, options). Body and path integrity maintained.
        let pubkey = verify_request_signature(
            pubkey_hex,
            signature_hex,
            timestamp,
            nonce,
            req.method().as_str(),
            req.uri().path(), // Path only, no query string
            &body_bytes,
        )?;

        // Restore body for downstream handlers
        *body = RequestBody::new(poem::Body::from(body_bytes));

        Ok(AuthenticatedUser { pubkey })
    }
}

/// poem-openapi compatible authenticated user
/// Uses custom headers for signature-based authentication
#[derive(Debug, Clone)]
pub struct ApiAuthenticatedUser {
    pub pubkey: Vec<u8>,
}

impl ApiAuthenticatedUser {
    fn from_headers(
        pubkey_hex: &str,
        signature_hex: &str,
        timestamp: &str,
        nonce: &str,
        method: &str,
        path: &str,
        body: &[u8],
    ) -> Result<Self, AuthError> {
        let pubkey =
            verify_request_signature(pubkey_hex, signature_hex, timestamp, nonce, method, path, body)?;
        Ok(ApiAuthenticatedUser { pubkey })
    }
}

impl<'a> poem_openapi::ApiExtractor<'a> for ApiAuthenticatedUser {
    const TYPES: &'static [poem_openapi::ApiExtractorType] =
        &[poem_openapi::ApiExtractorType::RequestObject];
    const PARAM_IS_REQUIRED: bool = true;

    type ParamType = ();
    type ParamRawType = ();

    fn register(registry: &mut poem_openapi::registry::Registry) {
        // Register custom security scheme
        registry.create_security_scheme(
            "SignatureAuth",
            MetaSecurityScheme {
                ty: "apiKey",
                description: Some("Signature-based authentication using X-Public-Key, X-Signature, and X-Timestamp headers"),
                name: Some("X-Public-Key"),
                key_in: Some("header"),
                scheme: None,
                bearer_format: None,
                flows: None,
                openid_connect_url: None,
            },
        );
    }

    fn security_schemes() -> Vec<&'static str> {
        vec!["SignatureAuth"]
    }

    async fn from_request(
        request: &'a poem::Request,
        body: &mut poem::RequestBody,
        _param_opts: poem_openapi::ExtractParamOptions<Self::ParamType>,
    ) -> poem::Result<Self> {
        let headers = request.headers();

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

        let nonce = headers
            .get("X-Nonce")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AuthError::MissingHeader("X-Nonce".to_string()))?;

        // Read body
        let body_bytes = body.take()?.into_vec().await?;

        // Verify signature
        let pubkey = verify_request_signature(
            pubkey_hex,
            signature_hex,
            timestamp,
            nonce,
            request.method().as_str(),
            request.uri().path(),
            &body_bytes,
        )?;

        // Restore body for downstream handlers
        *body = poem::RequestBody::new(poem::Body::from(body_bytes));

        Ok(ApiAuthenticatedUser { pubkey })
    }
}

#[cfg(test)]
mod tests;
