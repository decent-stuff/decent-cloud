//! Cloud account and resource API endpoints.
//!
//! Handles self-provisioning of cloud resources (Hetzner, Proxmox).

use super::common::{ApiResponse, ApiTags};
use crate::auth::ApiAuthenticatedUser;
use crate::cloud::types::BackendCatalog;
use crate::cloud::{hetzner::HetznerBackend, proxmox_api::ProxmoxApiBackend, CloudBackend};
use crate::crypto::{
    decrypt_server_credential, encrypt_server_credential, ServerEncryptionKey,
};
use crate::database::{
    CloudAccount, CloudAccountWithCatalog, CloudResourceWithDetails,
    CreateCloudAccountInput, CreateCloudResourceInput, Database,
};
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
    ServerEncryptionKey::from_env().context(
        "CREDENTIAL_ENCRYPTION_KEY not configured - cloud account management unavailable",
    )
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

        let credentials_encrypted = match encrypt_server_credential(&req.credentials, &encryption_key)
        {
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
    #[oai(
        path = "/cloud-accounts/:id",
        method = "get",
        tag = "ApiTags::Cloud"
    )]
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
    ) -> Json<ApiResponse<()>> {
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
                data: Some(()),
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
    #[oai(
        path = "/cloud-resources/:id",
        method = "get",
        tag = "ApiTags::Cloud"
    )]
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
    ) -> Json<ApiResponse<()>> {
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

        match db.delete_cloud_resource(&uuid, &account_id).await {
            Ok(true) => Json(ApiResponse {
                success: true,
                data: Some(()),
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
}
