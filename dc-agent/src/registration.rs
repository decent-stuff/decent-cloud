//! Agent registration and delegation management.
//!
//! Handles generating agent keypairs. Registration is done via setup tokens.

use anyhow::{Context, Result};
use dcc_common::DccIdentity;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::path::{Path, PathBuf};

/// Get the default agent keys directory (~/.dc-agent).
///
/// # Errors
/// Returns an error if the home directory cannot be determined.
pub fn default_agent_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| {
        anyhow::anyhow!("Failed to find home directory - HOME environment variable may not be set")
    })?;
    Ok(home.join(".dc-agent"))
}

/// Generate agent keypair in the given directory.
/// Returns (private_key_path, pubkey_hex).
pub fn generate_agent_keypair(agent_dir: &Path, force: bool) -> Result<(PathBuf, String)> {
    let private_key_path = agent_dir.join("agent.key");
    let public_key_path = agent_dir.join("agent.pub");

    // Return existing key if not forcing overwrite
    if private_key_path.exists() && !force {
        let pubkey_hex = std::fs::read_to_string(&public_key_path)
            .with_context(|| {
                format!(
                    "Failed to read existing agent key: {}",
                    public_key_path.display()
                )
            })?
            .trim()
            .to_string();
        return Ok((private_key_path, pubkey_hex));
    }

    std::fs::create_dir_all(agent_dir)
        .with_context(|| format!("Failed to create agent directory: {}", agent_dir.display()))?;

    let signing_key = SigningKey::generate(&mut OsRng);
    let identity = DccIdentity::new_signing(&signing_key)?;

    let pubkey_bytes = identity.to_bytes_verifying();
    let pubkey_hex = hex::encode(pubkey_bytes);
    let secret_hex = hex::encode(signing_key.to_bytes());

    std::fs::write(&private_key_path, &secret_hex)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&private_key_path, std::fs::Permissions::from_mode(0o600))?;
    }

    std::fs::write(&public_key_path, &pubkey_hex)?;

    Ok((private_key_path, pubkey_hex))
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_agent_keypair_creates_files() {
        let temp_dir = TempDir::new().unwrap();
        let (key_path, pubkey_hex) = generate_agent_keypair(temp_dir.path(), false).unwrap();

        assert!(key_path.exists());
        assert!(temp_dir.path().join("agent.pub").exists());
        assert_eq!(pubkey_hex.len(), 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn test_generate_agent_keypair_returns_existing() {
        let temp_dir = TempDir::new().unwrap();

        let (_, pubkey1) = generate_agent_keypair(temp_dir.path(), false).unwrap();
        let (_, pubkey2) = generate_agent_keypair(temp_dir.path(), false).unwrap();

        assert_eq!(pubkey1, pubkey2);
    }

    #[test]
    fn test_generate_agent_keypair_force_regenerates() {
        let temp_dir = TempDir::new().unwrap();

        let (_, pubkey1) = generate_agent_keypair(temp_dir.path(), false).unwrap();
        let (_, pubkey2) = generate_agent_keypair(temp_dir.path(), true).unwrap();

        assert_ne!(pubkey1, pubkey2);
    }
}
