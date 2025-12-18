use anyhow::{bail, Context, Result};
use ed25519_dalek::{Signature, Signer, SigningKey};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::ApiConfig;
use crate::provisioner::{HealthStatus, Instance};

/// Authentication mode for the API client
#[derive(Debug, Clone)]
pub enum AuthMode {
    /// Legacy mode: using provider's main key directly
    Provider,
    /// Delegated mode: using agent's delegated key
    Agent { agent_pubkey: String },
}

#[derive(Debug)]
pub struct ApiClient {
    client: Client,
    endpoint: String,
    provider_pubkey: String,
    signing_key: SigningKey,
    auth_mode: AuthMode,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingContract {
    pub contract_id: String,
    pub offering_id: String,
    pub requester_ssh_pubkey: String,
    pub instance_config: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProvisionedRequest {
    pub status: String,
    pub instance_details: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProvisionFailedRequest {
    pub status: String,
    pub instance_details: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthCheckRequest {
    pub health_status: HealthStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HeartbeatRequest {
    pub version: Option<String>,
    pub provisioner_type: Option<String>,
    pub capabilities: Option<Vec<String>>,
    pub active_contracts: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatResponse {
    pub acknowledged: bool,
    pub next_heartbeat_seconds: i64,
}

impl ApiClient {
    pub fn new(config: &ApiConfig) -> Result<Self> {
        // Determine auth mode and load the appropriate key
        let (signing_key, auth_mode) = if let Some(agent_key) = &config.agent_secret_key {
            let signing_key = Self::load_signing_key(agent_key)?;
            // Get agent public key from the signing key
            let agent_pubkey = hex::encode(signing_key.verifying_key().to_bytes());
            (signing_key, AuthMode::Agent { agent_pubkey })
        } else if let Some(provider_key) = &config.provider_secret_key {
            let signing_key = Self::load_signing_key(provider_key)?;
            (signing_key, AuthMode::Provider)
        } else {
            bail!("Either agent_secret_key or provider_secret_key must be configured");
        };

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self {
            client,
            endpoint: config.endpoint.clone(),
            provider_pubkey: config.provider_pubkey.clone(),
            signing_key,
            auth_mode,
        })
    }

    fn load_signing_key(key_or_path: &str) -> Result<SigningKey> {
        // Try hex first
        if let Ok(bytes) = hex::decode(key_or_path) {
            if bytes.len() == 32 {
                let key_bytes: [u8; 32] = bytes.try_into().unwrap();
                return Ok(SigningKey::from_bytes(&key_bytes));
            }
        }

        // Try file path
        let file_content = std::fs::read_to_string(key_or_path)
            .with_context(|| format!("Failed to read signing key from file: {}", key_or_path))?;

        let trimmed = file_content.trim();
        let bytes = hex::decode(trimmed).context("Key file must contain hex-encoded key")?;

        if bytes.len() != 32 {
            bail!(
                "Ed25519 signing key must be 32 bytes, got {} bytes",
                bytes.len()
            );
        }

        let key_bytes: [u8; 32] = bytes.try_into().unwrap();
        Ok(SigningKey::from_bytes(&key_bytes))
    }

    /// Build authentication headers for API requests.
    /// Returns (timestamp_str, nonce_str, signature_hex)
    fn build_auth_headers(&self, method: &str, path: &str, body: &[u8]) -> Result<(String, String, String)> {
        // API expects: timestamp (nanoseconds) + nonce (UUID) + method + path + body
        let timestamp = chrono::Utc::now()
            .timestamp_nanos_opt()
            .ok_or_else(|| anyhow::anyhow!("Failed to get timestamp in nanoseconds"))?;
        let nonce = Uuid::new_v4();
        let timestamp_str = timestamp.to_string();
        let nonce_str = nonce.to_string();

        let mut sign_message = Vec::new();
        sign_message.extend_from_slice(timestamp_str.as_bytes());
        sign_message.extend_from_slice(nonce_str.as_bytes());
        sign_message.extend_from_slice(method.as_bytes());
        sign_message.extend_from_slice(path.as_bytes());
        sign_message.extend_from_slice(body);

        let signature: Signature = self.signing_key.sign(&sign_message);
        let signature_hex = hex::encode(signature.to_bytes());

        Ok((timestamp_str, nonce_str, signature_hex))
    }

    /// Get contracts pending provisioning
    pub async fn get_pending_contracts(&self) -> Result<Vec<PendingContract>> {
        let path = format!(
            "/api/v1/providers/{}/contracts/pending-provision",
            self.provider_pubkey
        );
        let response: ApiResponse<Vec<PendingContract>> = self.get(&path).await?;

        if !response.success {
            let error_msg = response
                .error
                .unwrap_or_else(|| "Unknown error".to_string());
            bail!("API error: {}", error_msg);
        }

        response
            .data
            .ok_or_else(|| anyhow::anyhow!("No data in response"))
    }

    /// Report successful provisioning
    pub async fn report_provisioned(&self, contract_id: &str, instance: &Instance) -> Result<()> {
        let path = format!(
            "/api/v1/provider/rental-requests/{}/provisioning",
            contract_id
        );

        let instance_json =
            serde_json::to_string(instance).context("Failed to serialize instance details")?;

        let request = ProvisionedRequest {
            status: "provisioned".to_string(),
            instance_details: instance_json,
        };

        let response: ApiResponse<serde_json::Value> = self.post(&path, &request).await?;

        if !response.success {
            let error_msg = response
                .error
                .unwrap_or_else(|| "Unknown error".to_string());
            bail!("API error: {}", error_msg);
        }

        Ok(())
    }

    /// Report provisioning failure
    pub async fn report_failed(&self, contract_id: &str, error: &str) -> Result<()> {
        let path = format!(
            "/api/v1/provider/rental-requests/{}/provision-failed",
            contract_id
        );

        let request = ProvisionFailedRequest {
            status: "provision-failed".to_string(),
            instance_details: error.to_string(),
        };

        let response: ApiResponse<serde_json::Value> = self.post(&path, &request).await?;

        if !response.success {
            let error_msg = response
                .error
                .unwrap_or_else(|| "Unknown error".to_string());
            bail!("API error: {}", error_msg);
        }

        Ok(())
    }

    /// Report health check
    pub async fn report_health(&self, contract_id: &str, status: &HealthStatus) -> Result<()> {
        let path = format!("/api/v1/provider/contracts/{}/health", contract_id);

        let request = HealthCheckRequest {
            health_status: status.clone(),
        };

        let response: ApiResponse<serde_json::Value> = self.post(&path, &request).await?;

        if !response.success {
            let error_msg = response
                .error
                .unwrap_or_else(|| "Unknown error".to_string());
            bail!("API error: {}", error_msg);
        }

        Ok(())
    }

    /// Send heartbeat to report agent is online
    pub async fn send_heartbeat(
        &self,
        version: Option<&str>,
        provisioner_type: Option<&str>,
        capabilities: Option<&[String]>,
        active_contracts: i64,
    ) -> Result<HeartbeatResponse> {
        let path = format!("/api/v1/providers/{}/heartbeat", self.provider_pubkey);

        let request = HeartbeatRequest {
            version: version.map(|s| s.to_string()),
            provisioner_type: provisioner_type.map(|s| s.to_string()),
            capabilities: capabilities.map(|c| c.to_vec()),
            active_contracts,
        };

        let response: ApiResponse<HeartbeatResponse> =
            self.post_agent_auth(&path, &request).await?;

        if !response.success {
            let error_msg = response
                .error
                .unwrap_or_else(|| "Unknown error".to_string());
            bail!("Heartbeat failed: {}", error_msg);
        }

        response
            .data
            .ok_or_else(|| anyhow::anyhow!("No heartbeat response data"))
    }

    /// Returns the auth mode for diagnostics
    pub fn auth_mode(&self) -> &AuthMode {
        &self.auth_mode
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let (timestamp_str, nonce_str, signature) = self.build_auth_headers("GET", path, b"")?;

        let url = format!("{}{}", self.endpoint, path);
        let response = self
            .client
            .get(&url)
            .header("X-Public-Key", &self.provider_pubkey)
            .header("X-Timestamp", &timestamp_str)
            .header("X-Nonce", &nonce_str)
            .header("X-Signature", signature)
            .send()
            .await
            .with_context(|| format!("Failed to GET {}", path))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("Failed to read response body")?;

        if !status.is_success() {
            bail!("HTTP {} for GET {}: {}", status, path, body);
        }

        serde_json::from_str(&body)
            .with_context(|| format!("Failed to deserialize response from GET {}: {}", path, body))
    }

    async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", self.endpoint, path);
        let body_json = serde_json::to_string(body).context("Failed to serialize request body")?;
        let (timestamp_str, nonce_str, signature) =
            self.build_auth_headers("POST", path, body_json.as_bytes())?;

        let response = self
            .client
            .post(&url)
            .header("X-Public-Key", &self.provider_pubkey)
            .header("X-Timestamp", &timestamp_str)
            .header("X-Nonce", &nonce_str)
            .header("X-Signature", signature)
            .header("Content-Type", "application/json")
            .body(body_json)
            .send()
            .await
            .with_context(|| format!("Failed to POST {}", path))?;

        let status = response.status();
        let response_body = response
            .text()
            .await
            .context("Failed to read response body")?;

        if !status.is_success() {
            bail!("HTTP {} for POST {}: {}", status, path, response_body);
        }

        serde_json::from_str(&response_body).with_context(|| {
            format!(
                "Failed to deserialize response from POST {}: {}",
                path, response_body
            )
        })
    }

    /// POST with agent authentication (uses X-Agent-Pubkey header for delegated auth)
    async fn post_agent_auth<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", self.endpoint, path);
        let body_json = serde_json::to_string(body).context("Failed to serialize request body")?;
        let (timestamp_str, nonce_str, signature) =
            self.build_auth_headers("POST", path, body_json.as_bytes())?;

        // Set auth headers based on mode
        let mut request_builder = self
            .client
            .post(&url)
            .header("X-Timestamp", &timestamp_str)
            .header("X-Nonce", &nonce_str)
            .header("X-Signature", signature)
            .header("Content-Type", "application/json");

        match &self.auth_mode {
            AuthMode::Agent { agent_pubkey } => {
                request_builder = request_builder.header("X-Agent-Pubkey", agent_pubkey);
            }
            AuthMode::Provider => {
                request_builder = request_builder.header("X-Public-Key", &self.provider_pubkey);
            }
        }

        let response = request_builder
            .body(body_json)
            .send()
            .await
            .with_context(|| format!("Failed to POST {}", path))?;

        let status = response.status();
        let response_body = response
            .text()
            .await
            .context("Failed to read response body")?;

        if !status.is_success() {
            bail!("HTTP {} for POST {}: {}", status, path, response_body);
        }

        serde_json::from_str(&response_body).with_context(|| {
            format!(
                "Failed to deserialize response from POST {}: {}",
                path, response_body
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Verifier;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_signing_key_from_hex() {
        // Generate a valid Ed25519 key for testing
        let signing_key = SigningKey::from_bytes(&[42u8; 32]);
        let hex_key = hex::encode(signing_key.to_bytes());

        let loaded_key = ApiClient::load_signing_key(&hex_key).unwrap();
        assert_eq!(signing_key.to_bytes(), loaded_key.to_bytes());
    }

    #[test]
    fn test_load_signing_key_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join("secret.key");

        let signing_key = SigningKey::from_bytes(&[99u8; 32]);
        let hex_key = hex::encode(signing_key.to_bytes());
        fs::write(&key_path, hex_key).unwrap();

        let loaded_key = ApiClient::load_signing_key(key_path.to_str().unwrap()).unwrap();
        assert_eq!(signing_key.to_bytes(), loaded_key.to_bytes());
    }

    #[test]
    fn test_load_signing_key_from_file_with_whitespace() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join("secret.key");

        let signing_key = SigningKey::from_bytes(&[77u8; 32]);
        let hex_key = format!("  {}\n", hex::encode(signing_key.to_bytes()));
        fs::write(&key_path, hex_key).unwrap();

        let loaded_key = ApiClient::load_signing_key(key_path.to_str().unwrap()).unwrap();
        assert_eq!(signing_key.to_bytes(), loaded_key.to_bytes());
    }

    #[test]
    fn test_load_signing_key_invalid_hex() {
        let result = ApiClient::load_signing_key("not_hex");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to read signing key from file"));
    }

    #[test]
    fn test_load_signing_key_wrong_length() {
        let short_key = hex::encode([1u8; 16]); // Only 16 bytes
        let result = ApiClient::load_signing_key(&short_key);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_auth_headers() {
        let signing_key = SigningKey::from_bytes(&[88u8; 32]);
        let verifying_key = signing_key.verifying_key();

        let config = ApiConfig {
            endpoint: "https://api.example.com".to_string(),
            provider_pubkey: "test_pubkey".to_string(),
            agent_secret_key: None,
            provider_secret_key: Some(hex::encode(signing_key.to_bytes())),
        };

        let client = ApiClient::new(&config).unwrap();

        let method = "GET";
        let path = "/api/v1/test";
        let body = b"";

        let (timestamp_str, nonce_str, signature_hex) =
            client.build_auth_headers(method, path, body).unwrap();

        // Verify timestamp is nanoseconds (at least 19 digits)
        assert!(timestamp_str.len() >= 19);

        // Verify nonce is UUID format
        assert!(uuid::Uuid::parse_str(&nonce_str).is_ok());

        // Verify signature is valid hex
        let signature_bytes = hex::decode(&signature_hex).unwrap();
        assert_eq!(signature_bytes.len(), 64);

        // Verify the signature is valid for the message
        let mut message = Vec::new();
        message.extend_from_slice(timestamp_str.as_bytes());
        message.extend_from_slice(nonce_str.as_bytes());
        message.extend_from_slice(method.as_bytes());
        message.extend_from_slice(path.as_bytes());
        message.extend_from_slice(body);

        let signature = Signature::from_bytes(&signature_bytes.try_into().unwrap());
        assert!(verifying_key.verify(&message, &signature).is_ok());
    }

    #[test]
    fn test_build_auth_headers_unique_nonce() {
        let signing_key = SigningKey::from_bytes(&[123u8; 32]);

        let config = ApiConfig {
            endpoint: "https://api.example.com".to_string(),
            provider_pubkey: "test_pubkey".to_string(),
            agent_secret_key: None,
            provider_secret_key: Some(hex::encode(signing_key.to_bytes())),
        };

        let client = ApiClient::new(&config).unwrap();

        let (_, nonce1, _) = client.build_auth_headers("GET", "/path", b"").unwrap();
        let (_, nonce2, _) = client.build_auth_headers("GET", "/path", b"").unwrap();

        // Each call should produce a unique nonce
        assert_ne!(nonce1, nonce2);
    }

    #[test]
    fn test_auth_mode_agent() {
        let signing_key = SigningKey::from_bytes(&[100u8; 32]);
        let expected_pubkey = hex::encode(signing_key.verifying_key().to_bytes());

        let config = ApiConfig {
            endpoint: "https://api.example.com".to_string(),
            provider_pubkey: "provider_pubkey".to_string(),
            agent_secret_key: Some(hex::encode(signing_key.to_bytes())),
            provider_secret_key: None,
        };

        let client = ApiClient::new(&config).unwrap();

        match client.auth_mode() {
            AuthMode::Agent { agent_pubkey } => {
                assert_eq!(*agent_pubkey, expected_pubkey);
            }
            AuthMode::Provider => panic!("Expected Agent auth mode"),
        }
    }

    #[test]
    fn test_auth_mode_provider() {
        let signing_key = SigningKey::from_bytes(&[101u8; 32]);

        let config = ApiConfig {
            endpoint: "https://api.example.com".to_string(),
            provider_pubkey: "provider_pubkey".to_string(),
            agent_secret_key: None,
            provider_secret_key: Some(hex::encode(signing_key.to_bytes())),
        };

        let client = ApiClient::new(&config).unwrap();

        match client.auth_mode() {
            AuthMode::Provider => {}
            AuthMode::Agent { .. } => panic!("Expected Provider auth mode"),
        }
    }

    #[test]
    fn test_no_key_configured_fails() {
        let config = ApiConfig {
            endpoint: "https://api.example.com".to_string(),
            provider_pubkey: "provider_pubkey".to_string(),
            agent_secret_key: None,
            provider_secret_key: None,
        };

        let result = ApiClient::new(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must be configured"));
    }
}
