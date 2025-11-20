use crate::{
    auth::{AdminAuthenticatedUser, ApiAuthenticatedUser},
    database::Database,
    metadata_cache::MetadataCache,
};
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, Object, OpenApi};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct MainApi;

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct HealthResponse {
    pub success: bool,
    pub message: String,
    pub environment: String,
}

#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(skip_serializing_if_is_none)]
pub struct ApiResponse<T: poem_openapi::types::ParseFromJSON + poem_openapi::types::ToJSON> {
    pub success: bool,
    #[oai(skip_serializing_if_is_none)]
    pub data: Option<T>,
    #[oai(skip_serializing_if_is_none)]
    pub error: Option<String>,
}

fn default_limit() -> i64 {
    50
}

fn default_false() -> bool {
    false
}

// Request types for write operations
#[derive(Debug, Deserialize, Object)]
pub struct DuplicateOfferingRequest {
    pub new_offering_id: String,
}

#[derive(Debug, Deserialize, Object)]
pub struct BulkUpdateStatusRequest {
    pub offering_ids: Vec<i64>,
    pub stock_status: String,
}

#[derive(Debug, Serialize, Object)]
pub struct RentalRequestResponse {
    pub contract_id: String,
    pub message: String,
}

#[derive(Debug, Deserialize, Object)]
pub struct RentalResponseRequest {
    pub accept: bool,
    pub memo: Option<String>,
}

#[derive(Debug, Deserialize, Object)]
pub struct ProvisioningStatusRequest {
    pub status: String,
    pub instance_details: Option<String>,
}

#[derive(Debug, Deserialize, Object)]
pub struct ExtendContractRequest {
    pub extension_hours: i64,
    pub memo: Option<String>,
}

#[derive(Debug, Serialize, Object)]
pub struct ExtendContractResponse {
    pub extension_payment_e9s: i64,
    pub new_end_timestamp_ns: i64,
    pub message: String,
}

#[derive(Debug, Deserialize, Object)]
pub struct CancelContractRequest {
    pub memo: Option<String>,
}

#[derive(Debug, Deserialize, Object)]
pub struct UpdateUserProfileRequest {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize, Object)]
pub struct UpsertContactRequest {
    pub contact_type: String,
    pub contact_value: String,
    #[serde(default)]
    pub verified: bool,
}

#[derive(Debug, Deserialize, Object)]
pub struct UpsertSocialRequest {
    pub platform: String,
    pub username: String,
    pub profile_url: Option<String>,
}

#[derive(Debug, Deserialize, Object)]
pub struct AddPublicKeyRequest {
    pub key_type: String,
    pub key_data: String,
    pub key_fingerprint: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Serialize, Object)]
pub struct CsvImportResult {
    pub success_count: usize,
    pub errors: Vec<CsvImportError>,
}

#[derive(Debug, Serialize, Object)]
pub struct CsvImportError {
    pub row: usize,
    pub message: String,
}

// Helper functions
fn decode_pubkey(pubkey_hex: &str) -> Result<Vec<u8>, String> {
    hex::decode(pubkey_hex).map_err(|_| "Invalid pubkey format".to_string())
}

fn check_authorization(pubkey: &[u8], user: &ApiAuthenticatedUser) -> Result<(), String> {
    if pubkey != user.pubkey {
        Err("Unauthorized".to_string())
    } else {
        Ok(())
    }
}

#[OpenApi]
impl MainApi {
    /// Health check endpoint
    ///
    /// Returns the health status of the API server
    #[oai(path = "/health", method = "get", tag = "ApiTags::System")]
    async fn health(&self) -> Json<HealthResponse> {
        let environment =
            std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        Json(HealthResponse {
            success: true,
            message: "Decent Cloud API is running".to_string(),
            environment,
        })
    }

