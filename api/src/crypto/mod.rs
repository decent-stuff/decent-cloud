//! Cryptographic utilities for credential encryption
//!
//! This module provides two encryption schemes:
//!
//! 1. **Client-side E2EE** (credential_encryption): Ed25519→X25519 + XChaCha20Poly1305
//!    - For VM credentials that only the requester can decrypt
//!    - Server cannot read these credentials
//!
//! 2. **Server-side encryption** (server_credential_encryption): AES-256-GCM
//!    - For cloud account tokens (Hetzner, Proxmox) that the server needs during provisioning
//!    - Key from CREDENTIAL_ENCRYPTION_KEY env var

mod credential_encryption;
mod server_credential_encryption;

// Re-export public functions for client-side credential encryption
pub use credential_encryption::encrypt_credentials_with_aad;

// These are used by frontend decryption (documented in API)
#[allow(unused_imports)]
pub use credential_encryption::{
    decrypt_credentials, decrypt_credentials_with_aad, EncryptedCredentials,
    CREDENTIAL_ENCRYPTION_VERSION, CREDENTIAL_ENCRYPTION_VERSION_AAD,
};

// Re-export public functions for server-side credential encryption
pub use server_credential_encryption::{
    decrypt_server_credential, encrypt_server_credential, EncryptedServerCredential,
    ServerEncryptionKey, ENV_CREDENTIAL_ENCRYPTION_KEY, SERVER_CREDENTIAL_ENCRYPTION_VERSION,
};
