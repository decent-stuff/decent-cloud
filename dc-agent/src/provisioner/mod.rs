use crate::api_client::ResourceInventory;
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
    /// Gateway slug (6-char alphanumeric identifier for subdomain)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_slug: Option<String>,
    /// Full gateway subdomain (e.g., "k7m2p4.dc-lk.decent-cloud.org")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_subdomain: Option<String>,
    /// SSH port accessible via gateway (external port)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_ssh_port: Option<u16>,
    /// Start of allocated port range
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_port_range_start: Option<u16>,
    /// End of allocated port range
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_port_range_end: Option<u16>,
}

/// Running instance for reconciliation reporting
#[derive(Debug, Clone)]
pub struct RunningInstance {
    /// External ID of the VM (e.g., Proxmox VMID)
    pub external_id: String,
    /// Contract ID extracted from VM name (dc-{contract_id})
    pub contract_id: Option<String>,
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
    /// Script to execute via SSH after VM provisioning (uses shebang for interpreter)
    pub post_provision_script: Option<String>,
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

    /// List all running instances managed by this agent.
    /// Used for reconciliation to detect expired/cancelled contracts.
    async fn list_running_instances(&self) -> Result<Vec<RunningInstance>> {
        // Default: return empty list (legacy provisioners)
        Ok(vec![])
    }

    /// Verify provisioner setup without creating resources
    async fn verify_setup(&self) -> SetupVerification {
        SetupVerification::default()
    }

    /// Collect hardware resource inventory.
    /// Returns None if not supported or collection fails.
    async fn collect_resources(&self) -> Option<ResourceInventory> {
        // Default: not supported
        None
    }
}
