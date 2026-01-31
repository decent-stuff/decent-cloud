//! Agent-related API endpoints.
//!
//! Handles agent delegations, heartbeats, and DNS management for provider provisioning agents.

use super::common::{check_authorization, decode_pubkey, ApiResponse, RecordHealthCheckRequest};
use crate::auth::{AgentAuthenticatedUser, ApiAuthenticatedUser};
use crate::cloudflare_dns::CloudflareDns;
use crate::database::agent_delegations::CreateAgentDelegationParams;
use crate::database::contracts::ContractHealthCheck;
use crate::database::{AgentDelegation, AgentPermission, AgentStatus, Database};
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, Object, OpenApi};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use ts_rs::TS;

/// Request to register agent using a setup token
#[derive(Debug, Deserialize, Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AgentSetupRequest {
    /// One-time setup token from the provider
    pub token: String,
    /// Agent's public key (hex, 32 bytes)
    pub agent_pubkey: String,
}

/// Response from agent setup
#[derive(Debug, Serialize, Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AgentSetupResponse {
    /// Provider's public key (hex)
    pub provider_pubkey: String,
    /// Pool ID the agent was assigned to
    pub pool_id: String,
    /// Pool name
    pub pool_name: String,
    /// Pool location
    pub pool_location: String,
    /// Pool provisioner type
    pub provisioner_type: String,
    /// Permissions granted to the agent
    pub permissions: Vec<String>,
}

/// Bandwidth stats for a single VM
#[derive(Debug, Clone, Deserialize, Serialize, Object, TS)]
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

/// Storage pool information
#[derive(Debug, Clone, Deserialize, Serialize, Object, TS)]
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
#[derive(Debug, Clone, Deserialize, Serialize, Object, TS)]
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
#[derive(Debug, Clone, Deserialize, Serialize, Object, TS)]
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
#[derive(Debug, Clone, Deserialize, Serialize, Object, TS)]
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

/// Request for agent heartbeat
#[derive(Debug, Deserialize, Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatRequest {
    /// Agent version string
    pub version: Option<String>,
    /// Provisioner type (e.g., "proxmox", "hetzner")
    pub provisioner_type: Option<String>,
    /// Capabilities (e.g., ["vm", "health_check"])
    pub capabilities: Option<Vec<String>>,
    /// Number of active contracts being managed
    #[ts(type = "number")]
    pub active_contracts: i64,
    /// Per-VM bandwidth stats (optional, only if gateway is configured)
    pub bandwidth_stats: Option<Vec<VmBandwidthReport>>,
    /// Hardware resource inventory (optional, reported periodically)
    pub resources: Option<ResourceInventory>,
}

/// Request to update agent delegation label
#[derive(Debug, Deserialize, Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct UpdateLabelRequest {
    /// New label for the agent
    pub label: String,
}

/// Response for heartbeat
#[derive(Debug, Serialize, Object, TS)]
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
    pub pool_id: Option<String>,
    /// The agent's pool name, if it belongs to one
    #[oai(skip_serializing_if_is_none)]
    pub pool_name: Option<String>,
}

/// Request to manage gateway DNS records
#[derive(Debug, Deserialize, Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct GatewayDnsRequest {
    /// Action to perform: "create" or "delete"
    pub action: String,
    /// Gateway slug (6 alphanumeric chars)
    pub slug: String,
    /// Datacenter identifier (e.g., "dc-lk")
    pub datacenter: String,
    /// Public IP address (required for create, ignored for delete)
    #[oai(skip_serializing_if_is_none)]
    pub public_ip: Option<String>,
}

/// Response for gateway DNS operations
#[derive(Debug, Serialize, Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct GatewayDnsResponse {
    /// Full subdomain that was created/deleted
    pub subdomain: String,
}

/// API Tags for agent operations
#[derive(poem_openapi::Tags)]
enum ApiTags {
    /// Agent delegation and status operations
    Agents,
}

pub struct AgentsApi;

