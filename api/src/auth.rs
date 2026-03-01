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
    let now = match now_ns {
        Some(ns) => ns,
        None => crate::now_ns().map_err(|e| AuthError::InternalError(e.to_string()))?,
    };
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
    pub account_id: Vec<u8>,
}

/// Agent authenticated user (signature-based with delegated permissions)
/// Used by provisioning agents that have been delegated authority by a provider
#[derive(Debug, Clone)]
pub struct AgentAuthenticatedUser {
    /// The agent's public key (used for signing and per-agent status keying)
    pub agent_pubkey: Vec<u8>,
    /// The provider's public key (the delegator)
    pub provider_pubkey: Vec<u8>,
    /// Permissions granted to this agent
    pub permissions: Vec<crate::database::AgentPermission>,
    /// The pool this agent belongs to, if any
    pub pool_id: Option<String>,
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
        if !account.is_admin {
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

        Ok(AdminAuthenticatedUser { pubkey, account_id })
    }
}

impl<'a> poem_openapi::ApiExtractor<'a> for AgentAuthenticatedUser {
    const TYPES: &'static [poem_openapi::ApiExtractorType] =
        &[poem_openapi::ApiExtractorType::RequestObject];
    const PARAM_IS_REQUIRED: bool = true;

    type ParamType = ();
    type ParamRawType = ();

    fn register(registry: &mut poem_openapi::registry::Registry) {
        registry.create_security_scheme(
            "AgentSignatureAuth",
            MetaSecurityScheme {
                ty: "apiKey",
                description: Some(
                    "Agent signature-based authentication using X-Agent-Pubkey, X-Signature, and X-Timestamp headers. \
                     Requires a valid delegation from a provider.",
                ),
                name: Some("X-Agent-Pubkey"),
                key_in: Some("header"),
                scheme: None,
                bearer_format: None,
                flows: None,
                openid_connect_url: None,
            },
        );
    }

    fn security_schemes() -> Vec<&'static str> {
        vec!["AgentSignatureAuth"]
    }

    async fn from_request(
        request: &'a poem::Request,
        body: &mut poem::RequestBody,
        _param_opts: poem_openapi::ExtractParamOptions<Self::ParamType>,
    ) -> poem::Result<Self> {
        let headers = request.headers();

        // Agent uses X-Agent-Pubkey instead of X-Public-Key
        let agent_pubkey_hex = headers
            .get("X-Agent-Pubkey")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AuthError::MissingHeader("X-Agent-Pubkey".to_string()))?;

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
        let full_path = format!("/api/v1{}", request.uri().path());

        // Verify signature using agent's pubkey
        let agent_pubkey = verify_request_signature(
            agent_pubkey_hex,
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

        // Look up delegation for this agent pubkey
        let delegation = db
            .get_active_delegation(&agent_pubkey)
            .await
            .map_err(|e| AuthError::InternalError(format!("Failed to query delegation: {}", e)))?
            .ok_or_else(|| {
                AuthError::InvalidSignature(format!(
                    "No active delegation found for agent key '{}'",
                    hex::encode(&agent_pubkey)
                ))
            })?;

        let (provider_pubkey, permissions, _signature, pool_id) = delegation;

        // Restore body for downstream handlers
        *body = poem::RequestBody::new(poem::Body::from(body_bytes));

        Ok(AgentAuthenticatedUser {
            agent_pubkey,
            provider_pubkey,
            permissions,
            pool_id,
        })
    }
}

impl AgentAuthenticatedUser {
    /// Check if this agent has a specific permission
    pub fn has_permission(&self, permission: crate::database::AgentPermission) -> bool {
        self.permissions.contains(&permission)
    }

    /// Require a specific permission, returning an error if not present
    pub fn require_permission(
        &self,
        permission: crate::database::AgentPermission,
    ) -> Result<(), poem::Error> {
        if self.has_permission(permission) {
            Ok(())
        } else {
            Err(poem::Error::from_string(
                format!("Agent does not have '{}' permission", permission.as_str()),
                StatusCode::FORBIDDEN,
            ))
        }
    }
}

/// Optional API authentication - returns authenticated pubkey if headers present, None otherwise.
/// Used for endpoints that work differently based on whether the user is authenticated (e.g. visibility filtering).
#[derive(Debug, Clone)]
pub struct OptionalApiAuth {
    pub pubkey: Option<Vec<u8>>,
}

