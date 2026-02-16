//! Server-side credential encryption using AES-256-GCM
//!
//! # Design
//!
//! Cloud account credentials (Hetzner tokens, Proxmox API tokens) need to be stored
//! securely and decrypted by the server during provisioning. This is different from
//! the client-side E2EE used for VM credentials.
//!
//! # Threat Model
//!
//! - Database compromise should not expose plaintext credentials
//! - Server memory compromise during provisioning is out of scope
//! - Key compromise requires re-encrypting all credentials
//!
//! # Algorithm
//!
//! - **AES-256-GCM**: Authenticated encryption with 256-bit key
//! - **12-byte nonce**: Standard for GCM mode
//! - **Base64 encoding**: For database storage
//!
//! # Key Management
//!
//! Key is provided via `CREDENTIAL_ENCRYPTION_KEY` environment variable (hex-encoded).
//! Key rotation requires re-encrypting all credentials in the database.

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand_core::RngCore;

pub const SERVER_CREDENTIAL_ENCRYPTION_VERSION: u8 = 1;

pub const ENV_CREDENTIAL_ENCRYPTION_KEY: &str = "CREDENTIAL_ENCRYPTION_KEY";

#[derive(Debug, Clone)]
pub struct ServerEncryptionKey([u8; 32]);

impl ServerEncryptionKey {
    pub fn from_env() -> Result<Self> {
        let key_hex = std::env::var(ENV_CREDENTIAL_ENCRYPTION_KEY).context(format!(
            "{} not set - server credential encryption will NOT work! Set {} to enable.",
            ENV_CREDENTIAL_ENCRYPTION_KEY, ENV_CREDENTIAL_ENCRYPTION_KEY
        ))?;

        let key_bytes = hex::decode(&key_hex).context(format!(
            "Invalid {}: must be 64 hex characters (32 bytes)",
            ENV_CREDENTIAL_ENCRYPTION_KEY
        ))?;

        if key_bytes.len() != 32 {
            anyhow::bail!(
                "Invalid {}: expected 32 bytes, got {}",
                ENV_CREDENTIAL_ENCRYPTION_KEY,
                key_bytes.len()
            );
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes);
        Ok(Self(key))
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

#[derive(Debug, Clone)]
pub struct EncryptedServerCredential {
    pub version: u8,
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

impl EncryptedServerCredential {
    pub fn to_base64(&self) -> String {
        let mut blob = Vec::with_capacity(1 + 12 + self.ciphertext.len());
        blob.push(self.version);
        blob.extend_from_slice(&self.nonce);
        blob.extend_from_slice(&self.ciphertext);
        BASE64.encode(&blob)
    }

    pub fn from_base64(s: &str) -> Result<Self> {
        let blob = BASE64
            .decode(s)
            .context("Invalid base64 encoding for encrypted credential")?;

        if blob.is_empty() {
            anyhow::bail!("Empty encrypted credential");
        }

        let version = blob[0];
        if version != SERVER_CREDENTIAL_ENCRYPTION_VERSION {
            anyhow::bail!(
                "Unsupported encryption version: {} (expected {})",
                version,
                SERVER_CREDENTIAL_ENCRYPTION_VERSION
            );
        }

        if blob.len() < 1 + 12 {
            anyhow::bail!("Encrypted credential too short (missing nonce)");
        }

        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&blob[1..13]);
        let ciphertext = blob[13..].to_vec();

        Ok(Self {
            version,
            nonce,
            ciphertext,
        })
    }
}

pub fn encrypt_server_credential(plaintext: &str, key: &ServerEncryptionKey) -> Result<String> {
    let cipher =
        Aes256Gcm::new_from_slice(&key.0).context("Failed to create AES-256-GCM cipher")?;

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from(nonce_bytes);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

    let encrypted = EncryptedServerCredential {
        version: SERVER_CREDENTIAL_ENCRYPTION_VERSION,
        nonce: nonce_bytes,
        ciphertext,
    };

    Ok(encrypted.to_base64())
}

pub fn decrypt_server_credential(
    encrypted_base64: &str,
    key: &ServerEncryptionKey,
) -> Result<String> {
    let encrypted = EncryptedServerCredential::from_base64(encrypted_base64)?;

    let cipher =
        Aes256Gcm::new_from_slice(&key.0).context("Failed to create AES-256-GCM cipher")?;

    let nonce = Nonce::from(encrypted.nonce);

    let plaintext = cipher
        .decrypt(&nonce, encrypted.ciphertext.as_ref())
        .map_err(|e| anyhow::anyhow!("Decryption failed: {} (wrong key or tampered data)", e))?;

    String::from_utf8(plaintext).context("Decrypted credential is not valid UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> ServerEncryptionKey {
        ServerEncryptionKey::from_bytes([42u8; 32])
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = test_key();
        let plaintext = "hcloud_my_secret_token_12345";

        let encrypted = encrypt_server_credential(plaintext, &key).expect("Encryption failed");
        let decrypted = decrypt_server_credential(&encrypted, &key).expect("Decryption failed");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_different_keys_cannot_decrypt() {
        let key1 = test_key();
        let key2 = ServerEncryptionKey::from_bytes([99u8; 32]);

        let encrypted = encrypt_server_credential("secret", &key1).expect("Encryption failed");
        let result = decrypt_server_credential(&encrypted, &key2);

        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let key = test_key();
        let encrypted = encrypt_server_credential("secret", &key).expect("Encryption failed");

        let mut encrypted_struct = EncryptedServerCredential::from_base64(&encrypted).unwrap();
        encrypted_struct.ciphertext[0] ^= 0xFF;
        let tampered = encrypted_struct.to_base64();

        let result = decrypt_server_credential(&tampered, &key);
        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_nonce_fails() {
        let key = test_key();
        let encrypted = encrypt_server_credential("secret", &key).expect("Encryption failed");

        let mut encrypted_struct = EncryptedServerCredential::from_base64(&encrypted).unwrap();
        encrypted_struct.nonce[0] ^= 0xFF;
        let tampered = encrypted_struct.to_base64();

        let result = decrypt_server_credential(&tampered, &key);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_plaintext() {
        let key = test_key();
        let encrypted = encrypt_server_credential("", &key).expect("Encryption failed");
        let decrypted = decrypt_server_credential(&encrypted, &key).expect("Decryption failed");

        assert_eq!(decrypted, "");
    }

    #[test]
    fn test_long_plaintext() {
        let key = test_key();
        let plaintext = "x".repeat(10000);

        let encrypted = encrypt_server_credential(&plaintext, &key).expect("Encryption failed");
        let decrypted = decrypt_server_credential(&encrypted, &key).expect("Decryption failed");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encryption_is_non_deterministic() {
        let key = test_key();
        let plaintext = "same_secret";

        let encrypted1 = encrypt_server_credential(plaintext, &key).expect("Encryption failed");
        let encrypted2 = encrypt_server_credential(plaintext, &key).expect("Encryption failed");

        assert_ne!(
            encrypted1, encrypted2,
            "Each encryption should use a different nonce"
        );
    }

    #[test]
    fn test_invalid_base64() {
        let key = test_key();
        let result = decrypt_server_credential("not valid base64!!!", &key);
        assert!(result.is_err());
    }

    #[test]
    fn test_too_short_blob() {
        let key = test_key();
        let short_blob = BASE64.encode([1u8; 5]);
        let result = decrypt_server_credential(&short_blob, &key);
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_version() {
        let key = test_key();
        let mut blob = vec![2u8];
        blob.extend_from_slice(&[0u8; 12]);
        blob.extend_from_slice(&[0u8; 16]);
        let wrong_version = BASE64.encode(&blob);

        let result = decrypt_server_credential(&wrong_version, &key);
        assert!(result.is_err());
    }
}
