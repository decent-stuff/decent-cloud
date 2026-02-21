//! Cloud account and resource API endpoints.
//!
//! Handles self-provisioning of cloud resources (Hetzner, Proxmox).

use super::common::{ApiResponse, ApiTags, EmptyResponse};
use crate::auth::ApiAuthenticatedUser;
use crate::cloud::types::BackendCatalog;
use crate::cloud::{hetzner::HetznerBackend, proxmox_api::ProxmoxApiBackend, CloudBackend};
use crate::crypto::{decrypt_server_credential, encrypt_server_credential, ServerEncryptionKey};
use crate::database::offerings::Offering;
use crate::database::{CloudAccount, CloudResourceWithDetails, Database};
use anyhow::Context;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, Object, OpenApi};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Deserialize, Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AddCloudAccountRequest {
    pub backend_type: String,
    pub name: String,
    pub credentials: String,
    pub config: Option<String>,
}

#[derive(Debug, Deserialize, Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ProvisionResourceRequest {
    pub cloud_account_id: String,
    pub name: String,
    pub server_type: String,
    pub location: String,
    pub image: String,
    pub ssh_pubkey: String,
}

#[derive(Debug, Deserialize, Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ListOnMarketplaceRequest {
    pub offer_name: String,
    pub monthly_price: f64,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct CloudAccountListResponse {
    pub accounts: Vec<CloudAccount>,
}

#[derive(Debug, Serialize, Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct CloudResourceListResponse {
    pub resources: Vec<CloudResourceWithDetails>,
}

pub struct CloudApi;

fn get_encryption_key() -> anyhow::Result<ServerEncryptionKey> {
    ServerEncryptionKey::from_env()
        .context("CREDENTIAL_ENCRYPTION_KEY not configured - cloud account management unavailable")
}

/// Resolve the authenticated user's account_id (pubkey bytes).
async fn resolve_account_id(
    db: &Database,
    user: &ApiAuthenticatedUser,
) -> Result<Vec<u8>, String> {
    match db.get_account_id_by_public_key(&user.pubkey).await {
        Ok(Some(id)) => Ok(id),
        Ok(None) => Err("Account not found".to_string()),
        Err(e) => Err(format!("Database error: {e}")),
    }
}

async fn create_backend(
    backend_type: &str,
    credentials: &str,
) -> anyhow::Result<Box<dyn CloudBackend>> {
    match backend_type {
        "hetzner" => {
            let backend = HetznerBackend::new(credentials.to_string())?;
            Ok(Box::new(backend))
        }
        "proxmox_api" => {
            let config: crate::cloud::proxmox_api::ProxmoxConfig =
                serde_json::from_str(credentials)?;
            let backend = ProxmoxApiBackend::new(config)?;
            Ok(Box::new(backend))
        }
        _ => anyhow::bail!("Unknown backend type: {}", backend_type),
    }
}