    /// Register account
    ///
    /// Creates a new account with a username and initial public key
    /// Uses header-based authentication: X-Public-Key, X-Signature, X-Timestamp, X-Nonce
    #[oai(path = "/accounts", method = "post", tag = "ApiTags::Accounts")]
    async fn register_account(
        &self,
        db: Data<&Arc<Database>>,
        req: poem_openapi::payload::PlainText<String>,
        #[oai(name = "X-Public-Key")] public_key_header: poem_openapi::param::Header<String>,
        #[oai(name = "X-Signature")] signature_header: poem_openapi::param::Header<String>,
        #[oai(name = "X-Timestamp")] timestamp_header: poem_openapi::param::Header<String>,
        #[oai(name = "X-Nonce")] nonce_header: poem_openapi::param::Header<String>,
    ) -> Json<ApiResponse<crate::database::accounts::AccountWithKeys>> {
        // Parse request body
        let body_data: RegisterAccountRequest = match serde_json::from_str(&req.0) {
            Ok(data) => data,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid request body: {}", e)),
                })
            }
        };

        // Validate username
        let username = match crate::validation::validate_account_username(&body_data.username) {
            Ok(u) => u,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        // Decode public key
        let public_key = match hex::decode(&body_data.public_key) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid public key format".to_string()),
                })
            }
        };

        if public_key.len() != 32 {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Public key must be 32 bytes".to_string()),
            });
        }

        // Verify public key from body matches header
        if body_data.public_key != public_key_header.0 {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Public key mismatch between body and header".to_string()),
            });
        }

        // Parse nonce
        let nonce = match uuid::Uuid::parse_str(&nonce_header.0) {
            Ok(n) => n,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid nonce format".to_string()),
                })
            }
        };

        // Parse timestamp
        let timestamp = match timestamp_header.0.parse::<i64>() {
            Ok(ts) => ts,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid timestamp format".to_string()),
                })
            }
        };

        // Verify signature
        if let Err(e) = crate::auth::verify_request_signature(
            &public_key_header.0,
            &signature_header.0,
            &timestamp_header.0,
            &nonce_header.0,
            "POST",
            "/api/v1/accounts",
            req.0.as_bytes(),
        ) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Signature verification failed: {}", e)),
            });
        }

        // Check nonce hasn't been used
        match db.check_nonce_exists(&nonce, 10).await {
            Ok(true) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Nonce already used (replay attack)".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Database error: {}", e)),
                })
            }
            _ => {}
        }

        // Check if username is already taken
        match db.get_account_by_username(&username).await {
            Ok(Some(_)) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Username already taken".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Database error: {}", e)),
                })
            }
            _ => {}
        }

        // Create account
        match db.create_account(&username, &public_key).await {
            Ok(account) => {
                // Insert audit record
                if let Err(e) = db
                    .insert_signature_audit(
                        Some(&account.id),
                        "register_account",
                        &req.0,
                        &hex::decode(&signature_header.0).unwrap(),
                        &public_key,
                        timestamp,
                        &nonce,
                        false,
                    )
                    .await
                {
                    tracing::warn!("Failed to insert audit record: {}", e);
                }

                // Fetch full account with keys
                match db.get_account_with_keys(&username).await {
                    Ok(Some(account_with_keys)) => Json(ApiResponse {
                        success: true,
                        data: Some(account_with_keys),
                        error: None,
                    }),
                    Ok(None) => Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some("Account created but not found".to_string()),
                    }),
                    Err(e) => Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                    }),
                }
            }
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get account
    ///
    /// Returns account information with all public keys
    #[oai(
        path = "/accounts/:username",
        method = "get",
        tag = "ApiTags::Accounts"
    )]
    async fn get_account(
        &self,
        db: Data<&Arc<Database>>,
        username: Path<String>,
    ) -> Json<ApiResponse<crate::database::accounts::AccountWithKeys>> {
        match db.get_account_with_keys(&username.0).await {
            Ok(Some(account)) => Json(ApiResponse {
                success: true,
                data: Some(account),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Account not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Add public key to account
    ///
    /// Adds a new public key to an existing account (requires authentication)
    #[oai(
        path = "/accounts/:username/keys",
        method = "post",
        tag = "ApiTags::Accounts"
    )]
    async fn add_account_key(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        username: Path<String>,
        req: Json<AddAccountKeyRequest>,
    ) -> Json<ApiResponse<crate::database::accounts::PublicKeyInfo>> {
        // Get account
        let account = match db.get_account_by_username(&username.0).await {
            Ok(Some(acc)) => acc,
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
                    error: Some(e.to_string()),
                })
            }
        };

        // Verify authenticated user owns this account
        match db.get_account_id_by_public_key(&auth.pubkey).await {
            Ok(Some(acc_id)) if acc_id == account.id => {}
            Ok(Some(_)) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Authenticated key does not belong to this account".to_string()),
                })
            }
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Authenticated key not found or not active".to_string()),
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

        // Decode new public key
        let new_public_key = match hex::decode(&req.new_public_key) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid new public key format".to_string()),
                })
            }
        };

        if new_public_key.len() != 32 {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Public key must be 32 bytes".to_string()),
            });
        }

        // Add new key
        match db.add_account_key(&account.id, &new_public_key).await {
            Ok(key) => Json(ApiResponse {
                success: true,
                data: Some(crate::database::accounts::PublicKeyInfo {
                    id: hex::encode(&key.id),
                    public_key: hex::encode(&key.public_key),
                    added_at: key.added_at,
                    is_active: key.is_active != 0,
                    disabled_at: key.disabled_at,
                    disabled_by_key_id: key.disabled_by_key_id.map(|id| hex::encode(&id)),
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

    /// Remove public key from account
    ///
    /// Removes (disables) a public key from an account (requires authentication)
    #[oai(
        path = "/accounts/:username/keys/:key_id",
        method = "delete",
        tag = "ApiTags::Accounts"
    )]
    async fn remove_account_key(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        username: Path<String>,
        key_id: Path<String>,
    ) -> Json<ApiResponse<crate::database::accounts::PublicKeyInfo>> {
        // Get account
        let account = match db.get_account_by_username(&username.0).await {
            Ok(Some(acc)) => acc,
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
                    error: Some(e.to_string()),
                })
            }
        };

        // Decode key ID
        let key_id_bytes = match hex::decode(&key_id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid key ID format".to_string()),
                })
            }
        };

        // Verify authenticated key belongs to account and find its ID
        let signing_key_id = match db.get_account_id_by_public_key(&auth.pubkey).await {
            Ok(Some(acc_id)) if acc_id == account.id => {
                // Find the signing key ID
                let keys = match db.get_account_keys(&account.id).await {
                    Ok(keys) => keys,
                    Err(e) => {
                        return Json(ApiResponse {
                            success: false,
                            data: None,
                            error: Some(e.to_string()),
                        })
                    }
                };
                match keys.iter().find(|k| k.public_key == auth.pubkey) {
                    Some(k) => k.id.clone(),
                    None => {
                        return Json(ApiResponse {
                            success: false,
                            data: None,
                            error: Some("Authenticated key not found".to_string()),
                        })
                    }
                }
            }
            Ok(Some(_)) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Authenticated key does not belong to this account".to_string()),
                })
            }
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Authenticated key not found or not active".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        // Disable key
        match db.disable_account_key(&key_id_bytes, &signing_key_id).await {
            Ok(_) => {
                // Fetch updated key
                let keys = match db.get_account_keys(&account.id).await {
                    Ok(keys) => keys,
                    Err(e) => {
                        return Json(ApiResponse {
                            success: false,
                            data: None,
                            error: Some(e.to_string()),
                        })
                    }
                };

                match keys.iter().find(|k| k.id == key_id_bytes) {
                    Some(key) => Json(ApiResponse {
                        success: true,
                        data: Some(crate::database::accounts::PublicKeyInfo {
                            id: hex::encode(&key.id),
                            public_key: hex::encode(&key.public_key),
                            added_at: key.added_at,
                            is_active: key.is_active != 0,
                            disabled_at: key.disabled_at,
                            disabled_by_key_id: key
                                .disabled_by_key_id
                                .as_ref()
                                .map(|id| hex::encode(id)),
                        }),
                        error: None,
                    }),
                    None => Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some("Key not found after disable".to_string()),
                    }),
                }
            }
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Admin: Disable an account key
    ///
    /// Allows an admin to disable a specific key for an account. Useful for security incidents or account recovery.
    #[oai(
        path = "/admin/accounts/:username/keys/:key_id/disable",
        method = "post",
        tag = "ApiTags::Admin"
    )]
    async fn admin_disable_key(
        &self,
        db: Data<&Arc<Database>>,
        _admin: AdminAuthenticatedUser,
        username: Path<String>,
        key_id: Path<String>,
        req: Json<AdminDisableKeyRequest>,
    ) -> Json<ApiResponse<crate::database::accounts::PublicKeyInfo>> {
        // Get account
        let account = match db.get_account_by_username(&username.0).await {
            Ok(Some(acc)) => acc,
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
                    error: Some(e.to_string()),
                })
            }
        };

        // Decode key ID
        let key_id_bytes = match hex::decode(&key_id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid key ID format".to_string()),
                })
            }
        };

        // Disable key (admin action bypasses last-key check)
        // Create a dummy disabled_by_key_id for admin actions
        let admin_marker_id = [0u8; 16]; // All zeros indicates admin action

        match db
            .disable_account_key(&key_id_bytes, &admin_marker_id)
            .await
        {
            Ok(_) => {
                // Insert audit record with is_admin_action = true
                if let Err(e) = db
                    .insert_signature_audit(
                        Some(&account.id),
                        "admin_disable_key",
                        &serde_json::to_string(&req.0).unwrap_or_default(),
                        &[0u8; 64], // No signature for admin action
                        &_admin.pubkey,
                        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                        &uuid::Uuid::new_v4(),
                        true, // is_admin_action
                    )
                    .await
                {
                    tracing::warn!("Failed to insert admin audit record: {}", e);
                }

                // Fetch updated key
                let keys = match db.get_account_keys(&account.id).await {
                    Ok(keys) => keys,
                    Err(e) => {
                        return Json(ApiResponse {
                            success: false,
                            data: None,
                            error: Some(e.to_string()),
                        })
                    }
                };

                let disabled_key = keys.iter().find(|k| k.id == key_id_bytes).map(|k| {
                    crate::database::accounts::PublicKeyInfo {
                        id: hex::encode(&k.id),
                        public_key: hex::encode(&k.public_key),
                        added_at: k.added_at,
                        is_active: k.is_active != 0,
                        disabled_at: k.disabled_at,
                        disabled_by_key_id: k.disabled_by_key_id.as_ref().map(|id| hex::encode(id)),
                    }
                });

                match disabled_key {
                    Some(key) => Json(ApiResponse {
                        success: true,
                        data: Some(key),
                        error: None,
                    }),
                    None => Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some("Key not found after disable".to_string()),
                    }),
                }
            }
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Admin: Add recovery key to account
    ///
    /// Allows an admin to add a new public key to an account. Used for account recovery when user loses all keys.
    #[oai(
        path = "/admin/accounts/:username/recovery-key",
        method = "post",
        tag = "ApiTags::Admin"
    )]
    async fn admin_add_recovery_key(
        &self,
        db: Data<&Arc<Database>>,
        _admin: AdminAuthenticatedUser,
        username: Path<String>,
        req: Json<AdminAddRecoveryKeyRequest>,
    ) -> Json<ApiResponse<crate::database::accounts::PublicKeyInfo>> {
        // Get account
        let account = match db.get_account_by_username(&username.0).await {
            Ok(Some(acc)) => acc,
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
                    error: Some(e.to_string()),
                })
            }
        };

        // Decode public key
        let public_key = match hex::decode(&req.public_key) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid public key format".to_string()),
                })
            }
        };

        // Add recovery key
        match db.add_account_key(&account.id, &public_key).await {
            Ok(key) => {
                // Insert audit record with is_admin_action = true
                if let Err(e) = db
                    .insert_signature_audit(
                        Some(&account.id),
                        "admin_add_recovery_key",
                        &serde_json::to_string(&req.0).unwrap_or_default(),
                        &[0u8; 64], // No signature for admin action
                        &_admin.pubkey,
                        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                        &uuid::Uuid::new_v4(),
                        true, // is_admin_action
                    )
                    .await
                {
                    tracing::warn!("Failed to insert admin audit record: {}", e);
                }

                Json(ApiResponse {
                    success: true,
                    data: Some(crate::database::accounts::PublicKeyInfo {
                        id: hex::encode(&key.id),
                        public_key: hex::encode(&key.public_key),
                        added_at: key.added_at,
                        is_active: key.is_active != 0,
                        disabled_at: key.disabled_at,
                        disabled_by_key_id: key.disabled_by_key_id.map(|id| hex::encode(id)),
                    }),
                    error: None,
                })
            }
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// List all providers
    ///
    /// Returns a paginated list of registered providers
    #[oai(path = "/providers", method = "get", tag = "ApiTags::Providers")]
    async fn list_providers(
        &self,
        db: Data<&Arc<Database>>,
        #[oai(default = "default_limit")] limit: poem_openapi::param::Query<i64>,
        #[oai(default)] offset: poem_openapi::param::Query<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::providers::ProviderProfile>>> {
        match db.list_providers(limit.0, offset.0).await {
            Ok(providers) => Json(ApiResponse {
                success: true,
                data: Some(providers),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get active providers
    ///
    /// Returns providers that have checked in within the specified number of days
    #[oai(
        path = "/providers/active/:days",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_active_providers(
        &self,
        db: Data<&Arc<Database>>,
        days: Path<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::providers::ProviderProfile>>> {
        match db.get_active_providers(days.0).await {
            Ok(providers) => Json(ApiResponse {
                success: true,
                data: Some(providers),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Search offerings
    ///
    /// Search for offerings with optional filters
    #[oai(path = "/offerings", method = "get", tag = "ApiTags::Offerings")]
    async fn search_offerings(
        &self,
        db: Data<&Arc<Database>>,
        #[oai(default = "default_limit")] limit: poem_openapi::param::Query<i64>,
        #[oai(default)] offset: poem_openapi::param::Query<i64>,
        product_type: poem_openapi::param::Query<Option<String>>,
        country: poem_openapi::param::Query<Option<String>>,
        #[oai(default = "default_false")] in_stock_only: poem_openapi::param::Query<bool>,
    ) -> Json<ApiResponse<Vec<crate::database::offerings::Offering>>> {
        let search_params = crate::database::offerings::SearchOfferingsParams {
            product_type: product_type.0.as_deref(),
            country: country.0.as_deref(),
            in_stock_only: in_stock_only.0,
            limit: limit.0,
            offset: offset.0,
        };

        match db.search_offerings(search_params).await {
            Ok(offerings) => Json(ApiResponse {
                success: true,
                data: Some(offerings),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get active validators
    ///
    /// Returns validators that have checked in within the specified number of days
    #[oai(
        path = "/validators/active/:days",
        method = "get",
        tag = "ApiTags::Validators"
    )]
    async fn get_active_validators(
        &self,
        db: Data<&Arc<Database>>,
        days: Path<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::providers::Validator>>> {
        match db.get_active_validators(days.0).await {
            Ok(validators) => Json(ApiResponse {
                success: true,
                data: Some(validators),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider profile
    ///
    /// Returns profile information for a specific provider
    #[oai(
        path = "/providers/:pubkey",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_profile(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<crate::database::providers::ProviderProfile>> {
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
                })
            }
        };

        match db.get_provider_profile(&pubkey_bytes).await {
            Ok(Some(profile)) => Json(ApiResponse {
                success: true,
                data: Some(profile),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Provider not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider contacts
    ///
    /// Returns contact information for a specific provider
    #[oai(
        path = "/providers/:pubkey/contacts",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_contacts(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::providers::ProviderContact>>> {
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
                })
            }
        };

        match db.get_provider_contacts(&pubkey_bytes).await {
            Ok(contacts) => Json(ApiResponse {
                success: true,
                data: Some(contacts),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider stats
    ///
    /// Returns statistics for a specific provider
    #[oai(
        path = "/providers/:pubkey/stats",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_stats(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<crate::database::stats::ProviderStats>> {
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
                })
            }
        };

        match db.get_provider_stats(&pubkey_bytes).await {
            Ok(stats) => Json(ApiResponse {
                success: true,
                data: Some(stats),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get offering by ID
    ///
    /// Returns details of a specific offering
    #[oai(path = "/offerings/:id", method = "get", tag = "ApiTags::Offerings")]
    async fn get_offering(
        &self,
        db: Data<&Arc<Database>>,
        id: Path<i64>,
    ) -> Json<ApiResponse<crate::database::offerings::Offering>> {
        match db.get_offering(id.0).await {
            Ok(Some(offering)) => Json(ApiResponse {
                success: true,
                data: Some(offering),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Offering not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// List contracts
    ///
    /// Returns a paginated list of all contracts
    #[oai(path = "/contracts", method = "get", tag = "ApiTags::Contracts")]
    async fn list_contracts(
        &self,
        db: Data<&Arc<Database>>,
        #[oai(default = "default_limit")] limit: poem_openapi::param::Query<i64>,
        #[oai(default)] offset: poem_openapi::param::Query<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::Contract>>> {
        match db.list_contracts(limit.0, offset.0).await {
            Ok(contracts) => Json(ApiResponse {
                success: true,
                data: Some(contracts),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get contract by ID
    ///
    /// Returns details of a specific contract
    #[oai(path = "/contracts/:id", method = "get", tag = "ApiTags::Contracts")]
    async fn get_contract(
        &self,
        db: Data<&Arc<Database>>,
        id: Path<String>,
    ) -> Json<ApiResponse<crate::database::contracts::Contract>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        match db.get_contract(&contract_id).await {
            Ok(Some(contract)) => Json(ApiResponse {
                success: true,
                data: Some(contract),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Contract not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get user contracts
    ///
    /// Returns contracts for a specific user (as requester)
    #[oai(
        path = "/users/:pubkey/contracts",
        method = "get",
        tag = "ApiTags::Users"
    )]
    async fn get_user_contracts(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::Contract>>> {
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
                })
            }
        };

        match db.get_user_contracts(&pubkey_bytes).await {
            Ok(contracts) => Json(ApiResponse {
                success: true,
                data: Some(contracts),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider contracts
    ///
    /// Returns contracts for a specific provider
    #[oai(
        path = "/providers/:pubkey/contracts",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_contracts(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::Contract>>> {
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
                })
            }
        };

        match db.get_provider_contracts(&pubkey_bytes).await {
            Ok(contracts) => Json(ApiResponse {
                success: true,
                data: Some(contracts),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get user profile
    ///
    /// Returns profile information for a specific user
    #[oai(
        path = "/users/:pubkey/profile",
        method = "get",
        tag = "ApiTags::Users"
    )]
    async fn get_user_profile(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<crate::database::users::UserProfile>> {
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
                })
            }
        };

        match db.get_user_profile(&pubkey_bytes).await {
            Ok(Some(profile)) => Json(ApiResponse {
                success: true,
                data: Some(profile),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("User not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get user contacts
    ///
    /// Returns contact information for a specific user
    #[oai(
        path = "/users/:pubkey/contacts",
        method = "get",
        tag = "ApiTags::Users"
    )]
    async fn get_user_contacts(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::users::UserContact>>> {
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
                })
            }
        };

        match db.get_user_contacts(&pubkey_bytes).await {
            Ok(contacts) => Json(ApiResponse {
                success: true,
                data: Some(contacts),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get user socials
    ///
    /// Returns social media profiles for a specific user
    #[oai(
        path = "/users/:pubkey/socials",
        method = "get",
        tag = "ApiTags::Users"
    )]
    async fn get_user_socials(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::users::UserSocial>>> {
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
                })
            }
        };

        match db.get_user_socials(&pubkey_bytes).await {
            Ok(socials) => Json(ApiResponse {
                success: true,
                data: Some(socials),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get user public keys
    ///
    /// Returns public keys for a specific user
    #[oai(path = "/users/:pubkey/keys", method = "get", tag = "ApiTags::Users")]
    async fn get_user_public_keys(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::users::UserPublicKey>>> {
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
                })
            }
        };

        match db.get_user_public_keys(&pubkey_bytes).await {
            Ok(keys) => Json(ApiResponse {
                success: true,
                data: Some(keys),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get user activity
    ///
    /// Returns activity summary for a specific user
    #[oai(
        path = "/users/:pubkey/activity",
        method = "get",
        tag = "ApiTags::Users"
    )]
    async fn get_user_activity(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<crate::database::users::UserActivity>> {
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
                })
            }
        };

        match db.get_user_activity(&pubkey_bytes).await {
            Ok(activity) => Json(ApiResponse {
                success: true,
                data: Some(activity),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get recent transfers
    ///
    /// Returns a list of recent token transfers
    #[oai(path = "/transfers", method = "get", tag = "ApiTags::Transfers")]
    async fn get_recent_transfers(
        &self,
        db: Data<&Arc<Database>>,
        #[oai(default = "default_limit")] limit: poem_openapi::param::Query<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::tokens::TokenTransfer>>> {
        match db.get_recent_transfers(limit.0).await {
            Ok(transfers) => Json(ApiResponse {
                success: true,
                data: Some(transfers),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get account transfers
    ///
    /// Returns transfers for a specific account
    #[oai(
        path = "/accounts/:account/transfers",
        method = "get",
        tag = "ApiTags::Transfers"
    )]
    async fn get_account_transfers(
        &self,
        db: Data<&Arc<Database>>,
        account: Path<String>,
        #[oai(default = "default_limit")] limit: poem_openapi::param::Query<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::tokens::TokenTransfer>>> {
        match db.get_account_transfers(&account.0, limit.0).await {
            Ok(transfers) => Json(ApiResponse {
                success: true,
                data: Some(transfers),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get account balance
    ///
    /// Returns the balance for a specific account
    #[oai(
        path = "/accounts/:account/balance",
        method = "get",
        tag = "ApiTags::Transfers"
    )]
    async fn get_account_balance(
        &self,
        db: Data<&Arc<Database>>,
        account: Path<String>,
    ) -> Json<ApiResponse<i64>> {
        match db.get_account_balance(&account.0).await {
            Ok(balance) => Json(ApiResponse {
                success: true,
                data: Some(balance),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get platform stats
    ///
    /// Returns overall platform statistics
    #[oai(path = "/stats", method = "get", tag = "ApiTags::Stats")]
    async fn get_platform_stats(
        &self,
        db: Data<&Arc<Database>>,
        metadata_cache: Data<&Arc<MetadataCache>>,
    ) -> Json<ApiResponse<crate::api_handlers::PlatformOverview>> {
        use std::collections::BTreeMap;

        let base_stats = match db.get_platform_stats().await {
            Ok(stats) => stats,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        // Count providers who checked in within last 24 hours
        let cutoff_24h =
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) - 24 * 3600 * 1_000_000_000;
        let validator_count: (i64,) = match sqlx::query_as(
            "SELECT COUNT(DISTINCT pubkey) FROM provider_check_ins WHERE block_timestamp_ns > ?",
        )
        .bind(cutoff_24h)
        .fetch_one(&db.pool)
        .await
        {
            Ok(count) => count,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        // Get latest block timestamp from database
        let latest_block_timestamp_ns = match db.get_latest_block_timestamp_ns().await {
            Ok(Some(ts)) if ts > 0 => Some(ts as u64),
            _ => None,
        };

        // Get all metadata from cache as JSON
        let metadata_map = match metadata_cache.get() {
            Ok(m) => m.to_json_map(),
            Err(_) => BTreeMap::new(),
        };

        let response = crate::api_handlers::PlatformOverview {
            total_providers: base_stats.total_providers,
            active_providers: base_stats.active_providers,
            total_offerings: base_stats.total_offerings,
            total_contracts: base_stats.total_contracts,
            total_transfers: base_stats.total_transfers,
            total_volume_e9s: base_stats.total_volume_e9s,
            validator_count_24h: validator_count.0,
            latest_block_timestamp_ns,
            metadata: metadata_map,
        };

        Json(ApiResponse {
            success: true,
            data: Some(response),
            error: None,
        })
    }

    /// Get reputation
    ///
    /// Returns reputation information for a specific public key
    #[oai(path = "/reputation/:pubkey", method = "get", tag = "ApiTags::Stats")]
    async fn get_reputation(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<crate::database::stats::ReputationInfo>> {
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
                })
            }
        };

        match db.get_reputation(&pubkey_bytes).await {
            Ok(Some(reputation)) => Json(ApiResponse {
                success: true,
                data: Some(reputation),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Reputation not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Create provider offering
    ///
    /// Creates a new offering for a provider (requires authentication)
    #[oai(
        path = "/providers/:pubkey/offerings",
        method = "post",
        tag = "ApiTags::Offerings"
    )]
    async fn create_provider_offering(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        offering: Json<crate::database::offerings::Offering>,
    ) -> Json<ApiResponse<i64>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        let mut params = offering.0;
        params.id = None;
        params.pubkey = pubkey_bytes.clone();

        match db.create_offering(&pubkey_bytes, params).await {
            Ok(id) => Json(ApiResponse {
                success: true,
                data: Some(id),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update provider offering
    ///
    /// Updates an existing offering (requires authentication)
    #[oai(
        path = "/providers/:pubkey/offerings/:id",
        method = "put",
        tag = "ApiTags::Offerings"
    )]
    async fn update_provider_offering(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        id: Path<i64>,
        offering: Json<crate::database::offerings::Offering>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        let mut params = offering.0;
        params.pubkey = pubkey_bytes.clone();

        match db.update_offering(&pubkey_bytes, id.0, params).await {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Offering updated successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete provider offering
    ///
    /// Deletes an offering (requires authentication)
    #[oai(
        path = "/providers/:pubkey/offerings/:id",
        method = "delete",
        tag = "ApiTags::Offerings"
    )]
    async fn delete_provider_offering(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        id: Path<i64>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db.delete_offering(&pubkey_bytes, id.0).await {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Offering deleted successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Duplicate provider offering
    ///
    /// Creates a duplicate of an existing offering (requires authentication)
    #[oai(
        path = "/providers/:pubkey/offerings/:id/duplicate",
        method = "post",
        tag = "ApiTags::Offerings"
    )]
    async fn duplicate_provider_offering(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        id: Path<i64>,
        req: Json<DuplicateOfferingRequest>,
    ) -> Json<ApiResponse<i64>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db
            .duplicate_offering(&pubkey_bytes, id.0, req.0.new_offering_id)
            .await
        {
            Ok(new_id) => Json(ApiResponse {
                success: true,
                data: Some(new_id),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Bulk update offering status
    ///
    /// Updates stock status for multiple offerings (requires authentication)
    #[oai(
        path = "/providers/:pubkey/offerings/bulk-status",
        method = "put",
        tag = "ApiTags::Offerings"
    )]
    async fn bulk_update_provider_offerings_status(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        req: Json<BulkUpdateStatusRequest>,
    ) -> Json<ApiResponse<usize>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db
            .bulk_update_stock_status(&pubkey_bytes, &req.offering_ids, &req.stock_status)
            .await
        {
            Ok(count) => Json(ApiResponse {
                success: true,
                data: Some(count),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Create rental request
    ///
    /// Creates a new contract rental request (requires authentication)
    #[oai(path = "/contracts", method = "post", tag = "ApiTags::Contracts")]
    async fn create_rental_request(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        params: Json<crate::database::contracts::RentalRequestParams>,
    ) -> Json<ApiResponse<RentalRequestResponse>> {
        match db.create_rental_request(&auth.pubkey, params.0).await {
            Ok(contract_id) => Json(ApiResponse {
                success: true,
                data: Some(RentalRequestResponse {
                    contract_id: hex::encode(&contract_id),
                    message: "Rental request created successfully".to_string(),
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

    /// Extend contract
    ///
    /// Extends a contract duration (requires authentication)
    #[oai(
        path = "/contracts/:id/extend",
        method = "post",
        tag = "ApiTags::Contracts"
    )]
    async fn extend_contract(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
        req: Json<ExtendContractRequest>,
    ) -> Json<ApiResponse<ExtendContractResponse>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        match db
            .extend_contract(
                &contract_id,
                &auth.pubkey,
                req.extension_hours,
                req.memo.clone(),
            )
            .await
        {
            Ok(extension_payment_e9s) => match db.get_contract(&contract_id).await {
                Ok(Some(contract)) => Json(ApiResponse {
                    success: true,
                    data: Some(ExtendContractResponse {
                        extension_payment_e9s,
                        new_end_timestamp_ns: contract.end_timestamp_ns.unwrap_or(0),
                        message: format!("Contract extended by {} hours", req.extension_hours),
                    }),
                    error: None,
                }),
                _ => Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(
                        "Contract extended but failed to retrieve updated details".to_string(),
                    ),
                }),
            },
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Cancel contract
    ///
    /// Cancels a rental contract (requires authentication)
    #[oai(
        path = "/contracts/:id/cancel",
        method = "put",
        tag = "ApiTags::Contracts"
    )]
    async fn cancel_contract(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
        req: Json<CancelContractRequest>,
    ) -> Json<ApiResponse<String>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        match db
            .cancel_contract(&contract_id, &auth.pubkey, req.memo.as_deref())
            .await
        {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Rental request cancelled successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get pending rental requests
    ///
    /// Returns pending rental requests for the authenticated provider
    #[oai(
        path = "/provider/rental-requests/pending",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_pending_rental_requests(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::Contract>>> {
        match db.get_pending_provider_contracts(&auth.pubkey).await {
            Ok(contracts) => Json(ApiResponse {
                success: true,
                data: Some(contracts),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Respond to rental request
    ///
    /// Accept or reject a rental request (requires authentication)
    #[oai(
        path = "/provider/rental-requests/:id/respond",
        method = "post",
        tag = "ApiTags::Providers"
    )]
    async fn respond_to_rental_request(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
        req: Json<RentalResponseRequest>,
    ) -> Json<ApiResponse<String>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        let new_status = if req.accept { "accepted" } else { "rejected" };

        match db
            .update_contract_status(&contract_id, new_status, &auth.pubkey, req.memo.as_deref())
            .await
        {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some(format!("Contract {}", new_status)),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update provisioning status
    ///
    /// Updates the provisioning status of a contract (requires authentication)
    #[oai(
        path = "/provider/rental-requests/:id/provisioning",
        method = "put",
        tag = "ApiTags::Providers"
    )]
    async fn update_provisioning_status(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        id: Path<String>,
        req: Json<ProvisioningStatusRequest>,
    ) -> Json<ApiResponse<String>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        let sanitized_details = match crate::api_handlers::normalize_provisioning_details(
            &req.status,
            req.instance_details.clone(),
        ) {
            Ok(details) => details,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        match db
            .update_contract_status(&contract_id, &req.status, &auth.pubkey, None)
            .await
        {
            Ok(_) => {
                if req.status == "provisioned" {
                    if let Some(details) = sanitized_details.as_deref() {
                        if let Err(e) = db.add_provisioning_details(&contract_id, details).await {
                            return Json(ApiResponse {
                                success: false,
                                data: None,
                                error: Some(format!(
                                    "Status updated but failed to save details: {}",
                                    e
                                )),
                            });
                        }
                    }
                }
                Json(ApiResponse {
                    success: true,
                    data: Some("Provisioning status updated".to_string()),
                    error: None,
                })
            }
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update user profile
    ///
    /// Updates user profile information (requires authentication)
    #[oai(
        path = "/users/:pubkey/profile",
        method = "put",
        tag = "ApiTags::Users"
    )]
    async fn update_user_profile(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        req: Json<UpdateUserProfileRequest>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db
            .upsert_user_profile(
                &auth.pubkey,
                req.display_name.as_deref(),
                req.bio.as_deref(),
                req.avatar_url.as_deref(),
            )
            .await
        {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Profile updated successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Upsert user contact
    ///
    /// Adds or updates a user contact (requires authentication)
    #[oai(
        path = "/users/:pubkey/contacts",
        method = "post",
        tag = "ApiTags::Users"
    )]
    async fn upsert_user_contact(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        req: Json<UpsertContactRequest>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        if let Err(e) = crate::validation::validate_contact_type(&req.contact_type) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
        }

        if let Err(e) =
            crate::validation::validate_contact_value(&req.contact_type, &req.contact_value)
        {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
        }

        match db
            .upsert_user_contact(
                &auth.pubkey,
                &req.contact_type,
                &req.contact_value,
                req.verified,
            )
            .await
        {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Contact added successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete user contact
    ///
    /// Deletes a user contact (requires authentication)
    #[oai(
        path = "/users/:pubkey/contacts/:contact_id",
        method = "delete",
        tag = "ApiTags::Users"
    )]
    async fn delete_user_contact(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        contact_id: Path<i64>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db.delete_user_contact(&auth.pubkey, contact_id.0).await {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Contact deleted successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Upsert user social
    ///
    /// Adds or updates a user social profile (requires authentication)
    #[oai(
        path = "/users/:pubkey/socials",
        method = "post",
        tag = "ApiTags::Users"
    )]
    async fn upsert_user_social(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        req: Json<UpsertSocialRequest>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        if let Err(e) = crate::validation::validate_social_platform(&req.platform) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
        }

        if let Err(e) = crate::validation::validate_social_username(&req.username) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
        }

        if let Some(ref url) = req.profile_url {
            if let Err(e) = crate::validation::validate_url(url) {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                });
            }
        }

        match db
            .upsert_user_social(
                &auth.pubkey,
                &req.platform,
                &req.username,
                req.profile_url.as_deref(),
            )
            .await
        {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Social account added successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete user social
    ///
    /// Deletes a user social profile (requires authentication)
    #[oai(
        path = "/users/:pubkey/socials/:social_id",
        method = "delete",
        tag = "ApiTags::Users"
    )]
    async fn delete_user_social(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        social_id: Path<i64>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db.delete_user_social(&auth.pubkey, social_id.0).await {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Social account deleted successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Add user public key
    ///
    /// Adds a new public key for a user (requires authentication)
    #[oai(path = "/users/:pubkey/keys", method = "post", tag = "ApiTags::Users")]
    async fn add_user_public_key(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        req: Json<AddPublicKeyRequest>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        if let Err(e) = crate::validation::validate_public_key(&req.key_type, &req.key_data) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
        }

        match db
            .add_user_public_key(
                &auth.pubkey,
                &req.key_type,
                &req.key_data,
                req.key_fingerprint.as_deref(),
                req.label.as_deref(),
            )
            .await
        {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Public key added successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete user public key
    ///
    /// Deletes a user public key (requires authentication)
    #[oai(
        path = "/users/:pubkey/keys/:key_id",
        method = "delete",
        tag = "ApiTags::Users"
    )]
    async fn delete_user_public_key(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        key_id: Path<i64>,
    ) -> Json<ApiResponse<String>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db.delete_user_public_key(&auth.pubkey, key_id.0).await {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Public key deleted successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get contract extensions
    ///
    /// Returns extension history for a contract
    #[oai(
        path = "/contracts/:id/extensions",
        method = "get",
        tag = "ApiTags::Contracts"
    )]
    async fn get_contract_extensions(
        &self,
        db: Data<&Arc<Database>>,
        id: Path<String>,
    ) -> Json<self::ApiResponse<Vec<crate::database::contracts::ContractExtension>>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(self::ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        match db.get_contract_extensions(&contract_id).await {
            Ok(extensions) => Json(self::ApiResponse {
                success: true,
                data: Some(extensions),
                error: None,
            }),
            Err(e) => Json(self::ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider offerings
    ///
    /// Returns all offerings for a specific provider
    #[oai(
        path = "/providers/:pubkey/offerings",
        method = "get",
        tag = "ApiTags::Offerings"
    )]
    async fn get_provider_offerings(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<self::ApiResponse<Vec<crate::database::offerings::Offering>>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(self::ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        match db.get_provider_offerings(&pubkey_bytes).await {
            Ok(offerings) => Json(self::ApiResponse {
                success: true,
                data: Some(offerings),
                error: None,
            }),
            Err(e) => Json(self::ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Export provider offerings as CSV
    ///
    /// Returns all offerings for a provider in CSV format (requires authentication)
    #[oai(
        path = "/providers/:pubkey/offerings/export",
        method = "get",
        tag = "ApiTags::Offerings"
    )]
    async fn export_provider_offerings_csv(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> poem_openapi::payload::PlainText<String> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => return poem_openapi::payload::PlainText("Invalid pubkey format".to_string()),
        };

        if check_authorization(&pubkey_bytes, &auth).is_err() {
            return poem_openapi::payload::PlainText("Unauthorized".to_string());
        }

        match db.get_provider_offerings(&pubkey_bytes).await {
            Ok(offerings) => {
                let mut csv_writer = csv::Writer::from_writer(vec![]);

                // Write header
                let _ = csv_writer.write_record([
                    "offering_id",
                    "offer_name",
                    "description",
                    "product_page_url",
                    "currency",
                    "monthly_price",
                    "setup_fee",
                    "visibility",
                    "product_type",
                    "virtualization_type",
                    "billing_interval",
                    "stock_status",
                    "processor_brand",
                    "processor_amount",
                    "processor_cores",
                    "processor_speed",
                    "processor_name",
                    "memory_error_correction",
                    "memory_type",
                    "memory_amount",
                    "hdd_amount",
                    "total_hdd_capacity",
                    "ssd_amount",
                    "total_ssd_capacity",
                    "unmetered_bandwidth",
                    "uplink_speed",
                    "traffic",
                    "datacenter_country",
                    "datacenter_city",
                    "datacenter_latitude",
                    "datacenter_longitude",
                    "control_panel",
                    "gpu_name",
                    "min_contract_hours",
                    "max_contract_hours",
                    "payment_methods",
                    "features",
                    "operating_systems",
                ]);

                // Write data rows
                for offering in offerings {
                    let _ = csv_writer.write_record([
                        &offering.offering_id,
                        &offering.offer_name,
                        &offering.description.unwrap_or_default(),
                        &offering.product_page_url.unwrap_or_default(),
                        &offering.currency,
                        &offering.monthly_price.to_string(),
                        &offering.setup_fee.to_string(),
                        &offering.visibility,
                        &offering.product_type,
                        &offering.virtualization_type.unwrap_or_default(),
                        &offering.billing_interval,
                        &offering.stock_status,
                        &offering.processor_brand.unwrap_or_default(),
                        &offering
                            .processor_amount
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering
                            .processor_cores
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering.processor_speed.unwrap_or_default(),
                        &offering.processor_name.unwrap_or_default(),
                        &offering.memory_error_correction.unwrap_or_default(),
                        &offering.memory_type.unwrap_or_default(),
                        &offering.memory_amount.unwrap_or_default(),
                        &offering
                            .hdd_amount
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering.total_hdd_capacity.unwrap_or_default(),
                        &offering
                            .ssd_amount
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering.total_ssd_capacity.unwrap_or_default(),
                        &offering.unmetered_bandwidth.to_string(),
                        &offering.uplink_speed.unwrap_or_default(),
                        &offering.traffic.map(|v| v.to_string()).unwrap_or_default(),
                        &offering.datacenter_country,
                        &offering.datacenter_city,
                        &offering
                            .datacenter_latitude
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering
                            .datacenter_longitude
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering.control_panel.unwrap_or_default(),
                        &offering.gpu_name.unwrap_or_default(),
                        &offering
                            .min_contract_hours
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering
                            .max_contract_hours
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering.payment_methods.unwrap_or_default(),
                        &offering.features.unwrap_or_default(),
                        &offering.operating_systems.unwrap_or_default(),
                    ]);
                }

                match csv_writer.into_inner() {
                    Ok(csv_data) => poem_openapi::payload::PlainText(
                        String::from_utf8_lossy(&csv_data).to_string(),
                    ),
                    Err(e) => {
                        poem_openapi::payload::PlainText(format!("CSV generation error: {}", e))
                    }
                }
            }
            Err(e) => poem_openapi::payload::PlainText(format!("Error: {}", e)),
        }
    }

    /// Get CSV template for offerings
    ///
    /// Returns a CSV template with example offerings
    #[oai(
        path = "/offerings/template",
        method = "get",
        tag = "ApiTags::Offerings"
    )]
    async fn get_offerings_csv_template(
        &self,
        db: Data<&Arc<Database>>,
    ) -> poem_openapi::payload::PlainText<String> {
        let mut csv_writer = csv::Writer::from_writer(vec![]);

        // Write header
        let _ = csv_writer.write_record([
            "offering_id",
            "offer_name",
            "description",
            "product_page_url",
            "currency",
            "monthly_price",
            "setup_fee",
            "visibility",
            "product_type",
            "virtualization_type",
            "billing_interval",
            "stock_status",
            "processor_brand",
            "processor_amount",
            "processor_cores",
            "processor_speed",
            "processor_name",
            "memory_error_correction",
            "memory_type",
            "memory_amount",
            "hdd_amount",
            "total_hdd_capacity",
            "ssd_amount",
            "total_ssd_capacity",
            "unmetered_bandwidth",
            "uplink_speed",
            "traffic",
            "datacenter_country",
            "datacenter_city",
            "datacenter_latitude",
            "datacenter_longitude",
            "control_panel",
            "gpu_name",
            "min_contract_hours",
            "max_contract_hours",
            "payment_methods",
            "features",
            "operating_systems",
        ]);

        // Get example offerings
        if let Ok(offerings) = db.get_example_offerings().await {
            for offering in offerings {
                let _ = csv_writer.write_record([
                    &offering.offering_id,
                    &offering.offer_name,
                    &offering.description.unwrap_or_default(),
                    &offering.product_page_url.unwrap_or_default(),
                    &offering.currency,
                    &offering.monthly_price.to_string(),
                    &offering.setup_fee.to_string(),
                    &offering.visibility,
                    &offering.product_type,
                    &offering.virtualization_type.unwrap_or_default(),
                    &offering.billing_interval,
                    &offering.stock_status,
                    &offering.processor_brand.unwrap_or_default(),
                    &offering
                        .processor_amount
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    &offering
                        .processor_cores
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    &offering.processor_speed.unwrap_or_default(),
                    &offering.processor_name.unwrap_or_default(),
                    &offering.memory_error_correction.unwrap_or_default(),
                    &offering.memory_type.unwrap_or_default(),
                    &offering.memory_amount.unwrap_or_default(),
                    &offering
                        .hdd_amount
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    &offering.total_hdd_capacity.unwrap_or_default(),
                    &offering
                        .ssd_amount
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    &offering.total_ssd_capacity.unwrap_or_default(),
                    &offering.unmetered_bandwidth.to_string(),
                    &offering.uplink_speed.unwrap_or_default(),
                    &offering.traffic.map(|v| v.to_string()).unwrap_or_default(),
                    &offering.datacenter_country,
                    &offering.datacenter_city,
                    &offering
                        .datacenter_latitude
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    &offering
                        .datacenter_longitude
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    &offering.control_panel.unwrap_or_default(),
                    &offering.gpu_name.unwrap_or_default(),
                    &offering
                        .min_contract_hours
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    &offering
                        .max_contract_hours
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    &offering.payment_methods.unwrap_or_default(),
                    &offering.features.unwrap_or_default(),
                    &offering.operating_systems.unwrap_or_default(),
                ]);
            }
        }

        match csv_writer.into_inner() {
            Ok(csv_data) => {
                poem_openapi::payload::PlainText(String::from_utf8_lossy(&csv_data).to_string())
            }
            Err(e) => poem_openapi::payload::PlainText(format!("CSV generation error: {}", e)),
        }
    }

    /// Import provider offerings from CSV
    ///
    /// Imports offerings from CSV format (requires authentication)
    #[oai(
        path = "/providers/:pubkey/offerings/import",
        method = "post",
        tag = "ApiTags::Offerings"
    )]
    async fn import_provider_offerings_csv(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        #[oai(default)] upsert: poem_openapi::param::Query<bool>,
        csv_data: poem_openapi::payload::PlainText<String>,
    ) -> Json<ApiResponse<CsvImportResult>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db
            .import_offerings_csv(&pubkey_bytes, &csv_data.0, upsert.0)
            .await
        {
            Ok((success_count, errors)) => {
                let result = CsvImportResult {
                    success_count,
                    errors: errors
                        .into_iter()
                        .map(|(row, message)| CsvImportError { row, message })
                        .collect(),
                };
                Json(ApiResponse {
                    success: true,
                    data: Some(result),
                    error: None,
                })
            }
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }
}

// Request/response types for account endpoints
#[derive(Debug, Deserialize, Object)]
pub struct RegisterAccountRequest {
    pub username: String,
    #[serde(rename = "publicKey")]
    pub public_key: String,
}

#[derive(Debug, Deserialize, Object)]
pub struct AddAccountKeyRequest {
    #[serde(rename = "newPublicKey")]
    pub new_public_key: String,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct AdminDisableKeyRequest {
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct AdminAddRecoveryKeyRequest {
    #[serde(rename = "publicKey")]
    pub public_key: String,
    pub reason: String,
}

// API Tags for grouping endpoints
#[derive(poem_openapi::Tags)]
enum ApiTags {
    /// System endpoints
    System,
    /// Account management endpoints
    Accounts,
    /// Admin operations endpoints
    Admin,
    /// Provider management endpoints
    Providers,
    /// Validator management endpoints
    Validators,
    /// Offering management endpoints
    Offerings,
    /// Contract management endpoints
    Contracts,
    /// User profile endpoints
    Users,
    /// Token transfer endpoints
    Transfers,
    /// Platform statistics endpoints
    Stats,
}
