//! Credential encryption using X25519 key exchange and XChaCha20Poly1305
//!
//! # Design
//!
//! VM credentials (like root passwords) need to be stored securely such that:
//! 1. Only the contract requester can decrypt them
//! 2. The server (or anyone with database access) cannot read them
//! 3. The credentials are authenticated (tamper-proof)
//!
//! # Algorithm
//!
//! 1. **Ed25519 → X25519 conversion**: Requester's Ed25519 public key is converted
//!    to an X25519 public key for key exchange.
//!
//! 2. **Ephemeral key pair**: Server generates an ephemeral X25519 key pair for
//!    each encryption operation.
//!
//! 3. **Key derivation**: Shared secret is derived from (ephemeral_secret, requester_x25519_pubkey)
//!    using X25519 Diffie-Hellman.
//!
//! 4. **Encryption**: XChaCha20Poly1305 with random nonce provides authenticated encryption.
//!
//! 5. **Output format**: `{version}:{ephemeral_pubkey}:{nonce}:{ciphertext}` (base64 encoded)
//!
//! # Security
//!
//! - Ephemeral keys ensure forward secrecy per-encryption
//! - XChaCha20Poly1305 provides 256-bit security with authentication
//! - 24-byte nonce (vs 12-byte) eliminates nonce-reuse concerns

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng, Payload},
    XChaCha20Poly1305, XNonce,
};
use curve25519_dalek::{edwards::CompressedEdwardsY, montgomery::MontgomeryPoint, scalar::Scalar};
use rand_core::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};

/// Current encryption version for future-proofing
pub const CREDENTIAL_ENCRYPTION_VERSION: u8 = 1;

/// Version with AAD (Additional Authenticated Data) binding
pub const CREDENTIAL_ENCRYPTION_VERSION_AAD: u8 = 2;

/// Encrypted credentials structure for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedCredentials {
    /// Encryption version for future algorithm upgrades
    pub version: u8,
    /// Ephemeral X25519 public key (32 bytes, base64)
    pub ephemeral_pubkey: String,
    /// Random nonce (24 bytes for XChaCha20, base64)
    pub nonce: String,
    /// Encrypted and authenticated ciphertext (base64)
    pub ciphertext: String,
    /// Additional Authenticated Data (base64, only for version 2+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aad: Option<String>,
}

impl EncryptedCredentials {
    /// Serialize to JSON string format
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Deserialize from JSON string format
    pub fn from_json(s: &str) -> Result<Self> {
        serde_json::from_str(s).context("Failed to parse encrypted credentials")
    }
}

/// Convert Ed25519 public key to X25519 public key
///
/// Ed25519 uses twisted Edwards curve, X25519 uses Montgomery curve.
/// This conversion is mathematically sound and one-way (cannot recover Ed25519 from X25519).
fn ed25519_pubkey_to_x25519(ed25519_pubkey: &[u8; 32]) -> Result<[u8; 32]> {
    // Parse the Ed25519 public key as a compressed Edwards point
    let compressed = CompressedEdwardsY(*ed25519_pubkey);
    let edwards_point = compressed
        .decompress()
        .context("Invalid Ed25519 public key: failed to decompress")?;

    // Convert to Montgomery form (X25519)
    let montgomery: MontgomeryPoint = edwards_point.to_montgomery();
    Ok(montgomery.0)
}

/// Convert Ed25519 secret key to X25519 secret key
///
/// Ed25519 secret keys are 64 bytes (seed + public key), but the actual
/// scalar is derived from the first 32 bytes via SHA-512 hash clamping.
fn ed25519_secret_to_x25519(ed25519_secret: &[u8]) -> Result<[u8; 32]> {
    // Ed25519 uses SHA-512 hash of seed for scalar derivation
    let seed = if ed25519_secret.len() == 64 {
        // Standard format: first 32 bytes are seed
        &ed25519_secret[..32]
    } else if ed25519_secret.len() == 32 {
        // Just the seed
        ed25519_secret
    } else {
        anyhow::bail!(
            "Invalid Ed25519 secret key length: {} (expected 32 or 64)",
            ed25519_secret.len()
        );
    };

    let mut hasher = Sha512::new();
    hasher.update(seed);
    let hash = hasher.finalize();

    // X25519 scalar clamping (same as Ed25519)
    let mut scalar_bytes = [0u8; 32];
    scalar_bytes.copy_from_slice(&hash[..32]);
    scalar_bytes[0] &= 248;
    scalar_bytes[31] &= 127;
    scalar_bytes[31] |= 64;

    Ok(scalar_bytes)
}