#[OpenApi]
impl CloudApi {
    /// List cloud accounts
    ///
    /// Returns all cloud accounts connected by the authenticated user.
    #[oai(path = "/cloud-accounts", method = "get", tag = "ApiTags::Cloud")]
    async fn list_cloud_accounts(
        &self,
        db: Data<&Arc<Database>>,
        user: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<CloudAccountListResponse>> {
        let account_id = match db.get_account_id_by_public_key(&user.pubkey).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Account not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Database error: {}", e)),
                })
            }
        };

        match db.list_cloud_accounts(&account_id).await {
            Ok(accounts) => Json(ApiResponse {
                success: true,
                data: Some(CloudAccountListResponse { accounts }),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to list cloud accounts: {}", e)),
            }),
        }
    }

    /// Add cloud account
    ///
    /// Connect a new cloud account (Hetzner, Proxmox) for self-provisioning.
    /// Credentials are validated and encrypted before storage.
    #[oai(path = "/cloud-accounts", method = "post", tag = "ApiTags::Cloud")]
    async fn add_cloud_account(
        &self,
        db: Data<&Arc<Database>>,
        user: ApiAuthenticatedUser,
        req: Json<AddCloudAccountRequest>,
    ) -> Json<ApiResponse<CloudAccount>> {
        let account_id = match db.get_account_id_by_public_key(&user.pubkey).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Account not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Database error: {}", e)),
                })
            }
        };

        let encryption_key = match get_encryption_key() {
            Ok(key) => key,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        let backend_type = match req.backend_type.parse() {
            Ok(bt) => bt,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid backend type: {}", e)),
                })
            }
        };

        let backend = match create_backend(&req.backend_type, &req.credentials).await {
            Ok(b) => b,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to create backend client: {}", e)),
                })
            }
        };

        if let Err(e) = backend.validate_credentials().await {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Credential validation failed: {}", e)),
            });
        }

        let credentials_encrypted =
            match encrypt_server_credential(&req.credentials, &encryption_key) {
                Ok(enc) => enc,
                Err(e) => {
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to encrypt credentials: {}", e)),
                    })
                }
            };

        match db
            .create_cloud_account(
                &account_id,
                backend_type,
                &req.name,
                &credentials_encrypted,
                req.config.as_deref(),
            )
            .await
        {
            Ok(account) => Json(ApiResponse {
                success: true,
                data: Some(account),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to create cloud account: {}", e)),
            }),
        }
    }

    /// Get cloud account
    ///
    /// Returns details for a specific cloud account.
    #[oai(path = "/cloud-accounts/:id", method = "get", tag = "ApiTags::Cloud")]
    async fn get_cloud_account(
        &self,
        db: Data<&Arc<Database>>,
        user: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<CloudAccount>> {
        let account_id = match db.get_account_id_by_public_key(&user.pubkey).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Account not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Database error: {}", e)),
                })
            }
        };

        let uuid = match id.0.parse::<Uuid>() {
            Ok(u) => u,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid account ID".to_string()),
                })
            }
        };

        match db.get_cloud_account(&uuid, &account_id).await {
            Ok(Some(account)) => Json(ApiResponse {
                success: true,
                data: Some(account),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Cloud account not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Database error: {}", e)),
            }),
        }
    }

    /// Delete cloud account
    ///
    /// Removes a cloud account. Resources provisioned through this account are not affected.
    #[oai(
        path = "/cloud-accounts/:id",
        method = "delete",
        tag = "ApiTags::Cloud"
    )]
    async fn delete_cloud_account(
        &self,
        db: Data<&Arc<Database>>,
        user: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<EmptyResponse>> {
        let account_id = match db.get_account_id_by_public_key(&user.pubkey).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Account not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Database error: {}", e)),
                })
            }
        };

        let uuid = match id.0.parse::<Uuid>() {
            Ok(u) => u,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid account ID".to_string()),
                })
            }
        };

        match db.delete_cloud_account(&uuid, &account_id).await {
            Ok(true) => Json(ApiResponse {
                success: true,
                data: Some(EmptyResponse {}),
                error: None,
            }),
            Ok(false) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Cloud account not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to delete cloud account: {}", e)),
            }),
        }
    }

    /// Get cloud account catalog
    ///
    /// Returns available server types, locations, and images for a cloud account.
    #[oai(
        path = "/cloud-accounts/:id/catalog",
        method = "get",
        tag = "ApiTags::Cloud"
    )]
    async fn get_cloud_account_catalog(
        &self,
        db: Data<&Arc<Database>>,
        user: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<BackendCatalog>> {
        let encryption_key = match get_encryption_key() {
            Ok(key) => key,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        let uuid = match id.0.parse::<Uuid>() {
            Ok(u) => u,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid account ID".to_string()),
                })
            }
        };

        let (account_id, backend_type, credentials_encrypted) =
            match db.get_cloud_account_credentials(&uuid).await {
                Ok(Some(row)) => row,
                Ok(None) => {
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some("Cloud account not found".to_string()),
                    })
                }
                Err(e) => {
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(format!("Database error: {}", e)),
                    })
                }
            };

        let user_account_id = match db.get_account_id_by_public_key(&user.pubkey).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Account not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Database error: {}", e)),
                })
            }
        };

        if account_id != user_account_id {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized".to_string()),
            });
        }

        let credentials = match decrypt_server_credential(&credentials_encrypted, &encryption_key) {
            Ok(c) => c,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to decrypt credentials: {}", e)),
                })
            }
        };

        let backend = match create_backend(&backend_type, &credentials).await {
            Ok(b) => b,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to create backend client: {}", e)),
                })
            }
        };

        match backend.get_catalog().await {
            Ok(catalog) => Json(ApiResponse {
                success: true,
                data: Some(catalog),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to get catalog: {}", e)),
            }),
        }
    }

    /// List cloud resources
    ///
    /// Returns all self-provisioned resources for the authenticated user.
    #[oai(path = "/cloud-resources", method = "get", tag = "ApiTags::Cloud")]
    async fn list_cloud_resources(
        &self,
        db: Data<&Arc<Database>>,
        user: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<CloudResourceListResponse>> {
        let account_id = match db.get_account_id_by_public_key(&user.pubkey).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Account not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Database error: {}", e)),
                })
            }
        };

        match db.list_cloud_resources(&account_id).await {
            Ok(resources) => Json(ApiResponse {
                success: true,
                data: Some(CloudResourceListResponse { resources }),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to list cloud resources: {}", e)),
            }),
        }
    }

    /// Get cloud resource
    ///
    /// Returns details for a specific self-provisioned resource.
    #[oai(path = "/cloud-resources/:id", method = "get", tag = "ApiTags::Cloud")]
    async fn get_cloud_resource(
        &self,
        db: Data<&Arc<Database>>,
        user: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<CloudResourceWithDetails>> {
        let account_id = match db.get_account_id_by_public_key(&user.pubkey).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Account not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Database error: {}", e)),
                })
            }
        };

        let uuid = match id.0.parse::<Uuid>() {
            Ok(u) => u,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid resource ID".to_string()),
                })
            }
        };

        match db.get_cloud_resource(&uuid, &account_id).await {
            Ok(Some(resource)) => Json(ApiResponse {
                success: true,
                data: Some(resource),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Cloud resource not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Database error: {}", e)),
            }),
        }
    }

    /// Provision cloud resource
    ///
    /// Creates a new cloud resource (VM) using the specified cloud account.
    /// The resource will be provisioned asynchronously - check status via GET endpoint.
    #[oai(path = "/cloud-resources", method = "post", tag = "ApiTags::Cloud")]
    async fn provision_cloud_resource(
        &self,
        db: Data<&Arc<Database>>,
        user: ApiAuthenticatedUser,
        req: Json<ProvisionResourceRequest>,
    ) -> Json<ApiResponse<crate::database::cloud_resources::CloudResource>> {
        let account_id = match db.get_account_id_by_public_key(&user.pubkey).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Account not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Database error: {}", e)),
                })
            }
        };

        let cloud_account_uuid = match req.cloud_account_id.parse::<Uuid>() {
            Ok(u) => u,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid cloud account ID".to_string()),
                })
            }
        };

        let cloud_account = match db.get_cloud_account(&cloud_account_uuid, &account_id).await {
            Ok(Some(acc)) => acc,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Cloud account not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Database error: {}", e)),
                })
            }
        };

        if !cloud_account.is_valid {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Cloud account credentials are invalid".to_string()),
            });
        }

        let external_id = format!("pending-{}", uuid::Uuid::new_v4());

        match db
            .create_cloud_resource(
                &cloud_account_uuid,
                &external_id,
                &req.name,
                &req.server_type,
                &req.location,
                &req.image,
                &req.ssh_pubkey,
            )
            .await
        {
            Ok(resource) => Json(ApiResponse {
                success: true,
                data: Some(resource),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to create cloud resource: {}", e)),
            }),
        }
    }

    /// Delete cloud resource
    ///
    /// Terminates a cloud resource. This action is irreversible.
    #[oai(
        path = "/cloud-resources/:id",
        method = "delete",
        tag = "ApiTags::Cloud"
    )]
    async fn delete_cloud_resource(
        &self,
        db: Data<&Arc<Database>>,
        user: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<EmptyResponse>> {
        let account_id = match db.get_account_id_by_public_key(&user.pubkey).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Account not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Database error: {}", e)),
                })
            }
        };

        let uuid = match id.0.parse::<Uuid>() {
            Ok(u) => u,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid resource ID".to_string()),
                })
            }
        };

        // Auto-unlist from marketplace if listed
        if let Ok(Some(resource)) = db.get_cloud_resource(&uuid, &account_id).await {
            if resource.resource.listing_mode == "marketplace" {
                if let Ok(old_offering_id) = db.unlist_from_marketplace(&uuid, &account_id).await {
                    if let Err(e) = db.delete_offering(&user.pubkey, old_offering_id).await {
                        tracing::warn!(resource_id = %uuid, offering_id = old_offering_id, "Failed to delete offering during resource deletion: {e:#}");
                    }
                }
            }
        }

        match db.delete_cloud_resource(&uuid, &account_id).await {
            Ok(true) => Json(ApiResponse {
                success: true,
                data: Some(EmptyResponse {}),
                error: None,
            }),
            Ok(false) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Cloud resource not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to delete cloud resource: {}", e)),
            }),
        }
    }

    /// Start cloud resource
    ///
    /// Powers on a stopped cloud resource (VM).
    #[oai(
        path = "/cloud-resources/:id/start",
        method = "post",
        tag = "ApiTags::Cloud"
    )]
    async fn start_cloud_resource(
        &self,
        db: Data<&Arc<Database>>,
        user: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<EmptyResponse>> {
        let account_id = match resolve_account_id(&db, &user).await {
            Ok(id) => id,
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(e) }),
        };

        let uuid = match id.0.parse::<Uuid>() {
            Ok(u) => u,
            Err(_) => return Json(ApiResponse { success: false, data: None, error: Some("Invalid resource ID".to_string()) }),
        };

        let encryption_key = match get_encryption_key() {
            Ok(key) => key,
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(e.to_string()) }),
        };

        let ctx = match db.get_cloud_resource_action_context(&uuid, &account_id).await {
            Ok(Some(ctx)) => ctx,
            Ok(None) => return Json(ApiResponse { success: false, data: None, error: Some("Cloud resource not found".to_string()) }),
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(format!("Database error: {e}")) }),
        };

        if ctx.status != "stopped" {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Cannot start resource in '{}' status (must be 'stopped')", ctx.status)),
            });
        }

        let credentials = match decrypt_server_credential(&ctx.credentials_encrypted, &encryption_key) {
            Ok(c) => c,
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(format!("Failed to decrypt credentials: {e}")) }),
        };

        let backend = match create_backend(&ctx.backend_type, &credentials).await {
            Ok(b) => b,
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(format!("Failed to create backend: {e}")) }),
        };

        if let Err(e) = backend.start_server(&ctx.external_id).await {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to start server: {e}")),
            });
        }

        if let Err(e) = db.transition_cloud_resource_status(&uuid, &account_id, "stopped", "running").await {
            tracing::error!(resource_id = %uuid, "Server started but DB update failed: {e:#}");
            return Json(ApiResponse { success: false, data: None, error: Some(format!("Server started but status update failed: {e}")) });
        }

        Json(ApiResponse { success: true, data: Some(EmptyResponse {}), error: None })
    }

    /// Stop cloud resource
    ///
    /// Shuts down a running cloud resource (VM). The resource can be started again later.
    #[oai(
        path = "/cloud-resources/:id/stop",
        method = "post",
        tag = "ApiTags::Cloud"
    )]
    async fn stop_cloud_resource(
        &self,
        db: Data<&Arc<Database>>,
        user: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<EmptyResponse>> {
        let account_id = match resolve_account_id(&db, &user).await {
            Ok(id) => id,
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(e) }),
        };

        let uuid = match id.0.parse::<Uuid>() {
            Ok(u) => u,
            Err(_) => return Json(ApiResponse { success: false, data: None, error: Some("Invalid resource ID".to_string()) }),
        };

        let encryption_key = match get_encryption_key() {
            Ok(key) => key,
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(e.to_string()) }),
        };

        let ctx = match db.get_cloud_resource_action_context(&uuid, &account_id).await {
            Ok(Some(ctx)) => ctx,
            Ok(None) => return Json(ApiResponse { success: false, data: None, error: Some("Cloud resource not found".to_string()) }),
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(format!("Database error: {e}")) }),
        };

        if ctx.status != "running" {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Cannot stop resource in '{}' status (must be 'running')", ctx.status)),
            });
        }

        if ctx.listing_mode == "marketplace" {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Cannot stop a marketplace-listed resource. Unlist it first.".to_string()),
            });
        }

        let credentials = match decrypt_server_credential(&ctx.credentials_encrypted, &encryption_key) {
            Ok(c) => c,
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(format!("Failed to decrypt credentials: {e}")) }),
        };

        let backend = match create_backend(&ctx.backend_type, &credentials).await {
            Ok(b) => b,
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(format!("Failed to create backend: {e}")) }),
        };

        if let Err(e) = backend.stop_server(&ctx.external_id).await {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to stop server: {e}")),
            });
        }

        if let Err(e) = db.transition_cloud_resource_status(&uuid, &account_id, "running", "stopped").await {
            tracing::error!(resource_id = %uuid, "Server stopped but DB update failed: {e:#}");
            return Json(ApiResponse { success: false, data: None, error: Some(format!("Server stopped but status update failed: {e}")) });
        }

        Json(ApiResponse { success: true, data: Some(EmptyResponse {}), error: None })
    }

    /// Validate cloud account credentials
    ///
    /// Re-validates the stored credentials against the cloud backend.
    /// Updates the account's validation status.
    #[oai(
        path = "/cloud-accounts/:id/validate",
        method = "post",
        tag = "ApiTags::Cloud"
    )]
    async fn validate_cloud_account(
        &self,
        db: Data<&Arc<Database>>,
        user: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<CloudAccount>> {
        let account_id = match resolve_account_id(&db, &user).await {
            Ok(id) => id,
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(e) }),
        };

        let uuid = match id.0.parse::<Uuid>() {
            Ok(u) => u,
            Err(_) => return Json(ApiResponse { success: false, data: None, error: Some("Invalid account ID".to_string()) }),
        };

        let encryption_key = match get_encryption_key() {
            Ok(key) => key,
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(e.to_string()) }),
        };

        let (owner_id, backend_type, credentials_encrypted) =
            match db.get_cloud_account_credentials(&uuid).await {
                Ok(Some(row)) => row,
                Ok(None) => return Json(ApiResponse { success: false, data: None, error: Some("Cloud account not found".to_string()) }),
                Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(format!("Database error: {e}")) }),
            };

        if owner_id != account_id {
            return Json(ApiResponse { success: false, data: None, error: Some("Unauthorized".to_string()) });
        }

        let credentials = match decrypt_server_credential(&credentials_encrypted, &encryption_key) {
            Ok(c) => c,
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(format!("Failed to decrypt credentials: {e}")) }),
        };

        let (is_valid, validation_error) = match create_backend(&backend_type, &credentials).await {
            Ok(backend) => match backend.validate_credentials().await {
                Ok(()) => (true, None),
                Err(e) => (false, Some(format!("{e}"))),
            },
            Err(e) => (false, Some(format!("Failed to create backend: {e}"))),
        };

        if let Err(e) = db
            .update_cloud_account_validation(&uuid, &account_id, is_valid, validation_error.as_deref())
            .await
        {
            return Json(ApiResponse { success: false, data: None, error: Some(format!("Failed to update validation status: {e}")) });
        }

        match db.get_cloud_account(&uuid, &account_id).await {
            Ok(Some(account)) => Json(ApiResponse { success: true, data: Some(account), error: None }),
            Ok(None) => Json(ApiResponse { success: false, data: None, error: Some("Cloud account not found after update".to_string()) }),
            Err(e) => Json(ApiResponse { success: false, data: None, error: Some(format!("Database error: {e}")) }),
        }
    }

    /// List cloud resource on marketplace
    ///
    /// Creates a marketplace offering from a running personal resource, making it
    /// discoverable by other users. The resource must be running and not already listed.
    #[oai(
        path = "/cloud-resources/:id/list-on-marketplace",
        method = "post",
        tag = "ApiTags::Cloud"
    )]
    async fn list_on_marketplace(
        &self,
        db: Data<&Arc<Database>>,
        user: ApiAuthenticatedUser,
        id: Path<String>,
        req: Json<ListOnMarketplaceRequest>,
    ) -> Json<ApiResponse<Offering>> {
        let account_id = match resolve_account_id(&db, &user).await {
            Ok(id) => id,
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(e) }),
        };

        let uuid = match id.0.parse::<Uuid>() {
            Ok(u) => u,
            Err(_) => return Json(ApiResponse { success: false, data: None, error: Some("Invalid resource ID".to_string()) }),
        };

        let encryption_key = match get_encryption_key() {
            Ok(key) => key,
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(e.to_string()) }),
        };

        // Fetch resource to validate state and get server_type/location
        let resource = match db.get_cloud_resource(&uuid, &account_id).await {
            Ok(Some(r)) => r,
            Ok(None) => return Json(ApiResponse { success: false, data: None, error: Some("Cloud resource not found".to_string()) }),
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(format!("Database error: {e}")) }),
        };

        if resource.resource.status != "running" {
            return Json(ApiResponse { success: false, data: None, error: Some(format!("Resource must be running to list (current status: '{}')", resource.resource.status)) });
        }
        if resource.resource.listing_mode != "personal" {
            return Json(ApiResponse { success: false, data: None, error: Some("Resource is already listed on the marketplace".to_string()) });
        }

        // Get credentials and catalog to populate offering hardware specs
        let cloud_account_uuid = match resource.resource.cloud_account_id.parse::<Uuid>() {
            Ok(u) => u,
            Err(_) => return Json(ApiResponse { success: false, data: None, error: Some("Invalid cloud account ID in resource".to_string()) }),
        };

        let (_owner_id, backend_type, credentials_encrypted) =
            match db.get_cloud_account_credentials(&cloud_account_uuid).await {
                Ok(Some(row)) => row,
                Ok(None) => return Json(ApiResponse { success: false, data: None, error: Some("Cloud account not found".to_string()) }),
                Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(format!("Database error: {e}")) }),
            };

        let credentials = match decrypt_server_credential(&credentials_encrypted, &encryption_key) {
            Ok(c) => c,
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(format!("Failed to decrypt credentials: {e}")) }),
        };

        let backend = match create_backend(&backend_type, &credentials).await {
            Ok(b) => b,
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(format!("Failed to create backend: {e}")) }),
        };

        let catalog = match backend.get_catalog().await {
            Ok(c) => c,
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(format!("Failed to get catalog: {e}")) }),
        };

        // Find server type and location in catalog
        let server_type = catalog.server_types.iter().find(|st| st.id == resource.resource.server_type);
        let location = catalog.locations.iter().find(|l| l.id == resource.resource.location);

        let resource_id_str = resource.resource.id.clone();
        let offering_id_str = format!("self-{}", &resource_id_str[..8.min(resource_id_str.len())]);

        let offering = Offering {
            id: None,
            pubkey: hex::encode(&user.pubkey),
            offering_id: offering_id_str,
            offer_name: req.offer_name.clone(),
            description: req.description.clone(),
            product_page_url: None,
            currency: "USD".to_string(),
            monthly_price: req.monthly_price,
            setup_fee: 0.0,
            visibility: "public".to_string(),
            product_type: "VPS".to_string(),
            virtualization_type: None,
            billing_interval: "monthly".to_string(),
            billing_unit: "month".to_string(),
            pricing_model: None,
            price_per_unit: None,
            included_units: None,
            overage_price_per_unit: None,
            stripe_metered_price_id: None,
            is_subscription: false,
            subscription_interval_days: None,
            stock_status: "in_stock".to_string(),
            processor_brand: None,
            processor_amount: None,
            processor_cores: server_type.map(|st| st.cores as i64),
            processor_speed: None,
            processor_name: None,
            memory_error_correction: None,
            memory_type: None,
            memory_amount: server_type.map(|st| format!("{} GB", st.memory_gb)),
            hdd_amount: None,
            total_hdd_capacity: None,
            ssd_amount: None,
            total_ssd_capacity: server_type.map(|st| format!("{} GB", st.disk_gb)),
            unmetered_bandwidth: false,
            uplink_speed: None,
            traffic: None,
            datacenter_country: location.map(|l| l.country.clone()).unwrap_or_default(),
            datacenter_city: location.map(|l| l.city.clone()).unwrap_or_default(),
            datacenter_latitude: None,
            datacenter_longitude: None,
            control_panel: None,
            gpu_name: None,
            gpu_count: None,
            gpu_memory_gb: None,
            min_contract_hours: None,
            max_contract_hours: None,
            payment_methods: None,
            features: None,
            operating_systems: None,
            trust_score: None,
            has_critical_flags: None,
            is_example: false,
            is_draft: false,
            offering_source: Some("self_provisioned".to_string()),
            external_checkout_url: None,
            reseller_name: None,
            reseller_commission_percent: None,
            owner_username: None,
            provisioner_type: None,
            provisioner_config: None,
            template_name: None,
            agent_pool_id: None,
            post_provision_script: None,
            provider_online: None,
            resolved_pool_id: None,
            resolved_pool_name: None,
            reliability_score: None,
            created_at_ns: None,
        };

        let offering_db_id = match db.create_offering(&user.pubkey, offering).await {
            Ok(id) => id,
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(format!("Failed to create offering: {e}")) }),
        };

        if let Err(e) = db.list_on_marketplace(&uuid, &account_id, offering_db_id).await {
            // Rollback: delete the offering we just created
            if let Err(rollback_err) = db.delete_offering(&user.pubkey, offering_db_id).await {
                tracing::error!("Failed to rollback offering {} after marketplace listing failure: {rollback_err:#}", offering_db_id);
            }
            return Json(ApiResponse { success: false, data: None, error: Some(format!("Failed to list on marketplace: {e}")) });
        }

        match db.get_offering(offering_db_id).await {
            Ok(Some(offering)) => Json(ApiResponse { success: true, data: Some(offering), error: None }),
            Ok(None) => Json(ApiResponse { success: false, data: None, error: Some("Offering created but not found".to_string()) }),
            Err(e) => Json(ApiResponse { success: false, data: None, error: Some(format!("Database error: {e}")) }),
        }
    }

    /// Unlist cloud resource from marketplace
    ///
    /// Removes the marketplace offering and returns the resource to personal mode.
    #[oai(
        path = "/cloud-resources/:id/unlist-from-marketplace",
        method = "post",
        tag = "ApiTags::Cloud"
    )]
    async fn unlist_from_marketplace(
        &self,
        db: Data<&Arc<Database>>,
        user: ApiAuthenticatedUser,
        id: Path<String>,
    ) -> Json<ApiResponse<EmptyResponse>> {
        let account_id = match resolve_account_id(&db, &user).await {
            Ok(id) => id,
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(e) }),
        };

        let uuid = match id.0.parse::<Uuid>() {
            Ok(u) => u,
            Err(_) => return Json(ApiResponse { success: false, data: None, error: Some("Invalid resource ID".to_string()) }),
        };

        let old_offering_id = match db.unlist_from_marketplace(&uuid, &account_id).await {
            Ok(id) => id,
            Err(e) => return Json(ApiResponse { success: false, data: None, error: Some(format!("Failed to unlist: {e}")) }),
        };

        if let Err(e) = db.delete_offering(&user.pubkey, old_offering_id).await {
            tracing::error!(offering_id = old_offering_id, "Resource unlisted but offering deletion failed: {e:#}");
            return Json(ApiResponse { success: false, data: None, error: Some(format!("Unlisted but failed to delete offering: {e}")) });
        }

        Json(ApiResponse { success: true, data: Some(EmptyResponse {}), error: None })
    }
}

