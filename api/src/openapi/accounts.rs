use super::common::{
    AddAccountKeyRequest, ApiResponse, ApiTags, RegisterAccountRequest, UpdateDeviceNameRequest,
};
use crate::{auth::ApiAuthenticatedUser, database::Database};
use poem::web::Data;
use poem_openapi::{param::Path, param::Query, payload::Binary, payload::Json, OpenApi};
use std::sync::Arc;

pub struct AccountsApi;

#[OpenApi]
impl AccountsApi {
    /// Register account
    ///
    /// Creates a new account with a username and initial public key
    /// Uses header-based authentication: X-Public-Key, X-Signature, X-Timestamp, X-Nonce
    #[oai(path = "/accounts", method = "post", tag = "ApiTags::Accounts")]
    async fn register_account(
        &self,
        db: Data<&Arc<Database>>,
        req: Binary<Vec<u8>>,
        #[oai(name = "X-Public-Key")] public_key_header: poem_openapi::param::Header<String>,
        #[oai(name = "X-Signature")] signature_header: poem_openapi::param::Header<String>,
        #[oai(name = "X-Timestamp")] timestamp_header: poem_openapi::param::Header<String>,
        #[oai(name = "X-Nonce")] nonce_header: poem_openapi::param::Header<String>,
    ) -> Json<ApiResponse<crate::database::accounts::AccountWithKeys>> {
        // Use the original request body bytes for signature verification (avoid re-serialization)
        let req_body_bytes = req.0;

        // Parse request body
        let body_data: RegisterAccountRequest = match serde_json::from_slice(&req_body_bytes) {
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
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!(
                        "Invalid public key hex: {} (value: {})",
                        e, &body_data.public_key
                    )),
                })
            }
        };

        if public_key.len() != 32 {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!(
                    "Public key must be 32 bytes, got {} bytes",
                    public_key.len()
                )),
            });
        }

        // Verify public key from body matches header
        if body_data.public_key != public_key_header.0 {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!(
                    "Public key mismatch: body='{}' header='{}'",
                    &body_data.public_key, &public_key_header.0
                )),
            });
        }

        // Decode signature for later audit use
        let signature_bytes = match hex::decode(&signature_header.0) {
            Ok(sig) => sig,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid signature hex: {}", e)),
                })
            }
        };

        // Parse nonce
        let nonce = match uuid::Uuid::parse_str(&nonce_header.0) {
            Ok(n) => n,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!(
                        "Invalid nonce format (expected UUID): {} (value: {})",
                        e, &nonce_header.0
                    )),
                })
            }
        };

        // Parse timestamp
        let timestamp = match timestamp_header.0.parse::<i64>() {
            Ok(ts) => ts,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!(
                        "Invalid timestamp (expected nanoseconds): {} (value: {})",
                        e, &timestamp_header.0
                    )),
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
            &req_body_bytes,
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
                let req_body_str = String::from_utf8_lossy(&req_body_bytes);
                if let Err(e) = db
                    .insert_signature_audit(
                        Some(&account.id),
                        "register_account",
                        &req_body_str,
                        &signature_bytes,
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

    /// Search account by public key
    ///
    /// Returns account if public key is registered, null if not found
    #[oai(path = "/accounts", method = "get", tag = "ApiTags::Accounts")]
    async fn search_account_by_public_key(
        &self,
        db: Data<&Arc<Database>>,
        #[oai(name = "publicKey")] public_key: Query<String>,
    ) -> Json<ApiResponse<crate::database::accounts::AccountWithKeys>> {
        let public_key_bytes = match hex::decode(&public_key.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!(
                        "Invalid public key hex: {} (value: {})",
                        e, &public_key.0
                    )),
                })
            }
        };

        if public_key_bytes.len() != 32 {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!(
                    "Public key must be 32 bytes, got {} bytes",
                    public_key_bytes.len()
                )),
            });
        }

        match db
            .get_account_with_keys_by_public_key(&public_key_bytes)
            .await
        {
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
                    device_name: key.device_name,
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
                            device_name: key.device_name.clone(),
                            disabled_at: key.disabled_at,
                            disabled_by_key_id: key.disabled_by_key_id.as_ref().map(hex::encode),
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

    /// Update device name for a public key
    ///
    /// Updates the device name for a public key (requires authentication)
    #[oai(
        path = "/accounts/:username/keys/:key_id",
        method = "put",
        tag = "ApiTags::Accounts"
    )]
    async fn update_device_name(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        username: Path<String>,
        key_id: Path<String>,
        req: Json<UpdateDeviceNameRequest>,
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

        // Verify authenticated key belongs to account
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

        // Verify the key being updated belongs to this account
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

        if !keys.iter().any(|k| k.id == key_id_bytes) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Key does not belong to this account".to_string()),
            });
        }

        // Update device name
        match db
            .update_device_name(&key_id_bytes, req.device_name.as_deref())
            .await
        {
            Ok(key) => Json(ApiResponse {
                success: true,
                data: Some(crate::database::accounts::PublicKeyInfo {
                    id: hex::encode(&key.id),
                    public_key: hex::encode(&key.public_key),
                    added_at: key.added_at,
                    is_active: key.is_active != 0,
                    device_name: key.device_name,
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
}
