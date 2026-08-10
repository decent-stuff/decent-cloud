//! Account key, device-name, and external SSH/GPG key endpoints.
//!
//! Extracted from `accounts.rs` (#444 large-file split). These handlers all
//! carry the `ApiTags::Accounts` tag and form a self-contained cluster with no
//! dependency on private helpers or local types defined in `accounts.rs`.
//! Registration is unchanged from the consumer's perspective: `AccountKeysApi`
//! is combined with the other `*Api` types in `openapi::create_combined_api`,
//! and every path, method, tag, and schema below is identical to the pre-split
//! API.

use super::common::{
    decode_hex_path, decode_pubkey, AddAccountExternalKeyRequest, AddAccountKeyRequest,
    ApiResponse, ApiTags, UpdateDeviceNameRequest,
};
use crate::auth::ApiAuthenticatedUser;
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, OpenApi};
use std::sync::Arc;

pub struct AccountKeysApi;

#[OpenApi]
impl AccountKeysApi {
    // ==================== Account Keys ====================
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
        let new_public_key = match decode_pubkey(&req.new_public_key) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        // Add new key
        match db.add_account_key(&account.id, &new_public_key).await {
            Ok(key) => Json(ApiResponse {
                success: true,
                data: Some(crate::database::accounts::PublicKeyInfo {
                    id: hex::encode(&key.id),
                    public_key: hex::encode(&key.public_key),
                    added_at: key.added_at,
                    is_active: key.is_active,
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
        let key_id_bytes = match decode_hex_path(&key_id.0, "key id") {
            Ok(id) => id,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
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
                            is_active: key.is_active,
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
        let key_id_bytes = match decode_hex_path(&key_id.0, "key id") {
            Ok(id) => id,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
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
                    is_active: key.is_active,
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

    /// Get account external keys
    ///
    /// Returns SSH/GPG keys for an account (public endpoint)
    #[oai(
        path = "/accounts/:username/external-keys",
        method = "get",
        tag = "ApiTags::Accounts"
    )]
    async fn get_account_external_keys(
        &self,
        db: Data<&Arc<Database>>,
        username: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::users::AccountExternalKey>>> {
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

        // Get external keys (public, no auth required)
        match db.get_account_external_keys(&account.id).await {
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

    /// Add account external key
    ///
    /// Adds an SSH or GPG key to an account (requires authentication)
    #[oai(
        path = "/accounts/:username/external-keys",
        method = "post",
        tag = "ApiTags::Accounts"
    )]
    async fn add_account_external_key(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        username: Path<String>,
        req: Json<AddAccountExternalKeyRequest>,
    ) -> Json<ApiResponse<String>> {
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
                    error: Some("Unauthorized: Cannot modify another user's keys".to_string()),
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

        // Validate key
        if let Err(e) = crate::validation::validate_public_key(&req.key_type, &req.key_data) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
        }

        // Add external key
        match db
            .add_account_external_key(
                &account.id,
                &req.key_type,
                &req.key_data,
                req.key_fingerprint.as_deref(),
                req.label.as_deref(),
            )
            .await
        {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("External key added successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete account external key
    ///
    /// Deletes an SSH or GPG key from an account (requires authentication)
    #[oai(
        path = "/accounts/:username/external-keys/:key_id",
        method = "delete",
        tag = "ApiTags::Accounts"
    )]
    async fn delete_account_external_key(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        username: Path<String>,
        key_id: Path<i64>,
    ) -> Json<ApiResponse<String>> {
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
                    error: Some("Unauthorized: Cannot modify another user's keys".to_string()),
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

        // Delete external key
        match db.delete_account_external_key(&account.id, key_id.0).await {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("External key deleted successfully".to_string()),
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

#[cfg(test)]
mod tests {
    use crate::database::accounts::PublicKeyInfo;
    use crate::openapi::common::{
        AddAccountExternalKeyRequest, AddAccountKeyRequest, UpdateDeviceNameRequest,
    };

    // ---- AddAccountKeyRequest ----

    #[test]
    fn test_add_account_key_request_camel_case() {
        let json = r#"{"newPublicKey":"cafebabe"}"#;
        let req: AddAccountKeyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.new_public_key, "cafebabe");
    }

    // ---- AddAccountExternalKeyRequest ----

    #[test]
    fn test_add_account_external_key_request_full() {
        let json = r#"{"keyType":"ssh-ed25519","keyData":"ssh-ed25519 AAAA...","keyFingerprint":"SHA256:abc","label":"laptop"}"#;
        let req: AddAccountExternalKeyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.key_type, "ssh-ed25519");
        assert_eq!(req.key_data, "ssh-ed25519 AAAA...");
        assert_eq!(req.key_fingerprint.as_deref(), Some("SHA256:abc"));
        assert_eq!(req.label.as_deref(), Some("laptop"));
    }

    #[test]
    fn test_add_account_external_key_request_optional_fields_none() {
        let json = r#"{"keyType":"gpg","keyData":"-----BEGIN PGP PUBLIC KEY BLOCK-----"}"#;
        let req: AddAccountExternalKeyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.key_type, "gpg");
        assert!(req.key_fingerprint.is_none());
        assert!(req.label.is_none());
    }

    // ---- UpdateDeviceNameRequest ----

    #[test]
    fn test_update_device_name_request_with_name() {
        let json = r#"{"deviceName":"My Laptop"}"#;
        let req: UpdateDeviceNameRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.device_name.as_deref(), Some("My Laptop"));
    }

    #[test]
    fn test_update_device_name_request_clear() {
        let json = r#"{"deviceName":null}"#;
        let req: UpdateDeviceNameRequest = serde_json::from_str(json).unwrap();
        assert!(req.device_name.is_none());
    }

    // ---- PublicKeyInfo ----

    #[test]
    fn test_public_key_info_inactive_key_serialization() {
        let key = PublicKeyInfo {
            id: "keyid".to_string(),
            public_key: "pubkey".to_string(),
            added_at: 1_700_000_000,
            is_active: false,
            device_name: None,
            disabled_at: Some(1_700_001_000),
            disabled_by_key_id: Some("otherid".to_string()),
        };
        let json = serde_json::to_value(&key).unwrap();
        assert_eq!(json["isActive"], false);
        assert_eq!(json["disabledAt"], 1_700_001_000_i64);
        assert_eq!(json["disabledByKeyId"], "otherid");
    }
}
