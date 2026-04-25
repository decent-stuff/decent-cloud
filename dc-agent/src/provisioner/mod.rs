use crate::api_client::ResourceInventory;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod digitalocean;
pub mod docker;
pub mod manual;
pub mod proxmox;
pub mod script;

pub fn extract_contract_id(name: &str) -> Option<String> {
    name.strip_prefix("dc-").map(String::from)
}

/// Instance provisioned by the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub external_id: String,
    pub ip_address: Option<String>,
    pub ipv6_address: Option<String>,
    /// Public IP of the gateway host (for Proxmox VMs behind NAT)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_ip: Option<String>,
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
    pub warnings: Vec<String>,
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

    /// Stop (suspend) an instance WITHOUT destroying its disk or DNS state.
    /// Used by the reconcile loop when a contract is paused (e.g. due to a
    /// Stripe dispute) and we expect to resume it later.
    ///
    /// Default: log a clear warning and fall through to `terminate`. A
    /// provisioner that distinguishes stop-vs-delete (e.g. Proxmox) should
    /// override this; provisioners that cannot stop without destroying are
    /// effectively unable to honor pause-and-resume, and the warning makes
    /// that visible at runtime instead of silently consuming the customer's
    /// state.
    async fn stop(&self, external_id: &str) -> Result<()> {
        tracing::warn!(
            external_id,
            "Provisioner does not implement stop(); destroying VM instead. \
             Pause-and-resume will lose customer state on this provisioner."
        );
        self.terminate(external_id).await
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// A test-only Provisioner that records terminate/stop calls. Used to
    /// assert that the trait's default `stop()` falls through to `terminate()`
    /// (so non-Proxmox provisioners still respond to a pause reconcile, even
    /// if they cannot preserve state). Phase 2: dc-agent runtime change.
    struct RecordingProvisioner {
        terminates: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Provisioner for RecordingProvisioner {
        async fn provision(&self, _request: &ProvisionRequest) -> Result<Instance> {
            unreachable!("not used in stop-fallback test")
        }
        async fn terminate(&self, _external_id: &str) -> Result<()> {
            self.terminates.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        async fn health_check(&self, _external_id: &str) -> Result<HealthStatus> {
            unreachable!("not used in stop-fallback test")
        }
        async fn get_instance(&self, _external_id: &str) -> Result<Option<Instance>> {
            Ok(None)
        }
    }

    /// A Provisioner that overrides `stop()` -- mirrors the Proxmox
    /// implementation's contract: stop MUST NOT delete the disk/VM.
    struct StopAwareProvisioner {
        terminates: Arc<AtomicUsize>,
        stops: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Provisioner for StopAwareProvisioner {
        async fn provision(&self, _request: &ProvisionRequest) -> Result<Instance> {
            unreachable!()
        }
        async fn terminate(&self, _external_id: &str) -> Result<()> {
            self.terminates.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        async fn stop(&self, _external_id: &str) -> Result<()> {
            self.stops.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        async fn health_check(&self, _external_id: &str) -> Result<HealthStatus> {
            unreachable!()
        }
        async fn get_instance(&self, _external_id: &str) -> Result<Option<Instance>> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_paused_contract_dispatch_routes_to_stop_when_supported() {
        // The reconcile pause loop calls `provisioner.stop(...)`. A
        // Provisioner that implements stop natively MUST NOT touch terminate;
        // a Provisioner that only implements terminate (default) MUST get
        // the call routed to terminate (with a warn-log) so dispute handling
        // is still effective at the cost of state loss.
        let stop_aware = StopAwareProvisioner {
            terminates: Arc::new(AtomicUsize::new(0)),
            stops: Arc::new(AtomicUsize::new(0)),
        };
        stop_aware.stop("vm-1").await.unwrap();
        assert_eq!(stop_aware.stops.load(Ordering::SeqCst), 1);
        assert_eq!(
            stop_aware.terminates.load(Ordering::SeqCst),
            0,
            "stop-aware provisioner MUST NOT destroy on pause"
        );

        let recording = RecordingProvisioner {
            terminates: Arc::new(AtomicUsize::new(0)),
        };
        recording.stop("vm-2").await.unwrap();
        assert_eq!(
            recording.terminates.load(Ordering::SeqCst),
            1,
            "default stop() MUST fall through to terminate()"
        );
    }

    #[test]
    fn test_instance_serialization_includes_public_ip_when_set() {
        let instance = Instance {
            external_id: "100".to_string(),
            ip_address: Some("10.0.0.5".to_string()),
            ipv6_address: None,
            public_ip: Some("203.0.113.1".to_string()),
            ssh_port: 22,
            root_password: None,
            additional_details: None,
            gateway_slug: Some("abc123".to_string()),
            gateway_subdomain: Some("abc123.dc-lk.gw.decent-cloud.org".to_string()),
            gateway_ssh_port: Some(20000),
            gateway_port_range_start: Some(20000),
            gateway_port_range_end: Some(20009),
        };

        let json = serde_json::to_value(&instance).unwrap();
        assert_eq!(json["public_ip"], "203.0.113.1");
    }

    #[test]
    fn test_instance_serialization_omits_public_ip_when_none() {
        let instance = Instance {
            external_id: "100".to_string(),
            ip_address: Some("10.0.0.5".to_string()),
            ipv6_address: None,
            public_ip: None,
            ssh_port: 22,
            root_password: None,
            additional_details: None,
            gateway_slug: None,
            gateway_subdomain: None,
            gateway_ssh_port: None,
            gateway_port_range_start: None,
            gateway_port_range_end: None,
        };

        let json = serde_json::to_value(&instance).unwrap();
        assert!(
            json.get("public_ip").is_none(),
            "public_ip should be omitted when None"
        );
    }

    #[test]
    fn test_instance_deserialization_without_public_ip_field() {
        let json = r#"{
            "external_id": "vm-123",
            "ip_address": "10.0.0.100",
            "ipv6_address": null,
            "ssh_port": 22,
            "root_password": null,
            "additional_details": null
        }"#;

        let instance: Instance = serde_json::from_str(json).unwrap();
        assert!(
            instance.public_ip.is_none(),
            "public_ip should default to None when absent from JSON"
        );
    }

    #[test]
    fn test_extract_contract_id() {
        assert_eq!(extract_contract_id("dc-abc123"), Some("abc123".to_string()));
        assert_eq!(
            extract_contract_id("dc-test-contract"),
            Some("test-contract".to_string())
        );
        assert_eq!(
            extract_contract_id("dc-contract-456"),
            Some("contract-456".to_string())
        );
        assert_eq!(
            extract_contract_id("dc-test-contract-123"),
            Some("test-contract-123".to_string())
        );
        assert_eq!(extract_contract_id("other-name"), None);
        assert_eq!(extract_contract_id("dc-"), Some("".to_string()));
    }
}