#[OpenApi]
impl AgentsApi {
    /// Register agent using setup token
    ///
    /// Unauthenticated endpoint for agents to register themselves using a one-time setup token.
    /// The token is consumed on successful registration and cannot be reused.
    #[oai(path = "/agents/setup", method = "post", tag = "ApiTags::Agents")]
    async fn setup_agent(
        &self,
        db: Data<&Arc<Database>>,
        req: Json<AgentSetupRequest>,
    ) -> Json<ApiResponse<AgentSetupResponse>> {
        // Decode agent pubkey
        let agent_pubkey = match decode_pubkey(&req.agent_pubkey) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid agent_pubkey: {}", e)),
                })
            }
        };

        // Validate and consume the setup token
        let (pool, label) = match db
            .validate_and_use_setup_token(&req.token, &agent_pubkey)
            .await
        {
            Ok(result) => result,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        // Get provider pubkey from pool
        let provider_pubkey = match hex::decode(&pool.provider_pubkey) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid provider pubkey in pool: {}", e)),
                })
            }
        };

        // Grant all standard permissions for pool-registered agents
        let permissions = AgentPermission::all();

        // Create the delegation with pool assignment
        // Note: For token-based setup, we use a placeholder signature since the token itself
        // proves the provider authorized this registration
        let placeholder_signature = vec![0u8; 64];
        if let Err(e) = db
            .create_agent_delegation(CreateAgentDelegationParams {
                provider_pubkey: &provider_pubkey,
                agent_pubkey: &agent_pubkey,
                permissions: &permissions,
                expires_at_ns: None, // No expiry for pool-registered agents
                label: label.as_deref(),
                signature: &placeholder_signature,
                pool_id: Some(&pool.pool_id),
            })
            .await
        {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to create delegation: {}", e)),
            });
        }

        Json(ApiResponse {
            success: true,
            data: Some(AgentSetupResponse {
                provider_pubkey: pool.provider_pubkey.clone(),
                pool_id: pool.pool_id.clone(),
                pool_name: pool.name.clone(),
                pool_location: pool.location.clone(),
                provisioner_type: pool.provisioner_type.clone(),
                permissions: permissions.iter().map(|p| p.as_str().to_string()).collect(),
            }),
            error: None,
        })
    }

    /// List agent delegations
    ///
    /// Returns all delegations for a provider, including revoked ones for audit purposes.
    #[oai(
        path = "/providers/:pubkey/agent-delegations",
        method = "get",
        tag = "ApiTags::Agents"
    )]
    async fn list_delegations(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<AgentDelegation>>> {
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&provider_pubkey, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db.list_agent_delegations(&provider_pubkey).await {
            Ok(delegations) => Json(ApiResponse {
                success: true,
                data: Some(delegations),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Revoke agent delegation
    ///
    /// Revokes an existing delegation. The agent will no longer be able to authenticate.
    #[oai(
        path = "/providers/:pubkey/agent-delegations/:agent_pubkey",
        method = "delete",
        tag = "ApiTags::Agents"
    )]
    async fn revoke_delegation(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        agent_pubkey: Path<String>,
    ) -> Json<ApiResponse<bool>> {
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&provider_pubkey, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        let agent_pk = match decode_pubkey(&agent_pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid agent_pubkey: {}", e)),
                })
            }
        };

        match db
            .revoke_agent_delegation(&provider_pubkey, &agent_pk)
            .await
        {
            Ok(revoked) => Json(ApiResponse {
                success: true,
                data: Some(revoked),
                error: if revoked {
                    None
                } else {
                    Some("No active delegation found to revoke".to_string())
                },
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update agent delegation label
    ///
    /// Updates the label for an existing agent delegation.
    #[oai(
        path = "/providers/:pubkey/agent-delegations/:agent_pubkey/label",
        method = "put",
        tag = "ApiTags::Agents"
    )]
    async fn update_delegation_label(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        agent_pubkey: Path<String>,
        Json(payload): Json<UpdateLabelRequest>,
    ) -> Json<ApiResponse<bool>> {
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&provider_pubkey, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        let agent_pk = match decode_pubkey(&agent_pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid agent_pubkey: {}", e)),
                })
            }
        };

        match db
            .update_agent_delegation_label(&provider_pubkey, &agent_pk, &payload.label)
            .await
        {
            Ok(updated) => Json(ApiResponse {
                success: true,
                data: Some(updated),
                error: if updated {
                    None
                } else {
                    Some("No delegation found to update".to_string())
                },
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get agent status
    ///
    /// Returns the current status of a provider's agent (online/offline, last heartbeat, etc.)
    #[oai(
        path = "/providers/:pubkey/agent-status",
        method = "get",
        tag = "ApiTags::Agents"
    )]
    async fn get_agent_status(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<AgentStatus>> {
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        match db.get_agent_status(&provider_pubkey).await {
            Ok(Some(status)) => Json(ApiResponse {
                success: true,
                data: Some(status),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("No agent status found for this provider".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Send agent heartbeat
    ///
    /// Called by agents to report their status. Requires agent authentication.
    #[oai(
        path = "/providers/:pubkey/heartbeat",
        method = "post",
        tag = "ApiTags::Agents"
    )]
    async fn send_heartbeat(
        &self,
        db: Data<&Arc<Database>>,
        auth: AgentAuthenticatedUser,
        pubkey: Path<String>,
        req: Json<HeartbeatRequest>,
    ) -> Json<ApiResponse<HeartbeatResponse>> {
        // Verify the pubkey matches the agent's delegated provider
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if provider_pubkey != auth.provider_pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Agent is not delegated by this provider".to_string()),
            });
        }

        // Check permission
        if let Err(e) = auth.require_permission(AgentPermission::Heartbeat) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
        }

        // Fetch pool info if agent belongs to a pool
        let pool_name = if let Some(pool_id) = &auth.pool_id {
            match db.get_agent_pool(pool_id).await {
                Ok(Some(pool)) => Some(pool.name),
                // Log error but don't fail heartbeat if pool lookup fails
                _ => None,
            }
        } else {
            None
        };

        // Convert resources to JSON value if provided
        let resources_json = req
            .resources
            .as_ref()
            .and_then(|r| serde_json::to_value(r).ok());

        // Update heartbeat
        if let Err(e) = db
            .update_agent_heartbeat(
                &provider_pubkey,
                req.version.as_deref(),
                req.provisioner_type.as_deref(),
                req.capabilities.as_deref(),
                req.active_contracts,
                resources_json.as_ref(),
            )
            .await
        {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
        }

        // Record bandwidth stats if provided
        if let Some(ref stats) = req.bandwidth_stats {
            for stat in stats {
                if let Err(e) = db
                    .record_bandwidth(
                        &stat.contract_id,
                        &stat.gateway_slug,
                        &pubkey.0,
                        stat.bytes_in,
                        stat.bytes_out,
                    )
                    .await
                {
                    tracing::warn!(
                        contract_id = %stat.contract_id,
                        error = %e,
                        "Failed to record bandwidth stats"
                    );
                    // Don't fail heartbeat for bandwidth recording errors
                }
            }
        }

        Json(ApiResponse {
            success: true,
            data: Some(HeartbeatResponse {
                acknowledged: true,
                next_heartbeat_seconds: 60,
                pool_id: auth.pool_id,
                pool_name,
            }),
            error: None,
        })
    }

    /// Record contract health check
    ///
    /// Called by dc-agent to report the health status of a provisioned contract.
    /// Requires agent authentication with HealthCheck permission.
    /// The agent must be delegated by the contract's provider.
    #[oai(
        path = "/contracts/:id/health",
        method = "post",
        tag = "ApiTags::Agents"
    )]
    async fn record_health_check(
        &self,
        db: Data<&Arc<Database>>,
        auth: AgentAuthenticatedUser,
        id: Path<String>,
        req: Json<RecordHealthCheckRequest>,
    ) -> Json<ApiResponse<ContractHealthCheck>> {
        // Check permission
        if let Err(e) = auth.require_permission(AgentPermission::HealthCheck) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
        }

        // Decode contract ID
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                });
            }
        };

        // Verify contract exists and agent is authorized (provider match)
        let contract = match db.get_contract(&contract_id).await {
            Ok(Some(c)) => c,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Contract not found: {}", id.0)),
                });
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to retrieve contract: {}", e)),
                });
            }
        };

        // Authorization: agent must be delegated by the contract's provider
        let provider_pubkey_hex = hex::encode(&auth.provider_pubkey);
        if contract.provider_pubkey != provider_pubkey_hex {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!(
                    "Unauthorized: agent's provider ({}) does not match contract provider ({})",
                    provider_pubkey_hex, contract.provider_pubkey
                )),
            });
        }

        // Record the health check
        let check_id = match db
            .record_health_check(
                &contract_id,
                req.checked_at,
                &req.status,
                req.latency_ms,
                req.details.as_deref(),
            )
            .await
        {
            Ok(id) => id,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                });
            }
        };

        // Return the created health check record
        let health_check = ContractHealthCheck {
            id: check_id,
            contract_id: id.0.clone(),
            checked_at: req.checked_at,
            status: req.status.clone(),
            latency_ms: req.latency_ms,
            details: req.details.clone(),
            created_at: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        };

        Json(ApiResponse {
            success: true,
            data: Some(health_check),
            error: None,
        })
    }

    /// Manage gateway DNS records
    ///
    /// Creates or deletes DNS A records for gateway subdomains.
    /// This proxies DNS management through the API so agents don't need Cloudflare credentials.
    /// Requires agent authentication with DnsManage permission.
    #[oai(path = "/agents/dns", method = "post", tag = "ApiTags::Agents")]
    async fn manage_gateway_dns(
        &self,
        cloudflare: Data<&Option<Arc<CloudflareDns>>>,
        auth: AgentAuthenticatedUser,
        req: Json<GatewayDnsRequest>,
    ) -> Json<ApiResponse<GatewayDnsResponse>> {
        // Check permission
        if let Err(e) = auth.require_permission(AgentPermission::DnsManage) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
        }

        // Verify Cloudflare is configured
        let cf = match cloudflare.as_ref() {
            Some(cf) => cf,
            None => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("DNS management not configured on server".to_string()),
                });
            }
        };

        // Validate slug format (6 lowercase alphanumeric)
        if req.slug.len() != 6
            || !req
                .slug
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(
                    "Invalid slug: must be 6 lowercase alphanumeric characters".to_string(),
                ),
            });
        }

        // Validate datacenter
        if req.datacenter.is_empty() || req.datacenter.len() > 20 {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Invalid datacenter: must be 1-20 characters".to_string()),
            });
        }

        let subdomain = format!("{}.{}.{}", req.slug, req.datacenter, cf.domain());

        match req.action.as_str() {
            "create" => {
                let public_ip = match &req.public_ip {
                    Some(ip) => ip,
                    None => {
                        return Json(ApiResponse {
                            success: false,
                            data: None,
                            error: Some("public_ip required for create action".to_string()),
                        });
                    }
                };

                match cf
                    .create_gateway_record(&req.slug, &req.datacenter, public_ip)
                    .await
                {
                    Ok(()) => Json(ApiResponse {
                        success: true,
                        data: Some(GatewayDnsResponse { subdomain }),
                        error: None,
                    }),
                    Err(e) => Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to create DNS record: {}", e)),
                    }),
                }
            }
            "delete" => match cf.delete_gateway_record(&req.slug, &req.datacenter).await {
                Ok(()) => Json(ApiResponse {
                    success: true,
                    data: Some(GatewayDnsResponse { subdomain }),
                    error: None,
                }),
                Err(e) => Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to delete DNS record: {}", e)),
                }),
            },
            _ => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Invalid action: must be 'create' or 'delete'".to_string()),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_dns_request_deserialization() {
        let json = r#"{
            "action": "create",
            "slug": "k7m2p4",
            "datacenter": "dc-lk",
            "publicIp": "203.0.113.1"
        }"#;

        let request: GatewayDnsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.action, "create");
        assert_eq!(request.slug, "k7m2p4");
        assert_eq!(request.datacenter, "dc-lk");
        assert_eq!(request.public_ip, Some("203.0.113.1".to_string()));
    }

    #[test]
    fn test_gateway_dns_request_delete_no_ip() {
        let json = r#"{
            "action": "delete",
            "slug": "k7m2p4",
            "datacenter": "dc-lk"
        }"#;

        let request: GatewayDnsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.action, "delete");
        assert_eq!(request.public_ip, None);
    }

    #[test]
    fn test_gateway_dns_response_serialization() {
        let response = GatewayDnsResponse {
            subdomain: "k7m2p4.dc-lk.decent-cloud.org".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("k7m2p4.dc-lk.decent-cloud.org"));
    }

    #[test]
    fn test_health_check_request_deserialization() {
        let json = r#"{
            "checkedAt": 1700000000000000000,
            "status": "healthy",
            "latencyMs": 42,
            "details": "{\"ssh\":\"ok\"}"
        }"#;

        let request: RecordHealthCheckRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.checked_at, 1700000000000000000);
        assert_eq!(request.status, "healthy");
        assert_eq!(request.latency_ms, Some(42));
        assert_eq!(request.details, Some(r#"{"ssh":"ok"}"#.to_string()));
    }

    #[test]
    fn test_health_check_request_minimal() {
        let json = r#"{
            "checkedAt": 1700000000000000000,
            "status": "unhealthy"
        }"#;

        let request: RecordHealthCheckRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.checked_at, 1700000000000000000);
        assert_eq!(request.status, "unhealthy");
        assert_eq!(request.latency_ms, None);
        assert_eq!(request.details, None);
    }

    #[test]
    fn test_contract_health_check_serialization() {
        let check = ContractHealthCheck {
            id: 1,
            contract_id: "abc123".to_string(),
            checked_at: 1700000000000000000,
            status: "healthy".to_string(),
            latency_ms: Some(25),
            details: Some(r#"{"port":22}"#.to_string()),
            created_at: 1700000000000000000,
        };

        let json = serde_json::to_string(&check).unwrap();
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"status\":\"healthy\""));
        assert!(json.contains("\"latencyMs\":25"));
    }
}