#[cfg(test)]
mod tests {
    use crate::database::cloud_accounts::CloudAccount;
    use crate::database::cloud_resources::{CloudResource, CloudResourceWithDetails};
    use crate::openapi::common::ApiResponse;

    use super::{
        AddCloudAccountRequest, CloudAccountListResponse, CloudResourceListResponse,
        ListOnMarketplaceRequest, ProvisionResourceRequest,
    };

    fn sample_cloud_account() -> CloudAccount {
        CloudAccount {
            id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            account_id: "aabbcc".to_string(),
            backend_type: "hetzner".to_string(),
            name: "my-hetzner".to_string(),
            config: None,
            is_valid: true,
            last_validated_at: Some("2024-01-01T00:00:00+00:00".to_string()),
            validation_error: None,
            created_at: "2024-01-01T00:00:00+00:00".to_string(),
            updated_at: "2024-01-01T00:00:00+00:00".to_string(),
        }
    }

    fn sample_cloud_resource() -> CloudResource {
        CloudResource {
            id: "660e8400-e29b-41d4-a716-446655440001".to_string(),
            cloud_account_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            external_id: "hetzner-12345".to_string(),
            name: "my-vm".to_string(),
            server_type: "cx22".to_string(),
            location: "nbg1".to_string(),
            image: "ubuntu-24.04".to_string(),
            ssh_pubkey: "ssh-ed25519 AAAA test".to_string(),
            status: "running".to_string(),
            public_ip: Some("1.2.3.4".to_string()),
            ssh_port: 22,
            ssh_username: "root".to_string(),
            external_ssh_key_id: None,
            gateway_slug: Some("abc123".to_string()),
            gateway_subdomain: Some("abc123.dc1.gw.decent-cloud.org".to_string()),
            gateway_ssh_port: Some(20000),
            gateway_port_range_start: Some(20000),
            gateway_port_range_end: Some(20009),
            offering_id: None,
            listing_mode: "personal".to_string(),
            error_message: None,
            platform_fee_e9s: 0,
            created_at: "2024-01-01T00:00:00+00:00".to_string(),
            updated_at: "2024-01-01T00:00:00+00:00".to_string(),
            terminated_at: None,
        }
    }