/// Perform X25519 Diffie-Hellman key exchange
fn x25519_dh(my_secret: &[u8; 32], their_pubkey: &[u8; 32]) -> [u8; 32] {
    let scalar = Scalar::from_bytes_mod_order(*my_secret);
    let point = MontgomeryPoint(*their_pubkey);
    let shared = scalar * point;
    shared.0
}

/// Encrypt credentials for a specific requester
///
/// # Arguments
/// * `credentials` - The plaintext credentials (e.g., root password)
/// * `requester_ed25519_pubkey` - The requester's Ed25519 public key (32 bytes)
///
/// # Returns
/// Encrypted credentials that can only be decrypted with the requester's private key
#[allow(dead_code)]
pub fn encrypt_credentials(
    credentials: &str,
    requester_ed25519_pubkey: &[u8],
) -> Result<EncryptedCredentials> {
    // Validate pubkey length
    if requester_ed25519_pubkey.len() != 32 {
        anyhow::bail!(
            "Invalid Ed25519 public key length: {} (expected 32)",
            requester_ed25519_pubkey.len()
        );
    }

    let mut pubkey_array = [0u8; 32];
    pubkey_array.copy_from_slice(requester_ed25519_pubkey);

    // Convert Ed25519 pubkey to X25519
    let x25519_pubkey = ed25519_pubkey_to_x25519(&pubkey_array)
        .context("Failed to convert Ed25519 public key to X25519")?;

    // Generate ephemeral X25519 key pair
    let mut ephemeral_secret = [0u8; 32];
    OsRng.fill_bytes(&mut ephemeral_secret);
    // Clamp the secret key
    ephemeral_secret[0] &= 248;
    ephemeral_secret[31] &= 127;
    ephemeral_secret[31] |= 64;

    // Compute ephemeral public key
    let ephemeral_scalar = Scalar::from_bytes_mod_order(ephemeral_secret);
    let basepoint = MontgomeryPoint([
        0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ]);
    let ephemeral_pubkey = (ephemeral_scalar * basepoint).0;

    // Perform X25519 key exchange
    let shared_secret = x25519_dh(&ephemeral_secret, &x25519_pubkey);

    // Derive encryption key from shared secret using SHA-512
    let mut hasher = Sha512::new();
    hasher.update(b"credential-encryption-v1");
    hasher.update(shared_secret);
    let key_material = hasher.finalize();
    let encryption_key: [u8; 32] = key_material[..32].try_into().unwrap();

    // Generate random nonce for XChaCha20Poly1305 (24 bytes)
    let mut nonce_bytes = [0u8; 24];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from(nonce_bytes);

    // Encrypt with XChaCha20Poly1305
    let cipher =
        XChaCha20Poly1305::new_from_slice(&encryption_key).context("Failed to create cipher")?;
    let ciphertext = cipher
        .encrypt(&nonce, credentials.as_bytes())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

    Ok(EncryptedCredentials {
        version: CREDENTIAL_ENCRYPTION_VERSION,
        ephemeral_pubkey: BASE64.encode(ephemeral_pubkey),
        nonce: BASE64.encode(nonce_bytes),
        ciphertext: BASE64.encode(ciphertext),
        aad: None,
    })
}

