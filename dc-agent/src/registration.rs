//! Agent registration and delegation management.
//!
//! Handles registering agents with the Decent Cloud API,
//! including signing delegations and managing agent keypairs.

use anyhow::{bail, Context, Result};
use dcc_common::DccIdentity;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Default permissions granted to agent delegations.
pub const DEFAULT_PERMISSIONS: &[&str] =
    &["provision", "health_check", "heartbeat", "fetch_contracts"];

/// Get the default agent keys directory (~/.dc-agent).
pub fn default_agent_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Failed to find home directory")
        .join(".dc-agent")
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

/// Load agent public key from the default or specified directory.
pub fn load_agent_pubkey(keys_dir: Option<&Path>) -> Result<String> {
    let agent_dir = keys_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(default_agent_dir);
    let public_key_path = agent_dir.join("agent.pub");

    if !public_key_path.exists() {
        bail!(
            "Agent key not found at {}. Run 'dc-agent init' first.",
            public_key_path.display()
        );
    }

    let pubkey_hex = std::fs::read_to_string(&public_key_path)?
        .trim()
        .to_string();

    let pubkey_bytes = hex::decode(&pubkey_hex).context("Invalid agent public key hex")?;

    if pubkey_bytes.len() != 32 {
        bail!(
            "Agent public key must be 32 bytes, got {}",
            pubkey_bytes.len()
        );
    }

    Ok(pubkey_hex)
}

/// Find and load provider identity.
/// If identity is None, auto-detect from ~/.dcc/identity/
/// If identity is Some, treat as identity name or absolute path.
pub fn load_provider_identity(identity: Option<&str>) -> Result<DccIdentity> {
    let identities_dir = DccIdentity::identities_dir();

    let identity_dir = match identity {
        Some(name) => {
            let path = PathBuf::from(name);
            if path.is_absolute() {
                path
            } else {
                identities_dir.join(name)
            }
        }
        None => {
            if !identities_dir.exists() {
                bail!(
                    "No identities found. Create one with: dc identity new <name>\n\
                     Or specify --identity <path>"
                );
            }

            let mut identities: Vec<_> = std::fs::read_dir(&identities_dir)?
                .filter_map(|entry_result| {
                    match entry_result {
                        Ok(e) => Some(e),
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                dir = %identities_dir.display(),
                                "Failed to read directory entry while listing identities"
                            );
                            None
                        }
                    }
                })
                .filter(|e| e.path().is_dir())
                .collect();

            if identities.is_empty() {
                bail!(
                    "No identities found in {}. Create one with: dc identity new <name>",
                    identities_dir.display()
                );
            }

            if identities.len() > 1 {
                identities.sort_by_key(|e| e.file_name());
                let names: Vec<_> = identities
                    .iter()
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect();
                bail!(
                    "Multiple identities found: {}\n\
                     Specify which to use with: --identity <name>",
                    names.join(", ")
                );
            }

            identities.remove(0).path()
        }
    };

    if !identity_dir.exists() {
        bail!("Identity not found: {}", identity_dir.display());
    }

    DccIdentity::load_from_dir(&identity_dir).map_err(|e| {
        anyhow::anyhow!(
            "Failed to load identity from {}: {}",
            identity_dir.display(),
            e
        )
    })
}

/// Build delegation message for signing.
fn build_delegation_message(
    agent_pubkey: &[u8],
    provider_pubkey: &[u8],
    permissions: &[&str],
    label: Option<&str>,
) -> Result<Vec<u8>> {
    let permissions_json = serde_json::to_string(&permissions)?;

    let mut message = Vec::new();
    message.extend_from_slice(agent_pubkey);
    message.extend_from_slice(provider_pubkey);
    message.extend_from_slice(permissions_json.as_bytes());
    if let Some(lbl) = label {
        message.extend_from_slice(lbl.as_bytes());
    }

    Ok(message)
}

/// Sign an HTTP request for API authentication.
fn sign_request(
    identity: &DccIdentity,
    method: &str,
    path: &str,
    body: &str,
) -> Result<RequestAuth> {
    let timestamp = chrono::Utc::now()
        .timestamp_nanos_opt()
        .ok_or_else(|| anyhow::anyhow!("Failed to get timestamp in nanoseconds"))?;
    let nonce = uuid::Uuid::new_v4();
    let timestamp_str = timestamp.to_string();
    let nonce_str = nonce.to_string();

    let mut sign_message = Vec::new();
    sign_message.extend_from_slice(timestamp_str.as_bytes());
    sign_message.extend_from_slice(nonce_str.as_bytes());
    sign_message.extend_from_slice(method.as_bytes());
    sign_message.extend_from_slice(path.as_bytes());
    sign_message.extend_from_slice(body.as_bytes());

    let signature = identity.sign(&sign_message)?;
    let signature_hex = hex::encode(signature.to_bytes());

    Ok(RequestAuth {
        timestamp: timestamp_str,
        nonce: nonce_str,
        signature: signature_hex,
        pubkey: hex::encode(identity.to_bytes_verifying()),
    })
}