    fn sample_cloud_resource_with_details() -> CloudResourceWithDetails {
        CloudResourceWithDetails {
            resource: sample_cloud_resource(),
            cloud_account_name: "my-hetzner".to_string(),
            cloud_account_backend: "hetzner".to_string(),
        }
    }

    // --- AddCloudAccountRequest ---

    #[test]
    fn test_add_cloud_account_request_camel_case_deserialization() {
        let json = r#"{"backendType":"hetzner","name":"my-account","credentials":"token-abc","config":null}"#;
        let req: AddCloudAccountRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.backend_type, "hetzner");
        assert_eq!(req.name, "my-account");
        assert_eq!(req.credentials, "token-abc");
        assert!(req.config.is_none());
    }

    #[test]
    fn test_add_cloud_account_request_with_config() {
        let json = r#"{"backendType":"proxmox_api","name":"prox","credentials":"{}","config":"{\"host\":\"10.0.0.1\"}"}"#;
        let req: AddCloudAccountRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.backend_type, "proxmox_api");
        assert!(req.config.is_some());
    }

    // --- ProvisionResourceRequest ---

    #[test]
    fn test_provision_resource_request_camel_case_deserialization() {
        let json = r#"{"cloudAccountId":"550e8400-e29b-41d4-a716-446655440000","name":"my-vm","serverType":"cx22","location":"nbg1","image":"ubuntu-24.04","sshPubkey":"ssh-ed25519 AAAA test"}"#;
        let req: ProvisionResourceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.cloud_account_id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(req.name, "my-vm");
        assert_eq!(req.server_type, "cx22");
        assert_eq!(req.location, "nbg1");
        assert_eq!(req.image, "ubuntu-24.04");
        assert_eq!(req.ssh_pubkey, "ssh-ed25519 AAAA test");
    }

    // --- ListOnMarketplaceRequest ---

    #[test]
    fn test_list_on_marketplace_request_with_description() {
        let json = r#"{"offerName":"Small VPS","monthlyPrice":9.99,"description":"A small server"}"#;
        let req: ListOnMarketplaceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.offer_name, "Small VPS");
        assert_eq!(req.monthly_price, 9.99);
        assert_eq!(req.description.as_deref(), Some("A small server"));
    }

    #[test]
    fn test_list_on_marketplace_request_without_description() {
        let json = r#"{"offerName":"Basic","monthlyPrice":4.50,"description":null}"#;
        let req: ListOnMarketplaceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.offer_name, "Basic");
        assert_eq!(req.monthly_price, 4.50);
        assert!(req.description.is_none());
    }

    // --- CloudAccount serialization ---

    #[test]
    fn test_cloud_account_serialization_field_names() {
        let account = sample_cloud_account();
        let json = serde_json::to_value(&account).unwrap();
        assert_eq!(json["id"], "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(json["backendType"], "hetzner");
        assert_eq!(json["name"], "my-hetzner");
        assert_eq!(json["isValid"], true);
        assert!(json["validationError"].is_null());
        assert!(json["lastValidatedAt"].is_string());
    }

    #[test]
    fn test_cloud_account_invalid_state_serialization() {
        let account = CloudAccount {
            is_valid: false,
            last_validated_at: Some("2024-06-01T00:00:00+00:00".to_string()),
            validation_error: Some("Token expired".to_string()),
            ..sample_cloud_account()
        };
        let json = serde_json::to_value(&account).unwrap();
        assert_eq!(json["isValid"], false);
        assert_eq!(json["validationError"], "Token expired");
    }

    // --- CloudResourceWithDetails serialization ---

    #[test]
    fn test_cloud_resource_with_details_serialization_flattens_resource() {
        let r = sample_cloud_resource_with_details();
        let json = serde_json::to_value(&r).unwrap();
        // Flattened fields from CloudResource appear at top level
        assert_eq!(json["id"], "660e8400-e29b-41d4-a716-446655440001");
        assert_eq!(json["serverType"], "cx22");
        assert_eq!(json["status"], "running");
        assert_eq!(json["listingMode"], "personal");
        assert_eq!(json["platformFeeE9s"], 0);
        // Extra fields from the wrapper
        assert_eq!(json["cloudAccountName"], "my-hetzner");
        assert_eq!(json["cloudAccountBackend"], "hetzner");
    }

    #[test]
    fn test_cloud_resource_optional_fields_null_when_absent() {
        let r = sample_cloud_resource_with_details();
        let json = serde_json::to_value(&r).unwrap();
        assert!(json["errorMessage"].is_null());
        assert!(json["terminatedAt"].is_null());
        assert!(json["externalSshKeyId"].is_null());
        assert!(json["offeringId"].is_null());
    }

    // --- CloudAccountListResponse ---

    #[test]
    fn test_cloud_account_list_response_serialization() {
        let resp = CloudAccountListResponse {
            accounts: vec![sample_cloud_account()],
        };
        let json = serde_json::to_value(&resp).unwrap();
        let accounts = json["accounts"].as_array().unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0]["backendType"], "hetzner");
    }

    #[test]
    fn test_cloud_account_list_response_empty() {
        let resp = CloudAccountListResponse { accounts: vec![] };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["accounts"].as_array().unwrap().len(), 0);
    }

    // --- CloudResourceListResponse ---

    #[test]
    fn test_cloud_resource_list_response_serialization() {
        let resp = CloudResourceListResponse {
            resources: vec![sample_cloud_resource_with_details()],
        };
        let json = serde_json::to_value(&resp).unwrap();
        let resources = json["resources"].as_array().unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0]["serverType"], "cx22");
        assert_eq!(resources[0]["cloudAccountName"], "my-hetzner");
    }

    // --- ApiResponse wrappers ---

    #[test]
    fn test_api_response_cloud_account_success() {
        let resp = ApiResponse {
            success: true,
            data: Some(sample_cloud_account()),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert!(json.get("error").is_none());
        assert_eq!(json["data"]["backendType"], "hetzner");
        assert_eq!(json["data"]["isValid"], true);
    }

    #[test]
    fn test_api_response_cloud_account_error() {
        let resp: ApiResponse<CloudAccount> = ApiResponse {
            success: false,
            data: None,
            error: Some("Cloud account not found".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], false);
        assert!(json.get("data").is_none());
        assert_eq!(json["error"], "Cloud account not found");
    }

    #[test]
    fn test_api_response_cloud_resource_with_details_success() {
        let resp = ApiResponse {
            success: true,
            data: Some(sample_cloud_resource_with_details()),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["status"], "running");
        assert_eq!(json["data"]["cloudAccountBackend"], "hetzner");
    }

    #[test]
    fn test_api_response_cloud_resource_invalid_id_error() {
        let resp: ApiResponse<CloudResourceWithDetails> = ApiResponse {
            success: false,
            data: None,
            error: Some("Invalid resource ID".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], false);
        assert_eq!(json["error"], "Invalid resource ID");
    }

    #[test]
    fn test_api_response_cloud_account_list_success() {
        let resp = ApiResponse {
            success: true,
            data: Some(CloudAccountListResponse {
                accounts: vec![sample_cloud_account(), {
                    let mut a = sample_cloud_account();
                    a.name = "second-account".to_string();
                    a
                }],
            }),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["accounts"].as_array().unwrap().len(), 2);
        assert_eq!(json["data"]["accounts"][1]["name"], "second-account");
    }

    #[test]
    fn test_api_response_cloud_resource_list_error() {
        let resp: ApiResponse<CloudResourceListResponse> = ApiResponse {
            success: false,
            data: None,
            error: Some("Failed to list cloud resources: connection refused".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], false);
        assert!(json.get("data").is_none());
        assert!(json["error"]
            .as_str()
            .unwrap()
            .contains("connection refused"));
    }
}
