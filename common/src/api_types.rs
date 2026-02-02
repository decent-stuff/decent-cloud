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
    fn test_heartbeat_response_serialization() {
        let response = HeartbeatResponse {
            acknowledged: true,
            next_heartbeat_seconds: 60,
            pool_id: Some("pool-1".to_string()),
            pool_name: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("acknowledged"));
        assert!(json.contains("nextHeartbeatSeconds"));
        assert!(json.contains("poolId"));
        // pool_name should be skipped when None
        assert!(!json.contains("poolName"));

        let parsed: HeartbeatResponse = serde_json::from_str(&json).unwrap();
        assert!(parsed.acknowledged);
        assert_eq!(parsed.pool_id, Some("pool-1".to_string()));
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
