//! Shared request-signing primitives for the Decent Cloud API auth scheme.
//!
//! Single source of truth for the signed-message layout so the **sign** side
//! (api-cli, the user-facing `dc` CLI) and the **verify** side (api-server
//! `auth.rs`) can never drift. Duplicated layouts previously broke the user
//! CLI's provider commands entirely (wrong header names, millis-vs-nanos
//! timestamp, missing nonce, newline-joined message): the CLI signed one layout
//! and the server verified another.
//!
//! Message layout (no separators, raw byte concatenation):
//! `timestamp || nonce || method || path || body`

use crate::dcc_identity::{CryptoError, DccIdentity};

/// Header names for the signature auth scheme. Use these constants everywhere
/// instead of re-typing the string literals, so a typo cannot silently break
/// auth (as it did for the `dc` CLI's `X-DC-*` vs `X-*` mismatch).
pub const HEADER_PUBLIC_KEY: &str = "X-Public-Key";
pub const HEADER_SIGNATURE: &str = "X-Signature";
pub const HEADER_TIMESTAMP: &str = "X-Timestamp";
pub const HEADER_NONCE: &str = "X-Nonce";

/// Maximum clock skew the server tolerates (5 minutes, in nanoseconds).
/// Mirrors `api::auth::verify_request_signature`.
pub const MAX_TIMESTAMP_SKEW_NS: i64 = 5 * 60 * 1_000_000_000;

/// Construct the canonical signed-request message: byte concatenation of
/// `timestamp || nonce || method || path || body`.
///
/// Used by BOTH the signer (clients) and the verifier (server) so the layout is
/// defined exactly once. There are no separators or framing: each field is
/// appended raw, in this fixed order.
pub fn build_signed_message(
    timestamp: &str,
    nonce: &str,
    method: &str,
    path: &str,
    body: &[u8],
) -> Vec<u8> {
    let mut message = Vec::with_capacity(
        timestamp.len() + nonce.len() + method.len() + path.len() + body.len(),
    );
    message.extend_from_slice(timestamp.as_bytes());
    message.extend_from_slice(nonce.as_bytes());
    message.extend_from_slice(method.as_bytes());
    message.extend_from_slice(path.as_bytes());
    message.extend_from_slice(body);
    message
}

/// The values to send in the four auth headers for a signed request.
pub struct SignedRequest {
    /// UNIX timestamp in nanoseconds (string; the server parses as i64).
    pub timestamp: String,
    /// Fresh UUID v4 (replay-prevention nonce).
    pub nonce: String,
    /// Hex-encoded Ed25519ph signature (64 bytes) over the canonical message.
    pub signature_hex: String,
}

/// Produce the auth-header values for a request: a current-time nanoseconds
/// timestamp, a fresh UUID v4 nonce, and an Ed25519ph signature over the
/// canonical `build_signed_message` layout (signed with the `decent-cloud`
/// context, per `DccIdentity::sign`).
pub fn sign_request(
    identity: &DccIdentity,
    method: &str,
    path: &str,
    body: &[u8],
) -> Result<SignedRequest, CryptoError> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| CryptoError::Generic(format!("system clock before UNIX epoch: {e}")))?
        .as_nanos()
        .to_string();
    let nonce = uuid::Uuid::new_v4().to_string();
    let message = build_signed_message(&timestamp, &nonce, method, path, body);
    let signature = identity.sign(&message)?;
    Ok(SignedRequest {
        timestamp,
        nonce,
        signature_hex: hex::encode(signature.to_bytes()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_layout_is_raw_concatenation_in_fixed_order() {
        // No separators, fixed field order timestamp|nonce|method|path|body.
        let msg = build_signed_message("111", "N", "GET", "/p", b"body");
        assert_eq!(msg, b"111NGET/pbody".to_vec());
    }

    #[test]
    fn message_layout_handles_empty_body_and_fields() {
        let msg = build_signed_message("0", "n", "GET", "/", &[]);
        assert_eq!(msg, b"0nGET/".to_vec());
    }

    #[test]
    fn header_constants_match_the_server_expected_names() {
        // Guard against the exact drift that broke the dc CLI: it sent
        // X-DC-* headers that the server never reads.
        assert_eq!(HEADER_PUBLIC_KEY, "X-Public-Key");
        assert_eq!(HEADER_SIGNATURE, "X-Signature");
        assert_eq!(HEADER_TIMESTAMP, "X-Timestamp");
        assert_eq!(HEADER_NONCE, "X-Nonce");
    }
}
