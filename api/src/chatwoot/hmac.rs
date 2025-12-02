use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Generate HMAC-SHA256 hash for Chatwoot identity validation.
/// Used to authenticate users in the Chatwoot widget.
pub fn generate_identity_hash(identifier: &str, hmac_secret: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(hmac_secret.as_bytes()).expect("HMAC accepts any key size");
    mac.update(identifier.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_identity_hash() {
        let identifier = "user123";
        let secret = "test_secret";
        let hash = generate_identity_hash(identifier, secret);

        // Verify it's a valid hex string of expected length (64 chars for SHA256)
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_identity_hash_deterministic() {
        let identifier = "user123";
        let secret = "test_secret";

        let hash1 = generate_identity_hash(identifier, secret);
        let hash2 = generate_identity_hash(identifier, secret);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_identity_hash_different_inputs() {
        let secret = "test_secret";

        let hash1 = generate_identity_hash("user1", secret);
        let hash2 = generate_identity_hash("user2", secret);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_identity_hash_different_secrets() {
        let identifier = "user123";

        let hash1 = generate_identity_hash(identifier, "secret1");
        let hash2 = generate_identity_hash(identifier, "secret2");

        assert_ne!(hash1, hash2);
    }
}
