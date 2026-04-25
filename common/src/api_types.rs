//! Shared API types for communication between api-server and dc-agent.
//!
//! These types are the canonical definitions used by both crates.

use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ============================================================================
// Reconciliation Types
// ============================================================================

/// Instance that should continue running
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ReconcileKeepInstance {
    pub external_id: String,
    pub contract_id: String,
    /// When this contract ends (nanoseconds since epoch)
    pub ends_at: i64,
}

/// Instance that should be terminated
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ReconcileTerminateInstance {
    pub external_id: String,
    pub contract_id: String,
    /// Reason for termination: "expired", "cancelled"
    pub reason: String,
}

/// Instance whose VM must be stopped (NOT destroyed) while the contract is
/// paused. Stripe dispute handling parks the contract in `paused` while the
/// dispute is open; resuming the contract returns it to `active` and the
/// next provisioning poll restarts the VM.
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ReconcilePauseInstance {
    pub external_id: String,
    pub contract_id: String,
    /// Reason supplied by the API: e.g. "stripe_dispute:<dispute_id>".
    pub reason: String,
}

/// Unknown instance (orphan - no matching contract)
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ReconcileUnknownInstance {
    pub external_id: String,
    pub message: String,
}

/// Response for reconciliation request
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ReconcileResponse {
    /// Instances that should continue running
    pub keep: Vec<ReconcileKeepInstance>,
    /// Instances that should be terminated
    pub terminate: Vec<ReconcileTerminateInstance>,
    /// Instances with no matching contract (orphans)
    pub unknown: Vec<ReconcileUnknownInstance>,
    /// Instances whose VM should be stopped (NOT destroyed) for a paused
    /// contract. Empty for older API servers (`#[serde(default)]`).
    #[serde(default)]
    pub pause: Vec<ReconcilePauseInstance>,
}

// ============================================================================
// Agent Resource Types
// ============================================================================

/// Storage pool information
#[derive(Debug, Clone, Serialize, Deserialize, Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct StoragePoolInfo {
    /// Storage pool name (e.g., "local-lvm")
    pub name: String,
    /// Total capacity in GB
    #[ts(type = "number")]
    pub total_gb: u64,
    /// Available capacity in GB
    #[ts(type = "number")]
    pub available_gb: u64,
    /// Storage type (e.g., "lvmthin", "zfspool", "dir")
    pub storage_type: String,
}

/// GPU device information
#[derive(Debug, Clone, Serialize, Deserialize, Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct GpuDeviceInfo {
    /// PCI device ID (e.g., "0000:01:00.0")
    pub pci_id: String,
    /// Device name (e.g., "NVIDIA GeForce RTX 4090")
    pub name: String,
    /// Vendor name (e.g., "NVIDIA Corporation")
    pub vendor: String,
    /// VRAM in MB (if detectable)
    #[ts(type = "number | undefined")]
    pub memory_mb: Option<u32>,
}

/// VM template information
#[derive(Debug, Clone, Serialize, Deserialize, Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct TemplateInfo {
    /// Template VM ID
    pub vmid: u32,
    /// Template name (e.g., "ubuntu-22.04")
    pub name: String,
}

/// Hardware resource inventory reported by agent
#[derive(Debug, Clone, Serialize, Deserialize, Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ResourceInventory {
    /// CPU model name (e.g., "AMD EPYC 7763 64-Core Processor")
    pub cpu_model: Option<String>,
    /// Number of physical CPU cores
    pub cpu_cores: u32,
    /// Number of logical CPU threads
    pub cpu_threads: u32,
    /// CPU clock speed in MHz
    pub cpu_mhz: Option<u32>,
    /// Total RAM in MB
    #[ts(type = "number")]
    pub memory_total_mb: u64,
    /// Available (uncommitted) RAM in MB
    #[ts(type = "number")]
    pub memory_available_mb: u64,
    /// Storage pools with capacity info
    #[serde(default)]
    pub storage_pools: Vec<StoragePoolInfo>,
    /// GPU devices available for passthrough
    #[serde(default)]
    pub gpu_devices: Vec<GpuDeviceInfo>,
    /// VM templates available for provisioning
    #[serde(default)]
    pub templates: Vec<TemplateInfo>,
}

// ============================================================================
// Bandwidth Types
// ============================================================================

/// Bandwidth stats for a single VM
#[derive(Debug, Clone, Serialize, Deserialize, Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct VmBandwidthReport {
    /// Gateway slug (6-char identifier)
    pub gateway_slug: String,
    /// Contract ID this VM belongs to
    pub contract_id: String,
    /// Bytes received by the VM since last reset
    #[ts(type = "number")]
    pub bytes_in: u64,
    /// Bytes sent by the VM since last reset
    #[ts(type = "number")]
    pub bytes_out: u64,
}

// ============================================================================
// Heartbeat Types
// ============================================================================

