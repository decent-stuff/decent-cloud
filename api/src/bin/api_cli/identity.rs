use anyhow::{Context, Result};
use dcc_common::DccIdentity;
use ed25519_dalek::SigningKey;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Identity storage for api-cli test keys
/// Stored at ~/.dc-test-keys/{name}.json
#[derive(Debug, Serialize, Deserialize)]
pub struct Identity {
    pub name: String,
    pub secret_key_hex: String,
    pub public_key_hex: String,
    pub created_at: String,
}

#[allow(dead_code)]
impl Identity {
    /// Get the directory where test keys are stored
    pub fn keys_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Failed to find home directory")?;
        Ok(home.join(".dc-test-keys"))
    }

    /// Get the path to a specific identity file
    pub fn path(name: &str) -> Result<PathBuf> {
        Ok(Self::keys_dir()?.join(format!("{}.json", name)))
    }

    /// Generate a new identity with random keys
    pub fn generate(name: &str) -> Result<Self> {
        // Generate random seed bytes
        let mut seed = [0u8; 32];
        rand::rng().fill_bytes(&mut seed);

        // Create signing key from seed
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();

        let identity = Identity {
            name: name.to_string(),
            secret_key_hex: hex::encode(signing_key.to_bytes()),
            public_key_hex: hex::encode(verifying_key.to_bytes()),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        identity.save()?;
        Ok(identity)
    }

    /// Import an identity from a secret key file (32-byte raw or PEM format)
    pub fn import(name: &str, key_path: &str) -> Result<Self> {
        let key_data = fs::read_to_string(key_path)
            .with_context(|| format!("Failed to read key file: {}", key_path))?;

        let signing_key = if key_data.contains("-----BEGIN") {
            // PEM format
            DccIdentity::signing_key_from_pem(&key_data)
                .map_err(|e| anyhow::anyhow!("Failed to parse PEM key: {}", e))?
        } else {
            // Assume hex-encoded raw key
            let key_bytes = hex::decode(key_data.trim()).context("Failed to decode hex key")?;
            if key_bytes.len() != 32 {
                anyhow::bail!("Secret key must be 32 bytes, got {}", key_bytes.len());
            }
            let key_array: [u8; 32] = key_bytes.try_into().unwrap();
            SigningKey::from_bytes(&key_array)
        };

        let verifying_key = signing_key.verifying_key();

        let identity = Identity {
            name: name.to_string(),
            secret_key_hex: hex::encode(signing_key.to_bytes()),
            public_key_hex: hex::encode(verifying_key.to_bytes()),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        identity.save()?;
        Ok(identity)
    }

    /// Save the identity to disk
    pub fn save(&self) -> Result<()> {
        let dir = Self::keys_dir()?;
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create keys directory: {}", dir.display()))?;

        let path = Self::path(&self.name)?;
        if path.exists() {
            anyhow::bail!(
                "Identity '{}' already exists at {}",
                self.name,
                path.display()
            );
        }

        let json = serde_json::to_string_pretty(self)?;
        fs::write(&path, json)
            .with_context(|| format!("Failed to write identity to: {}", path.display()))?;

        Ok(())
    }

    /// Load an identity from disk
    pub fn load(name: &str) -> Result<Self> {
        let path = Self::path(name)?;
        let json = fs::read_to_string(&path).with_context(|| {
            format!(
                "Failed to read identity '{}' from: {}",
                name,
                path.display()
            )
        })?;
        let identity: Identity = serde_json::from_str(&json)
            .with_context(|| format!("Failed to parse identity JSON: {}", path.display()))?;
        Ok(identity)
    }

    /// List all available identities
    pub fn list() -> Result<Vec<Self>> {
        let dir = Self::keys_dir()?;
        if !dir.exists() {
            return Ok(vec![]);
        }

        let mut identities = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(identity) = Self::load(name) {
                        identities.push(identity);
                    }
                }
            }
        }

        identities.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(identities)
    }

    /// Delete an identity
    pub fn delete(name: &str) -> Result<()> {
        let path = Self::path(name)?;
        fs::remove_file(&path).with_context(|| {
            format!(
                "Failed to delete identity '{}' at: {}",
                name,
                path.display()
            )
        })?;
        Ok(())
    }

    /// Get the signing key bytes
    pub fn secret_key_bytes(&self) -> Result<[u8; 32]> {
        let bytes = hex::decode(&self.secret_key_hex)?;
        bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid secret key length"))
    }

    /// Get the public key bytes
    pub fn public_key_bytes(&self) -> Result<Vec<u8>> {
        hex::decode(&self.public_key_hex).context("Invalid public key hex")
    }

    /// Convert to DccIdentity for signing
    pub fn to_dcc_identity(&self) -> Result<DccIdentity> {
        let secret_bytes = self.secret_key_bytes()?;
        DccIdentity::new_signing_from_bytes(&secret_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to create DccIdentity: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[allow(dead_code)]
    fn with_temp_keys_dir<F>(test: F)
    where
        F: FnOnce(&TempDir),
    {
        let temp_dir = TempDir::new().unwrap();
        // We can't easily override keys_dir() in tests, so we test the core logic
        test(&temp_dir);
    }

    #[test]
    fn test_identity_serialization() {
        let identity = Identity {
            name: "test".to_string(),
            secret_key_hex: "a".repeat(64),
            public_key_hex: "b".repeat(64),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&identity).unwrap();
        let parsed: Identity = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "test");
        assert_eq!(parsed.secret_key_hex.len(), 64);
    }

    #[test]
    fn test_secret_key_bytes_valid() {
        let identity = Identity {
            name: "test".to_string(),
            secret_key_hex: "00".repeat(32),
            public_key_hex: "00".repeat(32),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let bytes = identity.secret_key_bytes().unwrap();
        assert_eq!(bytes.len(), 32);
        assert!(bytes.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_secret_key_bytes_invalid_length() {
        let identity = Identity {
            name: "test".to_string(),
            secret_key_hex: "00".repeat(16), // 16 bytes, not 32
            public_key_hex: "00".repeat(32),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        assert!(identity.secret_key_bytes().is_err());
    }
}
