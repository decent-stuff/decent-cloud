//! Agent-related API endpoints.
//!
//! Handles agent delegations and heartbeats for provider provisioning agents.

use super::common::{check_authorization, decode_pubkey, ApiResponse};
use crate::auth::{AgentAuthenticatedUser, ApiAuthenticatedUser};
use crate::database::agent_delegations::CreateAgentDelegationParams;
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

        // Update heartbeat
        match db
            .update_agent_heartbeat(
                &provider_pubkey,
                req.version.as_deref(),
                req.provisioner_type.as_deref(),
                req.capabilities.as_deref(),
                req.active_contracts,
            )
            .await
        {
            Ok(()) => Json(ApiResponse {
                success: true,
                data: Some(HeartbeatResponse {
                    acknowledged: true,
                    next_heartbeat_seconds: 60,
                    pool_id: auth.pool_id,
                    pool_name,
                }),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }
}