/// Authentication headers for API requests.
struct RequestAuth {
    timestamp: String,
    nonce: String,
    signature: String,
    pubkey: String,
}

/// API response wrapper.
#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    #[allow(dead_code)]
    data: Option<T>,
    error: Option<String>,
}

/// Request to create an agent delegation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateDelegationRequest {
    agent_pubkey: String,
    permissions: Vec<String>,
    expires_at_ns: Option<i64>,
    label: Option<String>,
    signature: String,
}

/// Register agent with API using provider identity.
pub async fn register_agent_with_api(
    provider_identity: &DccIdentity,
    agent_pubkey_hex: &str,
    api_endpoint: &str,
    label: Option<&str>,
) -> Result<()> {
    let provider_pubkey_bytes = provider_identity.to_bytes_verifying();
    let provider_pubkey = hex::encode(&provider_pubkey_bytes);

    let agent_pubkey = hex::decode(agent_pubkey_hex).context("Invalid agent public key hex")?;

    if agent_pubkey.len() != 32 {
        bail!("Agent public key must be 32 bytes");
    }

    // Build and sign delegation
    let message = build_delegation_message(
        &agent_pubkey,
        &provider_pubkey_bytes,
        DEFAULT_PERMISSIONS,
        label,
    )?;
    let signature = provider_identity.sign(&message)?;
    let signature_hex = hex::encode(signature.to_bytes());

    // Build request
    let path = format!("/api/v1/providers/{}/agent-delegations", provider_pubkey);
    let request_body = CreateDelegationRequest {
        agent_pubkey: agent_pubkey_hex.to_string(),
        permissions: DEFAULT_PERMISSIONS.iter().map(|s| s.to_string()).collect(),
        expires_at_ns: None,
        label: label.map(|s| s.to_string()),
        signature: signature_hex,
    };

    let body_json = serde_json::to_string(&request_body)?;
    let auth = sign_request(provider_identity, "POST", &path, &body_json)?;

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to build HTTP client")?;

    let url = format!("{}{}", api_endpoint, path);
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("X-Public-Key", &auth.pubkey)
        .header("X-Timestamp", &auth.timestamp)
        .header("X-Nonce", &auth.nonce)
        .header("X-Signature", auth.signature)
        .body(body_json)
        .send()
        .await
        .context("Failed to send registration request")?;

    let status = response.status();
    let response_text = response.text().await.context("Failed to read response")?;

    if !status.is_success() {
        bail!("Registration failed ({}): {}", status, response_text);
    }

    let api_response: ApiResponse<serde_json::Value> = serde_json::from_str(&response_text)
        .with_context(|| format!("Failed to parse API response: {}", response_text))?;

    if !api_response.success {
        bail!(
            "Registration failed: {}",
            api_response
                .error
                .unwrap_or_else(|| "Unknown error".to_string())
        );
    }

    Ok(())
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

    #[test]
    fn test_load_agent_pubkey_missing() {
        let temp_dir = TempDir::new().unwrap();
        let result = load_agent_pubkey(Some(temp_dir.path()));

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_load_agent_pubkey_success() {
        let temp_dir = TempDir::new().unwrap();
        let (_, expected_pubkey) = generate_agent_keypair(temp_dir.path(), false).unwrap();

        let loaded_pubkey = load_agent_pubkey(Some(temp_dir.path())).unwrap();

        assert_eq!(expected_pubkey, loaded_pubkey);
    }

    #[test]
    fn test_build_delegation_message() {
        let agent_pubkey = [1u8; 32];
        let provider_pubkey = [2u8; 32];

        let message = build_delegation_message(
            &agent_pubkey,
            &provider_pubkey,
            DEFAULT_PERMISSIONS,
            Some("test-label"),
        )
        .unwrap();

        // Should contain agent pubkey (32) + provider pubkey (32) + permissions json + label
        assert!(message.len() > 64);
        assert!(message.starts_with(&agent_pubkey));
    }

    #[test]
    fn test_default_permissions_not_empty() {
        assert!(!DEFAULT_PERMISSIONS.is_empty());
        assert!(DEFAULT_PERMISSIONS.contains(&"provision"));
        assert!(DEFAULT_PERMISSIONS.contains(&"heartbeat"));
    }
}
