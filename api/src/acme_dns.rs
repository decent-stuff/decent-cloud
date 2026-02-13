//! acme-dns credential generation for gateway TLS isolation.
//!
//! Generates per-provider credentials used by Caddy's acmedns plugin
//! to POST TXT record updates to our central API's `/api/v1/acme-dns/update` endpoint.
//! The central API then proxies the TXT records to Cloudflare.

use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Generate a new set of acme-dns credentials (username UUID + random password).
pub fn generate_credentials() -> (Uuid, String) {
    let username = Uuid::new_v4();

    let mut password_bytes = [0u8; 24];
    rand::rng().fill_bytes(&mut password_bytes);
    let password = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        password_bytes,
    );

    (username, password)
}

/// Hash a password with SHA-256 for storage.
pub fn hash_password(password: &str) -> String {
    let hash = Sha256::digest(password.as_bytes());
    hex::encode(hash)
}

/// Verify a password against a stored SHA-256 hash.
pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    hash_password(password) == stored_hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_credentials_unique() {
        let (u1, p1) = generate_credentials();
        let (u2, p2) = generate_credentials();
        assert_ne!(u1, u2);
        assert_ne!(p1, p2);
    }

    #[test]
    fn test_generate_credentials_password_length() {
        let (_, password) = generate_credentials();
        // 24 bytes base64url-encoded = 32 chars
        assert_eq!(password.len(), 32);
    }

    #[test]
    fn test_hash_and_verify_password() {
        let password = "test-password-123";
        let hash = hash_password(password);
        assert!(verify_password(password, &hash));
        assert!(!verify_password("wrong-password", &hash));
    }

    #[test]
    fn test_hash_is_hex_sha256() {
        let hash = hash_password("hello");
        // SHA-256 produces 32 bytes = 64 hex chars
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
