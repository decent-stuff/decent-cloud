//! Agent pool, setup-token, and distributed-lock endpoints for providers.
//!
//! Extracted from `providers.rs` (#444 large-file split). These handlers all
//! carry the `ApiTags::Pools` tag and form a self-contained cluster with no
//! dependency on private helpers or local types defined in `providers.rs`.
//! Registration is unchanged from the consumer's perspective: `PoolsApi` is
//! combined with the other `*Api` types in `openapi::create_combined_api`, and
//! every path, method, tag, and schema below is identical to the pre-split API.

use super::common::{
    check_authorization, decode_hex_path, decode_pubkey, ApiResponse, ApiTags, CreatePoolRequest,
    CreateSetupTokenRequest, LockResponse, PoolUpgradeRequest, UpdatePoolRequest,
};
use crate::auth::{AgentAuthenticatedUser, ApiAuthenticatedUser};
use crate::database::{AgentPoolWithStats, Database, SetupToken};
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, OpenApi};
use std::sync::Arc;

pub struct PoolsApi;

#[OpenApi]
impl PoolsApi {
    // ==================== Agent Pool Endpoints ====================

    /// Create agent pool
    ///
    /// Creates a new agent pool for grouping provisioning agents by location and type.
    #[oai(
        path = "/providers/:pubkey/pools",
        method = "post",
        tag = "ApiTags::Pools"
    )]
    async fn create_pool(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        req: Json<CreatePoolRequest>,
    ) -> Json<ApiResponse<crate::database::AgentPool>> {
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

        // Generate a unique pool_id from name (sanitized)
        let pool_id = format!(
            "{}-{}",
            req.name
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-')
                .collect::<String>()
                .to_lowercase(),
            &uuid::Uuid::new_v4().to_string()[..8]
        );

        match db
            .create_agent_pool(
                &pool_id,
                &provider_pubkey,
                &req.name,
                &req.location,
                &req.provisioner_type,
            )
            .await
        {
            Ok(pool) => Json(ApiResponse {
                success: true,
                data: Some(pool),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// List agent pools
    ///
    /// Returns all agent pools for a provider with statistics.
    #[oai(
        path = "/providers/:pubkey/pools",
        method = "get",
        tag = "ApiTags::Pools"
    )]
    async fn list_pools(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<AgentPoolWithStats>>> {
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

        match db.list_agent_pools_with_stats(&provider_pubkey).await {
            Ok(pools) => Json(ApiResponse {
                success: true,
                data: Some(pools),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get agent pool details
    ///
    /// Returns details and statistics for a specific agent pool.
    #[oai(
        path = "/providers/:pubkey/pools/:pool_id",
        method = "get",
        tag = "ApiTags::Pools"
    )]
    async fn get_pool_details(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        pool_id: Path<String>,
    ) -> Json<ApiResponse<AgentPoolWithStats>> {
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

        // This is not the most efficient way, but list_agent_pools_with_stats is what we have.
        // A dedicated get_pool_with_stats(pool_id) would be better.
        match db.list_agent_pools_with_stats(&provider_pubkey).await {
            Ok(pools) => {
                if let Some(pool) = pools.into_iter().find(|p| p.pool.pool_id == pool_id.0) {
                    Json(ApiResponse {
                        success: true,
                        data: Some(pool),
                        error: None,
                    })
                } else {
                    Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(
                            "Pool not found or does not belong to this provider".to_string(),
                        ),
                    })
                }
            }
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// List agents in a pool
    ///
    /// Returns all active agent delegations for a specific pool.
    #[oai(
        path = "/providers/:pubkey/pools/:pool_id/agents",
        method = "get",
        tag = "ApiTags::Pools"
    )]
    async fn list_agents_in_pool(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        pool_id: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::agent_delegations::AgentDelegation>>> {
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

        // Optional: Check if pool belongs to this provider
        match db.get_agent_pool(&pool_id.0).await {
            Ok(Some(pool)) => {
                let pool_pubkey = match hex::decode(&pool.provider_pubkey) {
                    Ok(pk) => pk,
                    Err(e) => {
                        tracing::warn!("Malformed hex in pool.provider_pubkey: {:#}", e);
                        return Json(ApiResponse {
                            success: false,
                            data: None,
                            error: Some("Invalid pubkey format in database".to_string()),
                        });
                    }
                };
                if pool_pubkey != provider_pubkey {
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some("Pool does not belong to this provider".to_string()),
                    });
                }
            }
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Pool not found".to_string()),
                });
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                });
            }
        }

        match db.list_agents_in_pool(&pool_id.0).await {
            Ok(agents) => Json(ApiResponse {
                success: true,
                data: Some(agents),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get agent pool
    ///
    /// Returns details for a specific agent pool.
    #[oai(
        path = "/providers/:pubkey/pools/:pool_id",
        method = "get",
        tag = "ApiTags::Pools"
    )]
    async fn get_pool(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        pool_id: Path<String>,
    ) -> Json<ApiResponse<crate::database::AgentPool>> {
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

        match db.get_agent_pool(&pool_id.0).await {
            Ok(Some(pool)) => {
                // Verify pool belongs to this provider
                if pool.provider_pubkey != hex::encode(&provider_pubkey) {
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some("Pool not found".to_string()),
                    });
                }
                Json(ApiResponse {
                    success: true,
                    data: Some(pool),
                    error: None,
                })
            }
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Pool not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update agent pool
    ///
    /// Updates an existing agent pool's name, location, or provisioner type.
    #[oai(
        path = "/providers/:pubkey/pools/:pool_id",
        method = "put",
        tag = "ApiTags::Pools"
    )]
    async fn update_pool(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        pool_id: Path<String>,
        req: Json<UpdatePoolRequest>,
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

        match db
            .update_agent_pool(
                &pool_id.0,
                &provider_pubkey,
                req.name.as_deref(),
                req.location.as_deref(),
                req.provisioner_type.as_deref(),
            )
            .await
        {
            Ok(updated) => Json(ApiResponse {
                success: true,
                data: Some(updated),
                error: if updated {
                    None
                } else {
                    Some("No fields to update or pool not found".to_string())
                },
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete agent pool
    ///
    /// Deletes an agent pool. Fails if pool has any agents assigned.
    #[oai(
        path = "/providers/:pubkey/pools/:pool_id",
        method = "delete",
        tag = "ApiTags::Pools"
    )]
    async fn delete_pool(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        pool_id: Path<String>,
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

        match db.delete_agent_pool(&pool_id.0, &provider_pubkey).await {
            Ok(deleted) => Json(ApiResponse {
                success: true,
                data: Some(deleted),
                error: if deleted {
                    None
                } else {
                    Some("Pool not found".to_string())
                },
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Request agent upgrade for a pool
    ///
    /// Sets the target version for all agents in a pool. Agents pick up
    /// the upgrade directive on their next heartbeat and self-upgrade.
    /// Pass `version: null` to cancel a pending upgrade.
    #[oai(
        path = "/providers/:pubkey/pools/:pool_id/upgrade",
        method = "post",
        tag = "ApiTags::Pools"
    )]
    async fn request_pool_upgrade(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        pool_id: Path<String>,
        req: Json<PoolUpgradeRequest>,
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

        // Validate version format if provided (semver: X.Y.Z)
        if let Some(ref version) = req.version {
            let v = version.trim().trim_start_matches('v');
            let parts: Vec<&str> = v.split('.').collect();
            let valid = parts.len() == 3 && parts.iter().all(|p| p.parse::<u32>().is_ok());
            if !valid {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!(
                        "Invalid version format '{}': expected semver like 0.4.21",
                        version
                    )),
                });
            }
        }

        match db
            .set_pool_upgrade_version(&pool_id.0, &provider_pubkey, req.version.as_deref())
            .await
        {
            Ok(updated) => Json(ApiResponse {
                success: true,
                data: Some(updated),
                error: if updated {
                    None
                } else {
                    Some("Pool not found".to_string())
                },
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Create setup token
    ///
    /// Creates a one-time setup token for agent registration in a pool.
    #[oai(
        path = "/providers/:pubkey/pools/:pool_id/setup-tokens",
        method = "post",
        tag = "ApiTags::Pools"
    )]
    async fn create_setup_token(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        pool_id: Path<String>,
        req: Json<CreateSetupTokenRequest>,
    ) -> Json<ApiResponse<SetupToken>> {
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

        // Verify pool exists and belongs to provider
        match db.get_agent_pool(&pool_id.0).await {
            Ok(Some(pool)) if pool.provider_pubkey == hex::encode(&provider_pubkey) => {}
            Ok(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Pool not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        }

        let expires_in_hours = req.expires_in_hours.unwrap_or(24);

        match db
            .create_setup_token(&pool_id.0, req.label.as_deref(), expires_in_hours)
            .await
        {
            Ok(token) => Json(ApiResponse {
                success: true,
                data: Some(token),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// List setup tokens
    ///
    /// Returns pending (unused, unexpired) setup tokens for a pool.
    #[oai(
        path = "/providers/:pubkey/pools/:pool_id/setup-tokens",
        method = "get",
        tag = "ApiTags::Pools"
    )]
    async fn list_setup_tokens(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        pool_id: Path<String>,
    ) -> Json<ApiResponse<Vec<SetupToken>>> {
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

        // Verify pool exists and belongs to provider
        match db.get_agent_pool(&pool_id.0).await {
            Ok(Some(pool)) if pool.provider_pubkey == hex::encode(&provider_pubkey) => {}
            Ok(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Pool not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        }

        match db.list_pending_setup_tokens(&pool_id.0).await {
            Ok(tokens) => Json(ApiResponse {
                success: true,
                data: Some(tokens),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete setup token
    ///
    /// Deletes a setup token (e.g., to revoke it before it's used).
    #[oai(
        path = "/providers/:pubkey/pools/:pool_id/setup-tokens/:token",
        method = "delete",
        tag = "ApiTags::Pools"
    )]
    async fn delete_setup_token(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        pool_id: Path<String>,
        token: Path<String>,
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

        // Verify pool exists and belongs to provider
        match db.get_agent_pool(&pool_id.0).await {
            Ok(Some(pool)) if pool.provider_pubkey == hex::encode(&provider_pubkey) => {}
            Ok(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Pool not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        }

        match db.delete_setup_token(&token.0).await {
            Ok(deleted) => Json(ApiResponse {
                success: true,
                data: Some(deleted),
                error: if deleted {
                    None
                } else {
                    Some("Token not found".to_string())
                },
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    // ==================== Provisioning Lock Endpoints ====================

    /// Acquire provisioning lock
    ///
    /// Atomically acquires a provisioning lock on a contract.
    /// Returns 200 with acquired=true if lock acquired, acquired=false if already locked.
    /// Requires agent authentication with provision permission.
    #[oai(
        path = "/providers/:pubkey/contracts/:contract_id/lock",
        method = "post",
        tag = "ApiTags::Contracts"
    )]
    async fn acquire_lock(
        &self,
        db: Data<&Arc<Database>>,
        auth: AgentAuthenticatedUser,
        pubkey: Path<String>,
        contract_id: Path<String>,
    ) -> Json<ApiResponse<LockResponse>> {
        use crate::database::AgentPermission;

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

        // Verify agent belongs to this provider
        if provider_pubkey != auth.provider_pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Agent is not delegated by this provider".to_string()),
            });
        }

        // Check provision permission
        if let Err(e) = auth.require_permission(AgentPermission::Provision) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
        }

        // Decode contract ID
        let contract_bytes = match decode_hex_path(&contract_id.0, "contract id") {
            Ok(b) => b,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        // Lock duration: 5 minutes
        let lock_duration_ns = 5 * 60 * 1_000_000_000i64;
        let expires_at_ns = match crate::now_ns() {
            Ok(ns) => ns + lock_duration_ns,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        match db
            .acquire_provisioning_lock(&contract_bytes, &auth.agent_pubkey, lock_duration_ns)
            .await
        {
            Ok(acquired) => Json(ApiResponse {
                success: true,
                data: Some(LockResponse {
                    acquired,
                    expires_at_ns,
                }),
                error: if acquired {
                    None
                } else {
                    Some("Contract already locked by another agent".to_string())
                },
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Release provisioning lock
    ///
    /// Releases a provisioning lock held by this agent.
    /// Requires agent authentication.
    #[oai(
        path = "/providers/:pubkey/contracts/:contract_id/lock",
        method = "delete",
        tag = "ApiTags::Contracts"
    )]
    async fn release_lock(
        &self,
        db: Data<&Arc<Database>>,
        auth: AgentAuthenticatedUser,
        pubkey: Path<String>,
        contract_id: Path<String>,
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

        // Verify agent belongs to this provider
        if provider_pubkey != auth.provider_pubkey {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Agent is not delegated by this provider".to_string()),
            });
        }

        // Decode contract ID
        let contract_bytes = match decode_hex_path(&contract_id.0, "contract id") {
            Ok(b) => b,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        match db
            .release_provisioning_lock(&contract_bytes, &auth.agent_pubkey)
            .await
        {
            Ok(released) => Json(ApiResponse {
                success: true,
                data: Some(released),
                error: if released {
                    None
                } else {
                    Some("Lock not held by this agent".to_string())
                },
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

}

#[cfg(test)]
mod tests {
    use crate::openapi::common::{
        CreatePoolRequest, CreateSetupTokenRequest, LockResponse, UpdatePoolRequest,
    };

    // ── CreatePoolRequest / UpdatePoolRequest / CreateSetupTokenRequest ───────

    #[test]
    fn test_create_pool_request_deserialization() {
        let raw = r#"{"name":"eu-proxmox","location":"europe","provisionerType":"proxmox"}"#;
        let req: CreatePoolRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.name, "eu-proxmox");
        assert_eq!(req.location, "europe");
        assert_eq!(req.provisioner_type, "proxmox");
    }

    #[test]
    fn test_update_pool_request_all_optional_none() {
        let raw = r#"{}"#;
        let req: UpdatePoolRequest = serde_json::from_str(raw).unwrap();
        assert!(req.name.is_none());
        assert!(req.location.is_none());
        assert!(req.provisioner_type.is_none());
    }

    #[test]
    fn test_create_setup_token_request_defaults() {
        let raw = r#"{}"#;
        let req: CreateSetupTokenRequest = serde_json::from_str(raw).unwrap();
        assert!(req.label.is_none());
        assert!(req.expires_in_hours.is_none());
    }

    #[test]
    fn test_create_setup_token_request_with_values() {
        let raw = r#"{"label":"worker-01","expiresInHours":48}"#;
        let req: CreateSetupTokenRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.label.as_deref(), Some("worker-01"));
        assert_eq!(req.expires_in_hours, Some(48));
    }

    // ── LockResponse ─────────────────────────────────────────────────────────

    #[test]
    fn test_lock_response_acquired_camelcase() {
        let resp = LockResponse {
            acquired: true,
            expires_at_ns: 1_700_000_300_000_000_000,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["acquired"], true);
        assert_eq!(json["expiresAtNs"], 1_700_000_300_000_000_000_i64);
    }

    #[test]
    fn test_lock_response_not_acquired() {
        let resp = LockResponse {
            acquired: false,
            expires_at_ns: 0,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["acquired"], false);
    }

}