/// Response for heartbeat
#[derive(Debug, Clone, Serialize, Deserialize, Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatResponse {
    /// Whether heartbeat was acknowledged
    pub acknowledged: bool,
    /// Recommended seconds until next heartbeat
    #[ts(type = "number")]
    pub next_heartbeat_seconds: i64,
    /// The agent's pool ID, if it belongs to one
    #[oai(skip_serializing_if_is_none)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_id: Option<String>,
    /// The agent's pool name, if it belongs to one
    #[oai(skip_serializing_if_is_none)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_name: Option<String>,
    /// Version to upgrade to, if an upgrade has been requested for this pool
    #[oai(skip_serializing_if_is_none)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upgrade_to_version: Option<String>,
}

// ============================================================================
// Lock Types
// ============================================================================

/// Response for provisioning lock acquisition
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct LockResponse {
    /// Whether the lock was acquired
    pub acquired: bool,
    /// Lock expiration timestamp (nanoseconds). Always present when acquired=true.
    pub expires_at_ns: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reconcile_response_serialization() {
        let response = ReconcileResponse {
            keep: vec![ReconcileKeepInstance {
                external_id: "vm-123".to_string(),
                contract_id: "contract-456".to_string(),
                ends_at: 1234567890,
            }],
            terminate: vec![],
            unknown: vec![],
            pause: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("externalId"));
        assert!(json.contains("contractId"));
        assert!(json.contains("endsAt"));

        let parsed: ReconcileResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.keep.len(), 1);
        assert_eq!(parsed.keep[0].external_id, "vm-123");
    }

    #[test]
    fn test_reconcile_response_pause_field_serializes_and_default_deserializes() {
        // pause must round-trip cleanly when populated...
        let with_pause = ReconcileResponse {
            keep: vec![],
            terminate: vec![],
            unknown: vec![],
            pause: vec![ReconcilePauseInstance {
                external_id: "vm-9".to_string(),
                contract_id: "c-9".to_string(),
                reason: "stripe_dispute:du_x".to_string(),
            }],
        };
        let json = serde_json::to_string(&with_pause).unwrap();
        assert!(json.contains("\"pause\""));
        let parsed: ReconcileResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.pause.len(), 1);
        assert_eq!(parsed.pause[0].reason, "stripe_dispute:du_x");

        // ...and `pause` MUST default to empty when an older server omits it,
        // so a freshly-deployed agent never panics against an old API server.
        let legacy = r#"{"keep":[],"terminate":[],"unknown":[]}"#;
        let parsed: ReconcileResponse = serde_json::from_str(legacy).unwrap();
        assert!(parsed.pause.is_empty(), "pause must default to empty");
    }

    #[test]
    fn test_heartbeat_response_serialization() {
        let response = HeartbeatResponse {
            acknowledged: true,
            next_heartbeat_seconds: 60,
            pool_id: Some("pool-1".to_string()),
            pool_name: None,
            upgrade_to_version: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("acknowledged"));
        assert!(json.contains("nextHeartbeatSeconds"));
        assert!(json.contains("poolId"));
        // pool_name should be skipped when None
        assert!(!json.contains("poolName"));
        // upgrade_to_version should be skipped when None
        assert!(!json.contains("upgradeToVersion"));

        let parsed: HeartbeatResponse = serde_json::from_str(&json).unwrap();
        assert!(parsed.acknowledged);
        assert_eq!(parsed.pool_id, Some("pool-1".to_string()));
    }

    #[test]
    fn test_heartbeat_response_with_upgrade_version() {
        let response = HeartbeatResponse {
            acknowledged: true,
            next_heartbeat_seconds: 30,
            pool_id: Some("pool-1".to_string()),
            pool_name: Some("EU Pool".to_string()),
            upgrade_to_version: Some("0.4.21".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("upgradeToVersion"));
        assert!(json.contains("0.4.21"));

        let parsed: HeartbeatResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.upgrade_to_version.as_deref(), Some("0.4.21"));
    }

    #[test]
    fn test_lock_response_serialization() {
        let response = LockResponse {
            acquired: true,
            expires_at_ns: 1234567890123456789,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("acquired"));
        assert!(json.contains("expiresAtNs"));

        let parsed: LockResponse = serde_json::from_str(&json).unwrap();
        assert!(parsed.acquired);
        assert_eq!(parsed.expires_at_ns, 1234567890123456789);
    }

    #[test]
    fn test_resource_inventory_defaults() {
        // Test that serde defaults work for optional Vec fields
        let json = r#"{
            "cpuModel": "Test CPU",
            "cpuCores": 4,
            "cpuThreads": 8,
            "cpuMhz": 3000,
            "memoryTotalMb": 16384,
            "memoryAvailableMb": 8192
        }"#;

        let inventory: ResourceInventory = serde_json::from_str(json).unwrap();
        assert!(inventory.storage_pools.is_empty());
        assert!(inventory.gpu_devices.is_empty());
        assert!(inventory.templates.is_empty());
    }
}