/// Decrypt credentials using the requester's Ed25519 private key
///
/// # Arguments
/// * `encrypted` - The encrypted credentials
/// * `ed25519_secret` - The requester's Ed25519 secret key (32 or 64 bytes)
///
/// # Returns
/// The decrypted plaintext credentials
pub fn decrypt_credentials(
    encrypted: &EncryptedCredentials,
    ed25519_secret: &[u8],
) -> Result<String> {
    if encrypted.version != CREDENTIAL_ENCRYPTION_VERSION {
        anyhow::bail!(
            "Unsupported encryption version: {} (expected {})",
            encrypted.version,
            CREDENTIAL_ENCRYPTION_VERSION
        );
    }

    // Decode base64 fields
    let ephemeral_pubkey: [u8; 32] = BASE64
        .decode(&encrypted.ephemeral_pubkey)
        .context("Invalid ephemeral pubkey base64")?
        .try_into()
        .map_err(|v: Vec<u8>| anyhow::anyhow!("Invalid ephemeral pubkey length: {}", v.len()))?;

    let nonce_bytes: [u8; 24] = BASE64
        .decode(&encrypted.nonce)
        .context("Invalid nonce base64")?
        .try_into()
        .map_err(|v: Vec<u8>| anyhow::anyhow!("Invalid nonce length: {}", v.len()))?;

    let ciphertext = BASE64
        .decode(&encrypted.ciphertext)
        .context("Invalid ciphertext base64")?;

    // Convert Ed25519 secret to X25519
    let x25519_secret = ed25519_secret_to_x25519(ed25519_secret)?;

    // Perform X25519 key exchange
    let shared_secret = x25519_dh(&x25519_secret, &ephemeral_pubkey);

    // Derive decryption key
    let mut hasher = Sha512::new();
    hasher.update(b"credential-encryption-v1");
    hasher.update(shared_secret);
    let key_material = hasher.finalize();
    let decryption_key: [u8; 32] = key_material[..32].try_into().unwrap();

    // Decrypt
    let cipher =
        XChaCha20Poly1305::new_from_slice(&decryption_key).context("Failed to create cipher")?;
    let nonce = XNonce::from(nonce_bytes);
    let plaintext = cipher
        .decrypt(&nonce, ciphertext.as_ref())
        .map_err(|e| anyhow::anyhow!("Decryption failed: {} (wrong key or tampered data)", e))?;

    String::from_utf8(plaintext).context("Decrypted credentials are not valid UTF-8")
}

/// Encrypt credentials for a specific requester with Additional Authenticated Data (AAD)
///
/// AAD binds the ciphertext to specific context (e.g., contract_id), preventing
/// credential replay attacks across different contracts.
///
/// # Arguments
/// * `credentials` - The plaintext credentials (e.g., root password)
/// * `requester_ed25519_pubkey` - The requester's Ed25519 public key (32 bytes)
/// * `aad` - Additional Authenticated Data (e.g., contract_id) - will be authenticated but not encrypted
///
/// # Returns
/// Encrypted credentials that can only be decrypted with the requester's private key
/// and the same AAD value.
pub fn encrypt_credentials_with_aad(
    credentials: &str,
    requester_ed25519_pubkey: &[u8],
    aad: &[u8],
) -> Result<EncryptedCredentials> {
    if requester_ed25519_pubkey.len() != 32 {
        anyhow::bail!(
            "Invalid Ed25519 public key length: {} (expected 32)",
            requester_ed25519_pubkey.len()
        );
    }

    let mut pubkey_array = [0u8; 32];
    pubkey_array.copy_from_slice(requester_ed25519_pubkey);

    let x25519_pubkey = ed25519_pubkey_to_x25519(&pubkey_array)
        .context("Failed to convert Ed25519 public key to X25519")?;

    let mut ephemeral_secret = [0u8; 32];
    OsRng.fill_bytes(&mut ephemeral_secret);
    ephemeral_secret[0] &= 248;
    ephemeral_secret[31] &= 127;
    ephemeral_secret[31] |= 64;

    let ephemeral_scalar = Scalar::from_bytes_mod_order(ephemeral_secret);
    let basepoint = MontgomeryPoint([
        0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ]);
    let ephemeral_pubkey = (ephemeral_scalar * basepoint).0;

    let shared_secret = x25519_dh(&ephemeral_secret, &x25519_pubkey);

    let mut hasher = Sha512::new();
    hasher.update(b"credential-encryption-v2-aad");
    hasher.update(shared_secret);
    let key_material = hasher.finalize();
    let encryption_key: [u8; 32] = key_material[..32].try_into().unwrap();

    let mut nonce_bytes = [0u8; 24];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from(nonce_bytes);

    let cipher =
        XChaCha20Poly1305::new_from_slice(&encryption_key).context("Failed to create cipher")?;

    let payload = Payload {
        msg: credentials.as_bytes(),
        aad,
    };
    let ciphertext = cipher
        .encrypt(&nonce, payload)
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

    Ok(EncryptedCredentials {
        version: CREDENTIAL_ENCRYPTION_VERSION_AAD,
        ephemeral_pubkey: BASE64.encode(ephemeral_pubkey),
        nonce: BASE64.encode(nonce_bytes),
        ciphertext: BASE64.encode(ciphertext),
        aad: Some(BASE64.encode(aad)),
    })
}

