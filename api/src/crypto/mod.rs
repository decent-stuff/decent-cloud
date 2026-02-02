//! Cryptographic utilities for credential encryption
//!
//! This module provides Ed25519→X25519 key conversion and authenticated encryption
//! for securely storing VM credentials that can only be decrypted by the requester.

mod credential_encryption;

// Re-export public functions for credential encryption
pub use credential_encryption::encrypt_credentials;

// These are used by frontend decryption (documented in API)
#[allow(unused_imports)]
pub use credential_encryption::{
    decrypt_credentials, EncryptedCredentials, CREDENTIAL_ENCRYPTION_VERSION,
};
