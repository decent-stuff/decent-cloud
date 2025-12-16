use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use ed25519_dalek::{Signature, Signer, SigningKey};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::ApiConfig;
use crate::provisioner::{HealthStatus, Instance};

pub struct ApiClient {
    client: Client,
    endpoint: String,
    provider_pubkey: String,
    signing_key: SigningKey,
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

impl ApiClient {
    pub fn new(config: &ApiConfig) -> Result<Self> {
        let signing_key = Self::load_signing_key(&config.provider_secret_key)?;

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self {
            client,
            endpoint: config.endpoint.clone(),
            provider_pubkey: config.provider_pubkey.clone(),
            signing_key,
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

    fn sign_request(&self, method: &str, path: &str, timestamp: i64) -> String {
        let message = format!("{}{}{}", method, path, timestamp);
        let signature: Signature = self.signing_key.sign(message.as_bytes());
        BASE64.encode(signature.to_bytes())
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

        let instance_json = serde_json::to_string(instance)
            .context("Failed to serialize instance details")?;

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

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let timestamp = chrono::Utc::now().timestamp();
        let signature = self.sign_request("GET", path, timestamp);

        let url = format!("{}{}", self.endpoint, path);
        let response = self
            .client
            .get(&url)
            .header("X-Provider-Pubkey", &self.provider_pubkey)
            .header("X-Timestamp", timestamp.to_string())
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

        serde_json::from_str(&body).with_context(|| {
            format!(
                "Failed to deserialize response from GET {}: {}",
                path, body
            )
        })
    }

    async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let timestamp = chrono::Utc::now().timestamp();
        let signature = self.sign_request("POST", path, timestamp);

        let url = format!("{}{}", self.endpoint, path);
        let response = self
            .client
            .post(&url)
            .header("X-Provider-Pubkey", &self.provider_pubkey)
            .header("X-Timestamp", timestamp.to_string())
            .header("X-Signature", signature)
            .header("Content-Type", "application/json")
            .json(body)
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
        let short_key = hex::encode(&[1u8; 16]); // Only 16 bytes
        let result = ApiClient::load_signing_key(&short_key);
        assert!(result.is_err());
    }

    #[test]
    fn test_sign_request() {
        let signing_key = SigningKey::from_bytes(&[88u8; 32]);
        let verifying_key = signing_key.verifying_key();

        let config = ApiConfig {
            endpoint: "https://api.example.com".to_string(),
            provider_pubkey: "test_pubkey".to_string(),
            provider_secret_key: hex::encode(signing_key.to_bytes()),
        };

        let client = ApiClient::new(&config).unwrap();

        let method = "GET";
        let path = "/api/v1/test";
        let timestamp = 1234567890i64;

        let signature_b64 = client.sign_request(method, path, timestamp);

        // Decode and verify signature
        let signature_bytes = BASE64.decode(signature_b64).unwrap();
        let signature = Signature::from_bytes(&signature_bytes.try_into().unwrap());

        let message = format!("{}{}{}", method, path, timestamp);
        assert!(verifying_key.verify(message.as_bytes(), &signature).is_ok());
    }

    #[test]
    fn test_sign_request_different_methods() {
        let signing_key = SigningKey::from_bytes(&[123u8; 32]);

        let config = ApiConfig {
            endpoint: "https://api.example.com".to_string(),
            provider_pubkey: "test_pubkey".to_string(),
            provider_secret_key: hex::encode(signing_key.to_bytes()),
        };

        let client = ApiClient::new(&config).unwrap();

        let sig_get = client.sign_request("GET", "/path", 1000);
        let sig_post = client.sign_request("POST", "/path", 1000);

        // Different methods should produce different signatures
        assert_ne!(sig_get, sig_post);
    }

    #[test]
    fn test_sign_request_different_timestamps() {
        let signing_key = SigningKey::from_bytes(&[200u8; 32]);

        let config = ApiConfig {
            endpoint: "https://api.example.com".to_string(),
            provider_pubkey: "test_pubkey".to_string(),
            provider_secret_key: hex::encode(signing_key.to_bytes()),
        };

        let client = ApiClient::new(&config).unwrap();

        let sig1 = client.sign_request("GET", "/path", 1000);
        let sig2 = client.sign_request("GET", "/path", 2000);

        // Different timestamps should produce different signatures
        assert_ne!(sig1, sig2);
    }
}