/// Decrypt credentials with AAD using the requester's Ed25519 private key
///
/// # Arguments
/// * `encrypted` - The encrypted credentials (must be version 2 with AAD)
/// * `ed25519_secret` - The requester's Ed25519 secret key (32 or 64 bytes)
/// * `aad` - The same AAD value used during encryption
///
/// # Returns
/// The decrypted plaintext credentials
pub fn decrypt_credentials_with_aad(
    encrypted: &EncryptedCredentials,
    ed25519_secret: &[u8],
    aad: &[u8],
) -> Result<String> {
    if encrypted.version != CREDENTIAL_ENCRYPTION_VERSION_AAD {
        anyhow::bail!(
            "Unsupported encryption version: {} (expected {} for AAD)",
            encrypted.version,
            CREDENTIAL_ENCRYPTION_VERSION_AAD
        );
    }

    let ephemeral_pubkey: [u8; 32] = BASE64
        .decode(&encrypted.ephemeral_pubkey)
        .context("Invalid ephemeral pubkey base64")?
        .try_into()
        .map_err(|v: Vec<u8>| anyhow::anyhow!("Invalid ephemeral pubkey length: {}", v.len()))?;

    let nonce_bytes: [u8; 24] = BASE64
        .decode(&encrypted.nonce)
        .context("Invalid nonce base64")?
        .try_into()
        .map_err(|v: Vec<u8>| anyhow::anyhow!("Invalid nonce length: {}", v.len()))?;

    let ciphertext = BASE64
        .decode(&encrypted.ciphertext)
        .context("Invalid ciphertext base64")?;

    let x25519_secret = ed25519_secret_to_x25519(ed25519_secret)?;

    let shared_secret = x25519_dh(&x25519_secret, &ephemeral_pubkey);

    let mut hasher = Sha512::new();
    hasher.update(b"credential-encryption-v2-aad");
    hasher.update(shared_secret);
    let key_material = hasher.finalize();
    let decryption_key: [u8; 32] = key_material[..32].try_into().unwrap();

    let cipher =
        XChaCha20Poly1305::new_from_slice(&decryption_key).context("Failed to create cipher")?;
    let nonce = XNonce::from(nonce_bytes);

    let payload = Payload {
        msg: ciphertext.as_ref(),
        aad,
    };
    let plaintext = cipher.decrypt(&nonce, payload).map_err(|e| {
        anyhow::anyhow!(
            "Decryption failed: {} (wrong key, wrong AAD, or tampered data)",
            e
        )
    })?;

    String::from_utf8(plaintext).context("Decrypted credentials are not valid UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{SigningKey, VerifyingKey};

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        // Generate Ed25519 key pair
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key: VerifyingKey = (&signing_key).into();

        let pubkey = verifying_key.as_bytes();
        let secret = signing_key.to_bytes();

        // Encrypt credentials
        let credentials = "super_secret_root_password_123!";
        let encrypted = encrypt_credentials(credentials, pubkey).expect("Encryption failed");

        // Verify structure
        assert_eq!(encrypted.version, CREDENTIAL_ENCRYPTION_VERSION);
        assert!(!encrypted.ephemeral_pubkey.is_empty());
        assert!(!encrypted.nonce.is_empty());
        assert!(!encrypted.ciphertext.is_empty());

        // Decrypt credentials
        let decrypted = decrypt_credentials(&encrypted, &secret).expect("Decryption failed");
        assert_eq!(decrypted, credentials);
    }

    #[test]
    fn test_different_keys_cannot_decrypt() {
        // Generate two different key pairs
        let signing_key1 = SigningKey::generate(&mut OsRng);
        let verifying_key1: VerifyingKey = (&signing_key1).into();

        let signing_key2 = SigningKey::generate(&mut OsRng);

        // Encrypt with key1's pubkey
        let credentials = "secret_password";
        let encrypted =
            encrypt_credentials(credentials, verifying_key1.as_bytes()).expect("Encryption failed");

        // Try to decrypt with key2's secret - should fail
        let result = decrypt_credentials(&encrypted, &signing_key2.to_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_deserialize() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key: VerifyingKey = (&signing_key).into();

        let credentials = "test_password";
        let encrypted =
            encrypt_credentials(credentials, verifying_key.as_bytes()).expect("Encryption failed");

        // Serialize to JSON
        let json = encrypted.to_json();
        assert!(!json.is_empty());

        // Deserialize back
        let restored = EncryptedCredentials::from_json(&json).expect("Deserialization failed");
        assert_eq!(restored.version, encrypted.version);
        assert_eq!(restored.ephemeral_pubkey, encrypted.ephemeral_pubkey);
        assert_eq!(restored.nonce, encrypted.nonce);
        assert_eq!(restored.ciphertext, encrypted.ciphertext);

        // Decrypt restored
        let decrypted =
            decrypt_credentials(&restored, &signing_key.to_bytes()).expect("Decryption failed");
        assert_eq!(decrypted, credentials);
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key: VerifyingKey = (&signing_key).into();

        let credentials = "secret";
        let mut encrypted =
            encrypt_credentials(credentials, verifying_key.as_bytes()).expect("Encryption failed");

        // Tamper with ciphertext
        let mut ct_bytes = BASE64.decode(&encrypted.ciphertext).unwrap();
        ct_bytes[0] ^= 0xFF;
        encrypted.ciphertext = BASE64.encode(&ct_bytes);

        // Decryption should fail due to authentication
        let result = decrypt_credentials(&encrypted, &signing_key.to_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_pubkey_length() {
        let result = encrypt_credentials("test", &[0u8; 31]); // Wrong length
        assert!(result.is_err());
    }

    #[test]
    fn test_ed25519_to_x25519_conversion() {
        // Test that the conversion produces valid X25519 keys
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key: VerifyingKey = (&signing_key).into();

        let x25519_pubkey =
            ed25519_pubkey_to_x25519(verifying_key.as_bytes()).expect("Conversion failed");

        // X25519 public keys are 32 bytes
        assert_eq!(x25519_pubkey.len(), 32);
    }

    #[test]
    fn test_aad_encrypt_decrypt_roundtrip() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key: VerifyingKey = (&signing_key).into();

        let credentials = "root_password_123";
        let contract_id = b"contract-abc123";

        let encrypted =
            encrypt_credentials_with_aad(credentials, verifying_key.as_bytes(), contract_id)
                .expect("Encryption with AAD failed");

        let decrypted =
            decrypt_credentials_with_aad(&encrypted, &signing_key.to_bytes(), contract_id)
                .expect("Decryption with AAD failed");

        assert_eq!(decrypted, credentials);
    }

    #[test]
    fn test_aad_wrong_aad_fails() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key: VerifyingKey = (&signing_key).into();

        let credentials = "root_password_123";
        let contract_id = b"contract-abc123";
        let wrong_contract_id = b"contract-xyz789";

        let encrypted =
            encrypt_credentials_with_aad(credentials, verifying_key.as_bytes(), contract_id)
                .expect("Encryption with AAD failed");

        let result =
            decrypt_credentials_with_aad(&encrypted, &signing_key.to_bytes(), wrong_contract_id);
        assert!(result.is_err(), "Decryption with wrong AAD should fail");
    }

    #[test]
    fn test_aad_missing_on_decrypt_fails() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key: VerifyingKey = (&signing_key).into();

        let credentials = "root_password_123";
        let contract_id = b"contract-abc123";

        let encrypted =
            encrypt_credentials_with_aad(credentials, verifying_key.as_bytes(), contract_id)
                .expect("Encryption with AAD failed");

        let result = decrypt_credentials_with_aad(&encrypted, &signing_key.to_bytes(), b"");
        assert!(
            result.is_err(),
            "Decryption with empty AAD should fail when encrypted with AAD"
        );
    }
}
