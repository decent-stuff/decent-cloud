use super::common::{AdminAddRecoveryKeyRequest, AdminDisableKeyRequest, ApiResponse, ApiTags};
use crate::{auth::AdminAuthenticatedUser, database::email::EmailQueueEntry, database::Database};
use poem::web::Data;
use poem_openapi::{param::Path, param::Query, payload::Json, OpenApi};
use std::sync::Arc;

pub struct AdminApi;

#[OpenApi]
impl AdminApi {
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
                        device_name: k.device_name.clone(),
                        disabled_at: k.disabled_at,
                        disabled_by_key_id: k.disabled_by_key_id.as_ref().map(hex::encode),
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
                        device_name: key.device_name,
                        disabled_at: key.disabled_at,
                        disabled_by_key_id: key.disabled_by_key_id.map(hex::encode),
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

    /// Admin: Get failed emails
    ///
    /// Returns a list of emails that failed permanently after all retry attempts.
    /// Useful for monitoring and manual intervention.
    #[oai(path = "/admin/emails/failed", method = "get", tag = "ApiTags::Admin")]
    async fn admin_get_failed_emails(
        &self,
        db: Data<&Arc<Database>>,
        _admin: AdminAuthenticatedUser,
        limit: Query<Option<i64>>,
    ) -> Json<ApiResponse<Vec<EmailQueueEntry>>> {
        let limit = limit.0.unwrap_or(50);

        match db.get_failed_emails(limit).await {
            Ok(emails) => Json(ApiResponse {
                success: true,
                data: Some(emails),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Admin: Retry a failed email
    ///
    /// Resets a failed email back to pending status with 0 attempts, allowing it to be retried.
    /// Use this for emails that failed due to temporary issues.
    #[oai(
        path = "/admin/emails/:email_id/retry",
        method = "post",
        tag = "ApiTags::Admin"
    )]
    async fn admin_retry_failed_email(
        &self,
        db: Data<&Arc<Database>>,
        _admin: AdminAuthenticatedUser,
        email_id: Path<String>,
    ) -> Json<ApiResponse<String>> {
        // Decode email ID
        let email_id_bytes = match hex::decode(&email_id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid email ID format".to_string()),
                })
            }
        };

        match db.retry_failed_email(&email_id_bytes).await {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Email queued for retry".to_string()),
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
