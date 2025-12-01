use anyhow::Result;
use dcc_common::DccIdentity;
use poem::{error::ResponseError, http::StatusCode};
use poem_openapi::registry::MetaSecurityScheme;
use serde::{Deserialize, Serialize};
use std::fmt;
use ts_rs::TS;

/// Headers for signed API requests
#[allow(dead_code)]
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
/// - now_ns: optional override for current time in nanoseconds (e.g. for testing)
#[allow(clippy::too_many_arguments)]
pub fn verify_request_signature(
    pubkey_hex: &str,
    signature_hex: &str,
    timestamp_str: &str,
    nonce_str: &str,
    method: &str,
    path: &str,
    body: &[u8],
    now_ns: Option<i64>,
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
    let now = now_ns.unwrap_or_else(|| chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
    let max_age_ns = 5 * 60 * 1_000_000_000; // 5 minutes
    let age = now - timestamp;

    if age > max_age_ns || age < -max_age_ns {
        return Err(AuthError::TimestampExpired);
    }

    // Validate nonce format (UUID v4)
    uuid::Uuid::parse_str(nonce_str).map_err(|e| {
        AuthError::InvalidFormat(format!("Invalid nonce format (must be UUID v4): {}", e))
    })?;

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
        tracing::warn!(
            "Signature verification FAILED: {}, pubkey={}, sig={}, message_len={}",
            e,
            hex::encode(&pubkey),
            hex::encode(&signature),
            message.len()
        );
        AuthError::InvalidSignature(format!("Signature verification failed: {}", e))
    })?;

    Ok(pubkey)
}

/// poem-openapi compatible authenticated user
/// Uses custom headers for signature-based authentication
#[derive(Debug, Clone)]
pub struct ApiAuthenticatedUser {
    pub pubkey: Vec<u8>,
}

/// Admin authenticated user (signature-based with is_admin database flag)
#[derive(Debug, Clone)]
pub struct AdminAuthenticatedUser {
    pub pubkey: Vec<u8>,
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

        // Get the full path including /api/v1 prefix for signature verification
        // The request.uri().path() only returns the path within the nested service
        // but the client signs the full path including the prefix
        let full_path = format!("/api/v1{}", request.uri().path());

        // Verify signature
        let pubkey = verify_request_signature(
            pubkey_hex,
            signature_hex,
            timestamp,
            nonce,
            request.method().as_str(),
            &full_path,
            &body_bytes,
            None,
        )?;

        // Restore body for downstream handlers
        *body = poem::RequestBody::new(poem::Body::from(body_bytes));

        Ok(ApiAuthenticatedUser { pubkey })
    }
}

/// DEPRECATED: Get admin public keys from environment variable
/// This function is deprecated and no longer used for admin authentication.
/// Admin access is now controlled by the is_admin flag in the accounts table.
/// Kept for backward compatibility only - will be removed in a future version.
#[deprecated(
    since = "0.1.0",
    note = "Admin authentication now uses is_admin database flag instead of ADMIN_PUBLIC_KEYS env var"
)]
#[allow(dead_code)]
pub(crate) fn get_admin_pubkeys() -> Vec<Vec<u8>> {
    std::env::var("ADMIN_PUBLIC_KEYS")
        .ok()
        .and_then(|keys_str| {
            let keys: Result<Vec<Vec<u8>>, _> = keys_str
                .split(',')
                .map(|k| k.trim())
                .filter(|k| !k.is_empty())
                .map(hex::decode)
                .collect();
            keys.ok()
        })
        .unwrap_or_default()
}

impl<'a> poem_openapi::ApiExtractor<'a> for AdminAuthenticatedUser {
    const TYPES: &'static [poem_openapi::ApiExtractorType] =
        &[poem_openapi::ApiExtractorType::RequestObject];
    const PARAM_IS_REQUIRED: bool = true;

    type ParamType = ();
    type ParamRawType = ();

    fn register(registry: &mut poem_openapi::registry::Registry) {
        registry.create_security_scheme(
            "AdminSignatureAuth",
            MetaSecurityScheme {
                ty: "apiKey",
                description: Some(
                    "Admin signature-based authentication using X-Public-Key, X-Signature, and X-Timestamp headers",
                ),
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
        vec!["AdminSignatureAuth"]
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

        // Get the full path including /api/v1 prefix for signature verification
        // The request.uri().path() only returns the path within the nested service
        // but the client signs the full path including the prefix
        let full_path = format!("/api/v1{}", request.uri().path());

        // Verify signature
        let pubkey = verify_request_signature(
            pubkey_hex,
            signature_hex,
            timestamp,
            nonce,
            request.method().as_str(),
            &full_path,
            &body_bytes,
            None,
        )?;

        // Get database from request data
        let db = request
            .data::<std::sync::Arc<crate::database::Database>>()
            .ok_or_else(|| {
                AuthError::InternalError("Database not available in request context".to_string())
            })?;

        // Look up account by public key and check is_admin flag
        let account_id = db
            .get_account_id_by_public_key(&pubkey)
            .await
            .map_err(|e| AuthError::InternalError(format!("Failed to query account: {}", e)))?
            .ok_or_else(|| {
                poem::Error::from_string(
                    format!(
                        "Admin access denied. Public key '{}' is not associated with any account",
                        hex::encode(&pubkey)
                    ),
                    StatusCode::FORBIDDEN,
                )
            })?;

        let account = db
            .get_account(&account_id)
            .await
            .map_err(|e| AuthError::InternalError(format!("Failed to fetch account: {}", e)))?;

        let account = account.ok_or_else(|| {
            AuthError::InternalError("Account not found after ID lookup".to_string())
        })?;

        // Check is_admin flag
        if account.is_admin != 1 {
            return Err(poem::Error::from_string(
                format!(
                    "Admin access denied. Account '{}' does not have admin privileges",
                    account.username
                ),
                StatusCode::FORBIDDEN,
            ));
        }

        // Restore body for downstream handlers
        *body = poem::RequestBody::new(poem::Body::from(body_bytes));

        Ok(AdminAuthenticatedUser { pubkey })
    }
}

#[cfg(test)]
mod tests;
