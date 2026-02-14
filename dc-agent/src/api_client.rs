use anyhow::{bail, Context, Result};
use dcc_common::DccIdentity;
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

use crate::config::ApiConfig;
use crate::provisioner::{HealthStatus, Instance, RunningInstance};

// Re-export shared types from dcc-common
pub use dcc_common::api_types::{
    GpuDeviceInfo, HeartbeatResponse, LockResponse, ReconcileKeepInstance, ReconcileResponse,
    ReconcileTerminateInstance, ReconcileUnknownInstance, ResourceInventory, StoragePoolInfo,
    TemplateInfo, VmBandwidthReport,
};

/// Authentication mode for the API client.
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
    identity: DccIdentity,
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
pub struct PendingContract {
    pub contract_id: String,
    pub offering_id: String,
    pub requester_ssh_pubkey: String,
    pub instance_config: Option<String>,
    /// CPU cores from offering (processor_cores)
    pub cpu_cores: Option<i64>,
    /// Memory amount from offering (e.g. "16 GB")
    pub memory_amount: Option<String>,
    /// Storage capacity from offering (e.g. "100 GB")
    pub storage_capacity: Option<String>,
    /// Provisioner type from offering (e.g. "proxmox", "script", "manual")
    /// NULL = use agent's default provisioner
    pub provisioner_type: Option<String>,
    /// Provisioner config JSON from offering
    pub provisioner_config: Option<String>,
    /// Script to execute via SSH after VM provisioning (uses shebang for interpreter)
    pub post_provision_script: Option<String>,
}

/// Contract pending termination (cancelled with VM still running)
#[derive(Debug, Deserialize)]
pub struct ContractPendingTermination {
    pub contract_id: String,
    /// Instance details JSON (contains external_id needed for termination)
    pub instance_details: String,
}

/// Contract pending password reset
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractPendingPasswordReset {
    pub contract_id: String,
}

impl PendingContract {
    /// Parse memory amount string to MB
    /// Handles formats like "16 GB", "2048 MB", "16GB", "2048MB"
    pub fn memory_mb(&self) -> Option<u32> {
        parse_size_to_mb(self.memory_amount.as_deref())
    }

    /// Parse storage capacity string to GB
    /// Handles formats like "100 GB", "500GB", "1 TB", "1TB"
    pub fn storage_gb(&self) -> Option<u32> {
        parse_size_to_gb(self.storage_capacity.as_deref())
    }
}

/// Target unit for size parsing
#[derive(Clone, Copy)]
enum SizeUnit {
    MB,
    GB,
}

/// Parse size string to bytes, then convert to target unit.
/// Supports: "1TB", "16GB", "512MB", "1 TB", "16 GB", "512 MB"
fn parse_size(size_str: Option<&str>, target_unit: SizeUnit) -> Option<u32> {
    let s = size_str?.trim().to_uppercase();

    // Try to extract number and unit
    let (num_str, unit) = if s.ends_with("TB") {
        (s.trim_end_matches("TB").trim(), "TB")
    } else if s.ends_with("GB") {
        (s.trim_end_matches("GB").trim(), "GB")
    } else if s.ends_with("MB") {
        (s.trim_end_matches("MB").trim(), "MB")
    } else {
        // Try splitting on space
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() >= 2 {
            (parts[0], parts[1])
        } else {
            return None;
        }
    };

    let num: f64 = num_str.parse().ok()?;

    // Convert to MB first (common base), then to target unit
    let mb = match unit {
        "TB" => num * 1024.0 * 1024.0,
        "GB" => num * 1024.0,
        "MB" => num,
        _ => return None,
    };

    let result = match target_unit {
        SizeUnit::MB => mb,
        SizeUnit::GB => mb / 1024.0,
    };

    Some(result as u32)
}

/// Parse size string to MB (e.g. "16 GB" -> 16384, "2048 MB" -> 2048)
fn parse_size_to_mb(size_str: Option<&str>) -> Option<u32> {
    parse_size(size_str, SizeUnit::MB)
}