impl<'a> poem_openapi::ApiExtractor<'a> for OptionalApiAuth {
    const TYPES: &'static [poem_openapi::ApiExtractorType] =
        &[poem_openapi::ApiExtractorType::RequestObject];
    const PARAM_IS_REQUIRED: bool = false; // Optional - endpoint works without auth

    type ParamType = ();
    type ParamRawType = ();

    fn register(registry: &mut poem_openapi::registry::Registry) {
        registry.create_security_scheme(
            "OptionalSignatureAuth",
            MetaSecurityScheme {
                ty: "apiKey",
                description: Some(
                    "Optional signature-based authentication. If headers are provided, signature is verified. \
                     Otherwise, the request proceeds as unauthenticated.",
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
        vec!["OptionalSignatureAuth"]
    }

    async fn from_request(
        request: &'a poem::Request,
        body: &mut poem::RequestBody,
        _param_opts: poem_openapi::ExtractParamOptions<Self::ParamType>,
    ) -> poem::Result<Self> {
        let headers = request.headers();

        // Check if auth headers are present
        let pubkey_hex = headers.get("X-Public-Key").and_then(|v| v.to_str().ok());
        let signature_hex = headers.get("X-Signature").and_then(|v| v.to_str().ok());
        let timestamp = headers.get("X-Timestamp").and_then(|v| v.to_str().ok());
        let nonce = headers.get("X-Nonce").and_then(|v| v.to_str().ok());

        // If no auth headers, return None (unauthenticated access)
        let (pubkey_hex, signature_hex, timestamp, nonce) =
            match (pubkey_hex, signature_hex, timestamp, nonce) {
                (Some(pk), Some(sig), Some(ts), Some(n)) => (pk, sig, ts, n),
                _ => return Ok(OptionalApiAuth { pubkey: None }),
            };

        // Auth headers present - verify signature
        let body_bytes = body.take()?.into_vec().await?;
        let full_path = format!("/api/v1{}", request.uri().path());

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

        Ok(OptionalApiAuth {
            pubkey: Some(pubkey),
        })
    }
}

/// Authentication that accepts either a provider (via X-Public-Key) or an agent (via X-Agent-Pubkey)
/// Used for endpoints that can be called by either the provider directly or their delegated agent.
#[derive(Debug, Clone)]
pub struct ProviderOrAgentAuth {
    /// The provider's public key (always present - either directly or via delegation)
    pub provider_pubkey: Vec<u8>,
    /// If authenticated via agent, contains the agent's pubkey; None if provider authenticated directly
    /// Kept for audit logging purposes.
    #[allow(dead_code)]
    pub agent_pubkey: Option<Vec<u8>>,
}

impl<'a> poem_openapi::ApiExtractor<'a> for ProviderOrAgentAuth {
    const TYPES: &'static [poem_openapi::ApiExtractorType] =
        &[poem_openapi::ApiExtractorType::RequestObject];
    const PARAM_IS_REQUIRED: bool = true;

    type ParamType = ();
    type ParamRawType = ();

    fn register(registry: &mut poem_openapi::registry::Registry) {
        registry.create_security_scheme(
            "ProviderOrAgentAuth",
            MetaSecurityScheme {
                ty: "apiKey",
                description: Some(
                    "Accepts either provider signature (X-Public-Key) or agent signature (X-Agent-Pubkey). \
                     For agent auth, requires a valid delegation from the provider.",
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
        vec!["ProviderOrAgentAuth"]
    }

    async fn from_request(
        request: &'a poem::Request,
        body: &mut poem::RequestBody,
        _param_opts: poem_openapi::ExtractParamOptions<Self::ParamType>,
    ) -> poem::Result<Self> {
        let headers = request.headers();

        // Check which auth method is being used
        let agent_pubkey_hex = headers.get("X-Agent-Pubkey").and_then(|v| v.to_str().ok());
        let user_pubkey_hex = headers.get("X-Public-Key").and_then(|v| v.to_str().ok());

        let (pubkey_hex, is_agent) = match (agent_pubkey_hex, user_pubkey_hex) {
            (Some(agent), _) => (agent, true), // Prefer agent auth if both present
            (None, Some(user)) => (user, false),
            (None, None) => {
                return Err(
                    AuthError::MissingHeader("X-Public-Key or X-Agent-Pubkey".to_string()).into(),
                )
            }
        };

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

        if is_agent {
            // Agent auth - look up delegation
            let db = request
                .data::<std::sync::Arc<crate::database::Database>>()
                .ok_or_else(|| {
                    AuthError::InternalError(
                        "Database not available in request context".to_string(),
                    )
                })?;

            let delegation = db
                .get_active_delegation(&pubkey)
                .await
                .map_err(|e| {
                    AuthError::InternalError(format!("Failed to query delegation: {}", e))
                })?
                .ok_or_else(|| {
                    AuthError::InvalidSignature(format!(
                        "No active delegation found for agent key '{}'",
                        hex::encode(&pubkey)
                    ))
                })?;

            let (provider_pubkey, _permissions, _signature, _pool_id) = delegation;

            Ok(ProviderOrAgentAuth {
                provider_pubkey,
                agent_pubkey: Some(pubkey),
            })
        } else {
            // Direct provider auth
            Ok(ProviderOrAgentAuth {
                provider_pubkey: pubkey,
                agent_pubkey: None,
            })
        }
    }
}

/// API token bearer authentication.
/// Accepts `Authorization: Bearer <hex-token>` header.
/// Looks up the token by SHA-256 hash, updates last_used_at, and returns the owning user's pubkey.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Constructed by poem framework as an API extractor, not directly in user code
pub struct BearerAuth {
    pub pubkey: Vec<u8>,
}

impl<'a> poem_openapi::ApiExtractor<'a> for BearerAuth {
    const TYPES: &'static [poem_openapi::ApiExtractorType] =
        &[poem_openapi::ApiExtractorType::RequestObject];
    const PARAM_IS_REQUIRED: bool = true;

    type ParamType = ();
    type ParamRawType = ();

    fn register(registry: &mut poem_openapi::registry::Registry) {
        registry.create_security_scheme(
            "BearerAuth",
            MetaSecurityScheme {
                ty: "http",
                description: Some("API token authentication via Authorization: Bearer <token>"),
                name: None,
                key_in: None,
                scheme: Some("bearer"),
                bearer_format: Some("hex"),
                flows: None,
                openid_connect_url: None,
            },
        );
    }

    fn security_schemes() -> Vec<&'static str> {
        vec!["BearerAuth"]
    }

    async fn from_request(
        request: &'a poem::Request,
        _body: &mut poem::RequestBody,
        _param_opts: poem_openapi::ExtractParamOptions<Self::ParamType>,
    ) -> poem::Result<Self> {
        use crate::database::api_tokens::hash_token_hex;

        let auth_header = request
            .headers()
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AuthError::MissingHeader("Authorization".to_string()))?;

        let token_hex = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            AuthError::InvalidFormat("Authorization header must be 'Bearer <token>'".to_string())
        })?;

        let token_hash = hash_token_hex(token_hex)
            .map_err(|e| AuthError::InvalidFormat(format!("Invalid token: {}", e)))?;

        let db = request
            .data::<std::sync::Arc<crate::database::Database>>()
            .ok_or_else(|| {
                AuthError::InternalError("Database not available in request context".to_string())
            })?;

        let pubkey = db
            .lookup_api_token_pubkey(&token_hash)
            .await
            .map_err(|e| AuthError::InternalError(format!("Token lookup failed: {}", e)))?
            .ok_or_else(|| {
                AuthError::InvalidSignature("Invalid or expired API token".to_string())
            })?;

        Ok(BearerAuth { pubkey })
    }
}

/// Authenticate a provider OR agent from a plain poem request (for use in non-OpenAPI handlers like SSE).
///
/// Accepts either:
/// - Provider auth: X-Public-Key header or pubkey query param
/// - Agent auth: X-Agent-Pubkey header or agent_pubkey query param
///
/// For agent auth, looks up the delegation to resolve the provider pubkey.
/// Supports query params for EventSource (which doesn't support custom headers).
/// Returns the provider's pubkey bytes (either directly or via delegation).
pub async fn authenticate_provider_or_agent_from_request(
    request: &poem::Request,
    db: &std::sync::Arc<crate::database::Database>,
) -> Result<Vec<u8>, AuthError> {
    let headers = request.headers();
    let query = request.uri().query().unwrap_or("");

    let agent_pubkey_hex = headers
        .get("X-Agent-Pubkey")
        .and_then(|v| v.to_str().ok())
        .or_else(|| get_query_param(query, "agent_pubkey"));

    let user_pubkey_hex = headers
        .get("X-Public-Key")
        .and_then(|v| v.to_str().ok())
        .or_else(|| get_query_param(query, "pubkey"));

    let (pubkey_hex, is_agent) = match (agent_pubkey_hex, user_pubkey_hex) {
        (Some(agent), _) => (agent, true),
        (None, Some(user)) => (user, false),
        (None, None) => {
            return Err(AuthError::MissingHeader(
                "X-Public-Key/X-Agent-Pubkey or pubkey/agent_pubkey query param".to_string(),
            ))
        }
    };

    let signature_hex = headers
        .get("X-Signature")
        .and_then(|v| v.to_str().ok())
        .or_else(|| get_query_param(query, "signature"))
        .ok_or_else(|| {
            AuthError::MissingHeader("X-Signature or signature query param".to_string())
        })?;

    let timestamp = headers
        .get("X-Timestamp")
        .and_then(|v| v.to_str().ok())
        .or_else(|| get_query_param(query, "timestamp"))
        .ok_or_else(|| {
            AuthError::MissingHeader("X-Timestamp or timestamp query param".to_string())
        })?;

    let nonce = headers
        .get("X-Nonce")
        .and_then(|v| v.to_str().ok())
        .or_else(|| get_query_param(query, "nonce"))
        .ok_or_else(|| AuthError::MissingHeader("X-Nonce or nonce query param".to_string()))?;

    let full_path = format!("/api/v1{}", request.uri().path());

    let pubkey = verify_request_signature(
        pubkey_hex,
        signature_hex,
        timestamp,
        nonce,
        request.method().as_str(),
        &full_path,
        &[],
        None,
    )?;

    if is_agent {
        let delegation = db
            .get_active_delegation(&pubkey)
            .await
            .map_err(|e| AuthError::InternalError(format!("Failed to query delegation: {}", e)))?
            .ok_or_else(|| {
                AuthError::InvalidSignature(format!(
                    "No active delegation found for agent key '{}'",
                    hex::encode(&pubkey)
                ))
            })?;

        let (provider_pubkey, _, _, _) = delegation;
        Ok(provider_pubkey)
    } else {
        Ok(pubkey)
    }
}

/// Authenticate a user from a plain poem request (for use in non-OpenAPI handlers like SSE).
///
/// Reads X-Public-Key, X-Signature, X-Timestamp, X-Nonce from headers first.
/// If headers are missing, falls back to query parameters (pubkey, signature, timestamp, nonce).
/// This allows EventSource (which doesn't support custom headers) to authenticate via URL params.
/// Returns the authenticated user's pubkey bytes.
pub fn authenticate_user_from_request(request: &poem::Request) -> Result<Vec<u8>, AuthError> {
    let headers = request.headers();
    let query = request.uri().query().unwrap_or("");

    let pubkey_hex = headers
        .get("X-Public-Key")
        .and_then(|v| v.to_str().ok())
        .or_else(|| get_query_param(query, "pubkey"))
        .ok_or_else(|| {
            AuthError::MissingHeader("X-Public-Key or pubkey query param".to_string())
        })?;

    let signature_hex = headers
        .get("X-Signature")
        .and_then(|v| v.to_str().ok())
        .or_else(|| get_query_param(query, "signature"))
        .ok_or_else(|| {
            AuthError::MissingHeader("X-Signature or signature query param".to_string())
        })?;

    let timestamp = headers
        .get("X-Timestamp")
        .and_then(|v| v.to_str().ok())
        .or_else(|| get_query_param(query, "timestamp"))
        .ok_or_else(|| {
            AuthError::MissingHeader("X-Timestamp or timestamp query param".to_string())
        })?;

    let nonce = headers
        .get("X-Nonce")
        .and_then(|v| v.to_str().ok())
        .or_else(|| get_query_param(query, "nonce"))
        .ok_or_else(|| AuthError::MissingHeader("X-Nonce or nonce query param".to_string()))?;

    let full_path = format!("/api/v1{}", request.uri().path());

    verify_request_signature(
        pubkey_hex,
        signature_hex,
        timestamp,
        nonce,
        request.method().as_str(),
        &full_path,
        &[], // SSE GET has no body
        None,
    )
}

fn get_query_param<'a>(query: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{}=", name);
    for pair in query.split('&') {
        if let Some(value) = pair.strip_prefix(&prefix) {
            return Some(value);
        }
    }
    None
}

#[cfg(test)]
mod tests;
