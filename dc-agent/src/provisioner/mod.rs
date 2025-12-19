use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod manual;
pub mod proxmox;
pub mod script;

/// Instance provisioned by the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub external_id: String,
    pub ip_address: Option<String>,
    pub ipv6_address: Option<String>,
    pub ssh_port: u16,
    pub root_password: Option<String>,
    pub additional_details: Option<serde_json::Value>,
}

/// Health status of a provisioned instance
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy { uptime_seconds: u64 },
    Unhealthy { reason: String },
    Unknown,
}

/// Contract requirements for provisioning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionRequest {
    pub contract_id: String,
    pub offering_id: String,
    pub cpu_cores: Option<u32>,
    pub memory_mb: Option<u32>,
    pub storage_gb: Option<u32>,
    pub requester_ssh_pubkey: Option<String>,
    pub instance_config: Option<serde_json::Value>,
}

/// Result of verifying provisioner setup
#[derive(Debug, Default)]
pub struct SetupVerification {
    pub api_reachable: Option<bool>,
    pub template_exists: Option<bool>,
    pub storage_accessible: Option<bool>,
    pub pool_exists: Option<bool>,
    pub errors: Vec<String>,
}

impl SetupVerification {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
            && self.api_reachable != Some(false)
            && self.template_exists != Some(false)
            && self.storage_accessible != Some(false)
            && self.pool_exists != Some(false)
    }
}

/// Provisioner trait - implement for each backend
#[async_trait]
pub trait Provisioner: Send + Sync {
    /// Provision a new instance
    async fn provision(&self, request: &ProvisionRequest) -> Result<Instance>;

    /// Terminate an instance
    async fn terminate(&self, external_id: &str) -> Result<()>;

    /// Check instance health
    async fn health_check(&self, external_id: &str) -> Result<HealthStatus>;

    /// Get instance details (for IP discovery after boot)
    async fn get_instance(&self, external_id: &str) -> Result<Option<Instance>>;

    /// Verify provisioner setup without creating resources
    async fn verify_setup(&self) -> SetupVerification {
        SetupVerification::default()
    }
}