/// Parse size string to GB (e.g. "100 GB" -> 100, "1 TB" -> 1024)
fn parse_size_to_gb(size_str: Option<&str>) -> Option<u32> {
    parse_size(size_str, SizeUnit::GB)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProvisionedRequest {
    status: String,
    instance_details: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProvisionFailedRequest {
    status: String,
    instance_details: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthCheckRequest {
    health_status: HealthStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HeartbeatRequest {
    version: Option<String>,
    provisioner_type: Option<String>,
    capabilities: Option<Vec<String>>,
    active_contracts: i64,
    /// Per-VM bandwidth stats (optional, only if gateway is configured)
    #[serde(skip_serializing_if = "Option::is_none")]
    bandwidth_stats: Option<Vec<VmBandwidthReport>>,
    /// Hardware resource inventory (optional, reported periodically)
    #[serde(skip_serializing_if = "Option::is_none")]
    resources: Option<ResourceInventory>,
}

// Reconciliation types (internal request types)

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReconcileRunningInstanceRequest {
    external_id: String,
    contract_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReconcileRequest {
    running_instances: Vec<ReconcileRunningInstanceRequest>,
}

/// HTTP method for requests.
#[derive(Clone, Copy)]
enum Method {
    Get,
    Post,
    Put,
    Delete,
}

impl Method {
    fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
        }
    }
}

impl ApiClient {
    pub fn new(config: &ApiConfig) -> Result<Self> {
        let (identity, auth_mode) = if let Some(agent_key) = &config.agent_secret_key {
            let identity = Self::load_identity(agent_key)?;
            let agent_pubkey = hex::encode(identity.to_bytes_verifying());
            (identity, AuthMode::Agent { agent_pubkey })
        } else if let Some(provider_key) = &config.provider_secret_key {
            let identity = Self::load_identity(provider_key)?;
            (identity, AuthMode::Provider)
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
            identity,
            auth_mode,
        })
    }

    fn load_identity(key_or_path: &str) -> Result<DccIdentity> {
        // Try hex first
        if let Ok(bytes) = hex::decode(key_or_path) {
            if bytes.len() == 32 {
                return DccIdentity::new_signing_from_bytes(&bytes)
                    .map_err(|e| anyhow::anyhow!("Failed to create identity from hex key: {}", e));
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

        DccIdentity::new_signing_from_bytes(&bytes)
            .map_err(|e| anyhow::anyhow!("Failed to create identity from key file: {}", e))
    }

    /// Build authentication headers for API requests.
    fn build_auth_headers(
        &self,
        method: &str,
        path: &str,
        body: &[u8],
    ) -> Result<(String, String, String)> {
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

        let signature = self
            .identity
            .sign(&sign_message)
            .map_err(|e| anyhow::anyhow!("Failed to sign message: {}", e))?;
        let signature_hex = hex::encode(signature.to_bytes());

        Ok((timestamp_str, nonce_str, signature_hex))
    }

    /// Execute an HTTP request with authentication.
    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&[u8]>,
    ) -> Result<T> {
        let body_bytes = body.unwrap_or(&[]);
        let (timestamp_str, nonce_str, signature) =
            self.build_auth_headers(method.as_str(), path, body_bytes)?;

        let url = format!("{}{}", self.endpoint, path);
        let mut request_builder = match method {
            Method::Get => self.client.get(&url),
            Method::Post => self.client.post(&url),
            Method::Put => self.client.put(&url),
            Method::Delete => self.client.delete(&url),
        };

        // Add auth headers
        request_builder = request_builder
            .header("X-Timestamp", &timestamp_str)
            .header("X-Nonce", &nonce_str)
            .header("X-Signature", signature);

        // Set identity header based on auth mode
        request_builder = match &self.auth_mode {
            AuthMode::Agent { agent_pubkey } => {
                request_builder.header("X-Agent-Pubkey", agent_pubkey)
            }
            AuthMode::Provider => request_builder.header("X-Public-Key", &self.provider_pubkey),
        };

        // Add body if present
        if let Some(body_data) = body {
            request_builder = request_builder
                .header("Content-Type", "application/json")
                .body(body_data.to_vec());
        }

        let response = request_builder
            .send()
            .await
            .with_context(|| format!("Failed to {} {}", method.as_str(), path))?;

        let status = response.status();
        let response_body = response
            .text()
            .await
            .context("Failed to read response body")?;

        if !status.is_success() {
            bail!(
                "HTTP {} for {} {}: {}",
                status,
                method.as_str(),
                path,
                response_body
            );
        }

        serde_json::from_str(&response_body).with_context(|| {
            format!(
                "Failed to deserialize response from {} {}: {}",
                method.as_str(),
                path,
                response_body
            )
        })
    }

    /// Helper to unwrap API response, checking success field.
    fn unwrap_response<T>(response: ApiResponse<T>, context: &str) -> Result<T> {
        if !response.success {
            let error_msg = response
                .error
                .unwrap_or_else(|| "Unknown error".to_string());
            bail!("{}: {}", context, error_msg);
        }
        response
            .data
            .ok_or_else(|| anyhow::anyhow!("{}: No data in response", context))
    }

    /// Get contracts pending provisioning.
    pub async fn get_pending_contracts(&self) -> Result<Vec<PendingContract>> {
        let path = format!(
            "/api/v1/providers/{}/contracts/pending-provision",
            self.provider_pubkey
        );
        let response: ApiResponse<Vec<PendingContract>> =
            self.request(Method::Get, &path, None).await?;
        Self::unwrap_response(response, "API error")
    }

    /// Report that provisioning has started (transitions status from accepted to provisioning).
    pub async fn report_provisioning_started(&self, contract_id: &str) -> Result<()> {
        let path = format!(
            "/api/v1/provider/rental-requests/{}/provisioning",
            contract_id
        );
        let request = ProvisionedRequest {
            status: "provisioning".to_string(),
            instance_details: String::new(),
        };
        let body = serde_json::to_vec(&request)?;
        let response: ApiResponse<serde_json::Value> =
            self.request(Method::Put, &path, Some(&body)).await?;
        Self::unwrap_response(response, "API error").map(|_| ())
    }

    /// Report successful provisioning.
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
        let body = serde_json::to_vec(&request)?;
        let response: ApiResponse<serde_json::Value> =
            self.request(Method::Put, &path, Some(&body)).await?;
        Self::unwrap_response(response, "API error").map(|_| ())
    }

    /// Report provisioning failure.
    pub async fn report_failed(&self, contract_id: &str, error: &str) -> Result<()> {
        let path = format!(
            "/api/v1/provider/rental-requests/{}/provisioning",
            contract_id
        );
        let request = ProvisionFailedRequest {
            status: "provision-failed".to_string(),
            instance_details: error.to_string(),
        };
        let body = serde_json::to_vec(&request)?;
        let response: ApiResponse<serde_json::Value> =
            self.request(Method::Put, &path, Some(&body)).await?;
        Self::unwrap_response(response, "API error").map(|_| ())
    }

    /// Report health check.
    pub async fn report_health(&self, contract_id: &str, status: &HealthStatus) -> Result<()> {
        let path = format!("/api/v1/provider/contracts/{}/health", contract_id);
        let request = HealthCheckRequest {
            health_status: status.clone(),
        };
        let body = serde_json::to_vec(&request)?;
        let response: ApiResponse<serde_json::Value> =
            self.request(Method::Post, &path, Some(&body)).await?;
        Self::unwrap_response(response, "API error").map(|_| ())
    }

    /// Send heartbeat to report agent is online.
    pub async fn send_heartbeat(
        &self,
        version: Option<&str>,
        provisioner_type: Option<&str>,
        capabilities: Option<&[String]>,
        active_contracts: i64,
        bandwidth_stats: Option<Vec<VmBandwidthReport>>,
        resources: Option<ResourceInventory>,
    ) -> Result<HeartbeatResponse> {
        let path = format!("/api/v1/providers/{}/heartbeat", self.provider_pubkey);
        let request = HeartbeatRequest {
            version: version.map(String::from),
            provisioner_type: provisioner_type.map(String::from),
            capabilities: capabilities.map(|c| c.to_vec()),
            active_contracts,
            bandwidth_stats,
            resources,
        };
        let body = serde_json::to_vec(&request)?;
        let response: ApiResponse<HeartbeatResponse> =
            self.request(Method::Post, &path, Some(&body)).await?;
        Self::unwrap_response(response, "Heartbeat failed")
    }

    /// Get contracts pending termination.
    pub async fn get_pending_terminations(&self) -> Result<Vec<ContractPendingTermination>> {
        let path = format!(
            "/api/v1/providers/{}/contracts/pending-termination",
            self.provider_pubkey
        );
        let response: ApiResponse<Vec<ContractPendingTermination>> =
            self.request(Method::Get, &path, None).await?;
        Self::unwrap_response(response, "API error")
    }

    /// Get contracts pending password reset.
    pub async fn get_pending_password_resets(&self) -> Result<Vec<ContractPendingPasswordReset>> {
        let path = format!(
            "/api/v1/providers/{}/contracts/pending-password-reset",
            self.provider_pubkey
        );
        let response: ApiResponse<Vec<ContractPendingPasswordReset>> =
            self.request(Method::Get, &path, None).await?;
        Self::unwrap_response(response, "API error")
    }

    /// Report successful termination.
    pub async fn report_terminated(&self, contract_id: &str) -> Result<()> {
        let path = format!(
            "/api/v1/providers/{}/contracts/{}/terminated",
            self.provider_pubkey, contract_id
        );
        let response: ApiResponse<serde_json::Value> =
            self.request(Method::Put, &path, None).await?;
        Self::unwrap_response(response, "API error").map(|_| ())
    }

    /// Update contract password after resetting via SSH.
    pub async fn update_contract_password(
        &self,
        contract_id: &str,
        new_password: &str,
    ) -> Result<()> {
        let path = format!(
            "/api/v1/provider/rental-requests/{}/password",
            contract_id
        );
        let request = serde_json::json!({ "newPassword": new_password });
        let body = serde_json::to_vec(&request)?;
        let response: ApiResponse<serde_json::Value> =
            self.request(Method::Put, &path, Some(&body)).await?;
        Self::unwrap_response(response, "Failed to update password").map(|_| ())
    }

    /// Reconcile running instances with API.
    /// Returns which VMs to keep, terminate, or are unknown (orphans).
    pub async fn reconcile(
        &self,
        running_instances: &[RunningInstance],
    ) -> Result<ReconcileResponse> {
        let path = format!("/api/v1/providers/{}/reconcile", self.provider_pubkey);
        let request = ReconcileRequest {
            running_instances: running_instances
                .iter()
                .map(|i| ReconcileRunningInstanceRequest {
                    external_id: i.external_id.clone(),
                    contract_id: i.contract_id.clone(),
                })
                .collect(),
        };
        let body = serde_json::to_vec(&request)?;
        let response: ApiResponse<ReconcileResponse> =
            self.request(Method::Post, &path, Some(&body)).await?;
        Self::unwrap_response(response, "Reconcile failed")
    }

    /// Returns the auth mode for diagnostics.
    pub fn auth_mode(&self) -> &AuthMode {
        &self.auth_mode
    }

    /// Acquire a provisioning lock for a contract.
    /// Returns Ok(true) if lock acquired, Ok(false) if already locked by another agent.
    pub async fn acquire_lock(&self, contract_id: &str) -> Result<bool> {
        let path = format!(
            "/api/v1/providers/{}/contracts/{}/lock",
            self.provider_pubkey, contract_id
        );
        let response: ApiResponse<LockResponse> = self.request(Method::Post, &path, None).await?;
        match Self::unwrap_response(response, "Failed to acquire lock") {
            Ok(r) => Ok(r.acquired),
            Err(e) => {
                // 409 Conflict means locked by another agent
                if e.to_string().contains("409") || e.to_string().contains("Conflict") {
                    Ok(false)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Release a provisioning lock without provisioning (giving up).
    pub async fn release_lock(&self, contract_id: &str) -> Result<()> {
        let path = format!(
            "/api/v1/providers/{}/contracts/{}/lock",
            self.provider_pubkey, contract_id
        );
        let response: ApiResponse<serde_json::Value> =
            self.request(Method::Delete, &path, None).await?;
        Self::unwrap_response(response, "Failed to release lock").map(|_| ())
    }

    /// Create a gateway DNS record via the central API.
    /// DNS record management is centralized — agents never get Cloudflare credentials.
    /// Returns the full subdomain (e.g., "k7m2p4.a3x9f2b1.dev-gw.decent-cloud.org").
    pub async fn create_dns_record(
        &self,
        slug: &str,
        dc_id: &str,
        public_ip: &str,
    ) -> Result<String> {
        let path = "/api/v1/agents/dns";
        let request = serde_json::json!({
            "action": "create",
            "slug": slug,
            "dcId": dc_id,
            "publicIp": public_ip
        });
        let body = serde_json::to_vec(&request)?;
        let response: ApiResponse<DnsResponse> =
            self.request(Method::Post, path, Some(&body)).await?;
        let data = Self::unwrap_response(response, "Failed to create DNS record")?;
        Ok(data.subdomain)
    }

    /// Delete a gateway DNS record via the central API.
    pub async fn delete_dns_record(&self, slug: &str, dc_id: &str) -> Result<String> {
        let path = "/api/v1/agents/dns";
        let request = serde_json::json!({
            "action": "delete",
            "slug": slug,
            "dcId": dc_id
        });
        let body = serde_json::to_vec(&request)?;
        let response: ApiResponse<DnsResponse> =
            self.request(Method::Post, path, Some(&body)).await?;
        let data = Self::unwrap_response(response, "Failed to delete DNS record")?;
        Ok(data.subdomain)
    }
}

/// Response from DNS management
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsResponse {
    pub subdomain: String,
}

/// Response from agent setup
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupResponse {
    pub provider_pubkey: String,
    pub pool_id: String,
    pub pool_name: String,
    pub pool_location: String,
    pub provisioner_type: String,
    pub permissions: Vec<String>,
}

/// Register agent using a setup token (unauthenticated).
pub async fn setup_agent(
    api_endpoint: &str,
    token: &str,
    agent_pubkey: &str,
) -> Result<SetupResponse> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to build HTTP client")?;

    let url = format!("{}/api/v1/agents/setup", api_endpoint);
    let body = serde_json::json!({
        "token": token,
        "agentPubkey": agent_pubkey
    });

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&body)?)
        .send()
        .await
        .context("Failed to send setup request")?;

    let status = response.status();
    let response_body = response
        .text()
        .await
        .context("Failed to read response body")?;

    if !status.is_success() {
        bail!("Setup failed (HTTP {}): {}", status, response_body);
    }

    let api_response: ApiResponse<SetupResponse> =
        serde_json::from_str(&response_body).context("Failed to parse setup response")?;

    if !api_response.success {
        bail!(
            "Setup failed: {}",
            api_response
                .error
                .unwrap_or_else(|| "Unknown error".to_string())
        );
    }

    api_response
        .data
        .ok_or_else(|| anyhow::anyhow!("Setup response missing data"))
}

/// acme-dns credentials returned from gateway registration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayRegistration {
    pub acme_dns_server_url: String,
    pub acme_dns_username: String,
    pub acme_dns_password: String,
    pub acme_dns_subdomain: String,
}

/// Register gateway for per-provider TLS via acme-dns.
///
/// Standalone function (like `setup_agent`) because we don't have a full `ApiConfig` during setup.
/// Uses agent key for authentication.
pub async fn register_gateway(
    api_endpoint: &str,
    agent_key_path: &str,
    dc_id: &str,
) -> Result<GatewayRegistration> {
    let identity = ApiClient::load_identity(agent_key_path)?;
    let agent_pubkey = hex::encode(identity.to_bytes_verifying());

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to build HTTP client")?;

    let path = "/api/v1/agents/gateway/register";
    let body = serde_json::json!({ "dcId": dc_id });
    let body_bytes = serde_json::to_vec(&body)?;

    // Build auth headers (same signing as ApiClient::build_auth_headers)
    let timestamp = chrono::Utc::now()
        .timestamp_nanos_opt()
        .ok_or_else(|| anyhow::anyhow!("Failed to get timestamp in nanoseconds"))?;
    let nonce = Uuid::new_v4();
    let timestamp_str = timestamp.to_string();
    let nonce_str = nonce.to_string();

    let mut sign_message = Vec::new();
    sign_message.extend_from_slice(timestamp_str.as_bytes());
    sign_message.extend_from_slice(nonce_str.as_bytes());
    sign_message.extend_from_slice(b"POST");
    sign_message.extend_from_slice(path.as_bytes());
    sign_message.extend_from_slice(&body_bytes);

    let signature = identity
        .sign(&sign_message)
        .map_err(|e| anyhow::anyhow!("Failed to sign gateway register request: {}", e))?;
    let signature_hex = hex::encode(signature.to_bytes());

    let url = format!("{}{}", api_endpoint, path);
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("X-Timestamp", &timestamp_str)
        .header("X-Nonce", &nonce_str)
        .header("X-Signature", &signature_hex)
        .header("X-Agent-Pubkey", &agent_pubkey)
        .body(body_bytes)
        .send()
        .await
        .context("Failed to send gateway register request")?;

    let status = response.status();
    let response_body = response
        .text()
        .await
        .context("Failed to read gateway register response")?;

    if !status.is_success() {
        bail!(
            "Gateway registration failed (HTTP {}): {}",
            status,
            response_body
        );
    }

    let api_response: ApiResponse<GatewayRegistration> = serde_json::from_str(&response_body)
        .context("Failed to parse gateway register response")?;

    if !api_response.success {
        bail!(
            "Gateway registration failed: {}",
            api_response
                .error
                .unwrap_or_else(|| "Unknown error".to_string())
        );
    }

    api_response
        .data
        .ok_or_else(|| anyhow::anyhow!("Gateway register response missing data"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_identity_from_hex() {
        let signing_key = SigningKey::from_bytes(&[42u8; 32]);
        let hex_key = hex::encode(signing_key.to_bytes());

        let identity = ApiClient::load_identity(&hex_key).unwrap();
        assert_eq!(
            identity.to_bytes_verifying(),
            signing_key.verifying_key().to_bytes()
        );
    }

    #[test]
    fn test_load_identity_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join("secret.key");

        let signing_key = SigningKey::from_bytes(&[99u8; 32]);
        let hex_key = hex::encode(signing_key.to_bytes());
        fs::write(&key_path, hex_key).unwrap();

        let identity = ApiClient::load_identity(key_path.to_str().unwrap()).unwrap();
        assert_eq!(
            identity.to_bytes_verifying(),
            signing_key.verifying_key().to_bytes()
        );
    }

    #[test]
    fn test_load_identity_from_file_with_whitespace() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join("secret.key");

        let signing_key = SigningKey::from_bytes(&[77u8; 32]);
        let hex_key = format!("  {}\n", hex::encode(signing_key.to_bytes()));
        fs::write(&key_path, hex_key).unwrap();

        let identity = ApiClient::load_identity(key_path.to_str().unwrap()).unwrap();
        assert_eq!(
            identity.to_bytes_verifying(),
            signing_key.verifying_key().to_bytes()
        );
    }

    #[test]
    fn test_load_identity_invalid_hex() {
        let result = ApiClient::load_identity("not_hex");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to read signing key from file"));
    }

    #[test]
    fn test_load_identity_wrong_length() {
        let short_key = hex::encode([1u8; 16]); // Only 16 bytes
        let result = ApiClient::load_identity(&short_key);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_auth_headers() {
        let signing_key = SigningKey::from_bytes(&[88u8; 32]);

        let config = ApiConfig {
            endpoint: "https://api.example.com".to_string(),
            provider_pubkey: "test_pubkey".to_string(),
            agent_secret_key: None,
            provider_secret_key: Some(hex::encode(signing_key.to_bytes())),
            pool_id: None,
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

        // Verify the signature using DccIdentity (prehashed + context)
        let mut message = Vec::new();
        message.extend_from_slice(timestamp_str.as_bytes());
        message.extend_from_slice(nonce_str.as_bytes());
        message.extend_from_slice(method.as_bytes());
        message.extend_from_slice(path.as_bytes());
        message.extend_from_slice(body);

        // Use DccIdentity for verification (matches server behavior)
        let verifier =
            DccIdentity::new_verifying_from_bytes(&signing_key.verifying_key().to_bytes()).unwrap();
        assert!(verifier.verify_bytes(&message, &signature_bytes).is_ok());
    }

    #[test]
    fn test_build_auth_headers_unique_nonce() {
        let signing_key = SigningKey::from_bytes(&[123u8; 32]);

        let config = ApiConfig {
            endpoint: "https://api.example.com".to_string(),
            provider_pubkey: "test_pubkey".to_string(),
            agent_secret_key: None,
            provider_secret_key: Some(hex::encode(signing_key.to_bytes())),
            pool_id: None,
        };

        let client = ApiClient::new(&config).unwrap();

        let (_, nonce1, _) = client.build_auth_headers("GET", "/path", b"").unwrap();
        let (_, nonce2, _) = client.build_auth_headers("GET", "/path", b"").unwrap();

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
            pool_id: None,
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
            pool_id: None,
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
            pool_id: None,
        };

        let result = ApiClient::new(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must be configured"));
    }

    #[test]
    fn test_unwrap_response_success() {
        let response = ApiResponse {
            success: true,
            data: Some("test_data".to_string()),
            error: None,
        };
        let result = ApiClient::unwrap_response(response, "test");
        assert_eq!(result.unwrap(), "test_data");
    }

    #[test]
    fn test_unwrap_response_failure() {
        let response: ApiResponse<String> = ApiResponse {
            success: false,
            data: None,
            error: Some("test error".to_string()),
        };
        let result = ApiClient::unwrap_response(response, "test context");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("test error"));
    }

    #[test]
    fn test_unwrap_response_no_data() {
        let response: ApiResponse<String> = ApiResponse {
            success: true,
            data: None,
            error: None,
        };
        let result = ApiClient::unwrap_response(response, "test context");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No data"));
    }

    #[test]
    fn test_parse_size_to_mb() {
        // GB formats
        assert_eq!(super::parse_size_to_mb(Some("16 GB")), Some(16 * 1024));
        assert_eq!(super::parse_size_to_mb(Some("16GB")), Some(16 * 1024));
        assert_eq!(super::parse_size_to_mb(Some("  16 GB  ")), Some(16 * 1024));

        // MB formats
        assert_eq!(super::parse_size_to_mb(Some("2048 MB")), Some(2048));
        assert_eq!(super::parse_size_to_mb(Some("2048MB")), Some(2048));

        // TB formats
        assert_eq!(super::parse_size_to_mb(Some("1 TB")), Some(1024 * 1024));
        assert_eq!(super::parse_size_to_mb(Some("1TB")), Some(1024 * 1024));

        // Invalid
        assert_eq!(super::parse_size_to_mb(None), None);
        assert_eq!(super::parse_size_to_mb(Some("")), None);
        assert_eq!(super::parse_size_to_mb(Some("invalid")), None);
    }

    #[test]
    fn test_parse_size_to_gb() {
        // GB formats
        assert_eq!(super::parse_size_to_gb(Some("100 GB")), Some(100));
        assert_eq!(super::parse_size_to_gb(Some("100GB")), Some(100));
        assert_eq!(super::parse_size_to_gb(Some("  500 GB  ")), Some(500));

        // TB formats
        assert_eq!(super::parse_size_to_gb(Some("1 TB")), Some(1024));
        assert_eq!(super::parse_size_to_gb(Some("2TB")), Some(2048));

        // Invalid
        assert_eq!(super::parse_size_to_gb(None), None);
        assert_eq!(super::parse_size_to_gb(Some("")), None);
        assert_eq!(super::parse_size_to_gb(Some("invalid")), None);
    }

    #[test]
    fn test_gateway_registration_deserialization() {
        let json = r#"{
            "acmeDnsServerUrl": "https://acme.decent-cloud.org",
            "acmeDnsUsername": "ebbcf5ce-4c3a-4f5a-b85e-0d2e2a68e8b0",
            "acmeDnsPassword": "htB9mR9DYgcu9bX_afHF62erPKmRNc",
            "acmeDnsSubdomain": "d420c923.acme.decent-cloud.org"
        }"#;

        let reg: GatewayRegistration = serde_json::from_str(json).unwrap();
        assert_eq!(reg.acme_dns_server_url, "https://acme.decent-cloud.org");
        assert_eq!(
            reg.acme_dns_username,
            "ebbcf5ce-4c3a-4f5a-b85e-0d2e2a68e8b0"
        );
        assert_eq!(reg.acme_dns_password, "htB9mR9DYgcu9bX_afHF62erPKmRNc");
        assert_eq!(
            reg.acme_dns_subdomain,
            "d420c923.acme.decent-cloud.org"
        );
    }

    #[test]
    fn test_pending_contract_specs() {
        let contract = PendingContract {
            contract_id: "test-contract-123".to_string(),
            offering_id: "offering-456".to_string(),
            requester_ssh_pubkey: "ssh-rsa AAAA...".to_string(),
            instance_config: None,
            cpu_cores: Some(4),
            memory_amount: Some("16 GB".to_string()),
            storage_capacity: Some("100 GB".to_string()),
            provisioner_type: None,
            provisioner_config: None,
            post_provision_script: None,
        };

        assert_eq!(contract.memory_mb(), Some(16 * 1024));
        assert_eq!(contract.storage_gb(), Some(100));
    }
}
