use super::common::{
    AdminAccountDeletionSummary, AdminAddRecoveryKeyRequest, AdminDisableKeyRequest,
    AdminRefundReviewRequest, AdminSendTestEmailRequest, AdminSetAccountEmailRequest,
    AdminSetAdminStatusRequest, AdminSetEmailVerifiedRequest, ApiResponse, ApiTags,
    decode_hex_path, decode_pubkey,
};
use crate::{
    auth::AdminAuthenticatedUser,
    database::email::{EmailQueueEntry, EmailStats},
    database::refund_requests::RefundRequest,
    database::Database,
    email_service::EmailService,
};
use poem::web::Data;
use poem_openapi::{param::Path, param::Query, payload::Json, Object, OpenApi};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Admin account info for lookup responses
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AdminAccountInfo {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub created_at: i64,
    pub last_login_at: Option<i64>,
    pub is_admin: bool,
    pub active_keys: i64,
    pub total_keys: i64,
}

/// Paginated list of accounts for admin listing
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AdminAccountListResponse {
    pub accounts: Vec<AdminAccountInfo>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// A single refund request for admin review (hex-encoded byte fields).
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AdminRefundRequestInfo {
    pub id: i64,
    pub contract_id: String,
    pub requester_pubkey: String,
    pub refund_amount_e9s: i64,
    pub reason: String,
    pub status: String,
    pub user_latest_payment_e9s: i64,
    pub cap_exceeded: bool,
    pub payment_intent_id: String,
    pub currency: String,
    pub stripe_dispute_id: Option<String>,
    pub stripe_refund_id: Option<String>,
    pub created_at_ns: i64,
    pub reviewed_at_ns: Option<i64>,
    pub reviewed_by: Option<String>,
    pub review_note: Option<String>,
}

/// Paginated list of refund requests for admin listing.
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AdminRefundRequestListResponse {
    pub requests: Vec<AdminRefundRequestInfo>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

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

        // Disable key (admin action bypasses last-key check)
        // Create a dummy disabled_by_key_id for admin actions
        let admin_marker_id = [0u8; 16]; // All zeros indicates admin action

        match db
            .disable_account_key(&key_id_bytes, &admin_marker_id)
            .await
        {
            Ok(_) => {
                // Insert audit record with is_admin_action = true
                match crate::now_ns() {
                    Err(e) => tracing::warn!("Failed to get timestamp for audit record: {:#}", e),
                    Ok(now_ns) => {
                        if let Err(e) = db
                            .insert_signature_audit(
                                Some(&account.id),
                                "admin_disable_key",
                                &serde_json::to_string(&req.0).unwrap_or_default(),
                                &[0u8; 64], // No signature for admin action
                                &_admin.pubkey,
                                now_ns,
                                &uuid::Uuid::new_v4(),
                                true, // is_admin_action
                            )
                            .await
                        {
                            tracing::warn!("Failed to insert admin audit record: {:#}", e);
                        }
                    }
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
                        is_active: k.is_active,
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
        let public_key = match decode_pubkey(&req.public_key) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        // Add recovery key
        match db.add_account_key(&account.id, &public_key).await {
            Ok(key) => {
                // Insert audit record with is_admin_action = true
                match crate::now_ns() {
                    Err(e) => tracing::warn!("Failed to get timestamp for audit record: {:#}", e),
                    Ok(now_ns) => {
                        if let Err(e) = db
                            .insert_signature_audit(
                                Some(&account.id),
                                "admin_add_recovery_key",
                                &serde_json::to_string(&req.0).unwrap_or_default(),
                                &[0u8; 64], // No signature for admin action
                                &_admin.pubkey,
                                now_ns,
                                &uuid::Uuid::new_v4(),
                                true, // is_admin_action
                            )
                            .await
                        {
                            tracing::warn!("Failed to insert admin audit record: {:#}", e);
                        }
                    }
                }

                Json(ApiResponse {
                    success: true,
                    data: Some(crate::database::accounts::PublicKeyInfo {
                        id: hex::encode(&key.id),
                        public_key: hex::encode(&key.public_key),
                        added_at: key.added_at,
                        is_active: key.is_active,
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

    /// Admin: Get sent emails
    ///
    /// Returns a list of successfully sent emails.
    /// Useful for monitoring and audit purposes.
    #[oai(path = "/admin/emails/sent", method = "get", tag = "ApiTags::Admin")]
    async fn admin_get_sent_emails(
        &self,
        db: Data<&Arc<Database>>,
        _admin: AdminAuthenticatedUser,
        limit: Query<Option<i64>>,
    ) -> Json<ApiResponse<Vec<EmailQueueEntry>>> {
        let limit = limit.0.unwrap_or(50);

        match db.get_sent_emails(limit).await {
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
        let email_id_bytes = match decode_hex_path(&email_id.0, "email id") {
            Ok(id) => id,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
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

    /// Admin: Reset email for retry
    ///
    /// Resets a single email back to pending status with 0 attempts, clearing any error state.
    /// Works on any email regardless of current status.
    #[oai(
        path = "/admin/emails/reset/:email_id",
        method = "post",
        tag = "ApiTags::Admin"
    )]
    async fn admin_reset_email(
        &self,
        db: Data<&Arc<Database>>,
        _admin: AdminAuthenticatedUser,
        email_id: Path<String>,
    ) -> Json<ApiResponse<String>> {
        let email_id_bytes = match decode_hex_path(&email_id.0, "email id") {
            Ok(id) => id,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        match db.reset_email_for_retry(&email_id_bytes).await {
            Ok(found) => {
                if found {
                    Json(ApiResponse {
                        success: true,
                        data: Some("Email reset for retry".to_string()),
                        error: None,
                    })
                } else {
                    Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some("Email not found".to_string()),
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

    /// Admin: Retry all failed emails
    ///
    /// Bulk operation to reset all failed emails back to pending status.
    /// Returns the count of emails that were reset.
    #[oai(
        path = "/admin/emails/retry-all-failed",
        method = "post",
        tag = "ApiTags::Admin"
    )]
    async fn admin_retry_all_failed_emails(
        &self,
        db: Data<&Arc<Database>>,
        _admin: AdminAuthenticatedUser,
    ) -> Json<ApiResponse<u64>> {
        match db.retry_all_failed_emails().await {
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

    /// Admin: Get email queue statistics
    ///
    /// Returns statistics about the email queue including counts of pending, sent, failed, and total emails.
    #[oai(path = "/admin/emails/stats", method = "get", tag = "ApiTags::Admin")]
    async fn admin_get_email_stats(
        &self,
        db: Data<&Arc<Database>>,
        _admin: AdminAuthenticatedUser,
    ) -> Json<ApiResponse<EmailStats>> {
        match db.get_email_stats().await {
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

    /// Admin: Send test email
    ///
    /// Sends a test email to verify email configuration. The email is sent via the queue
    /// and processed by the email processor to verify the full email pipeline.
    #[oai(path = "/admin/emails/test", method = "post", tag = "ApiTags::Admin")]
    async fn admin_send_test_email(
        &self,
        email_service: Data<&Option<Arc<EmailService>>>,
        _admin: AdminAuthenticatedUser,
        req: Json<AdminSendTestEmailRequest>,
    ) -> Json<ApiResponse<String>> {
        // Check email service is configured
        let Some(email_svc) = email_service.as_ref() else {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Email service not configured (missing MAILCHANNELS_API_KEY)".into()),
            });
        };

        // Validate email
        if let Err(e) = email_utils::validate_email(&req.to_email) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Invalid email: {}", e)),
            });
        }

        // Send test email directly (not via queue) for immediate feedback
        let subject = "Decent Cloud Admin Test Email";
        let body = format!(
            "This is a test email from the Decent Cloud Admin Dashboard.\n\n\
            Timestamp: {}\n\n\
            If you received this email, your email configuration is working correctly!\n\n\
            Best regards,\n\
            The Decent Cloud Team",
            chrono::Utc::now().to_rfc3339()
        );

        match email_svc
            .send_email(
                "noreply@decent-cloud.org",
                &req.to_email,
                subject,
                &body,
                false,
            )
            .await
        {
            Ok(()) => Json(ApiResponse {
                success: true,
                data: Some(format!("Test email sent to {}", req.to_email)),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to send test email: {:#}", e)),
            }),
        }
    }

    /// Admin: Lookup account by username
    ///
    /// Returns detailed account information including email verification status and key counts.
    #[oai(
        path = "/admin/accounts/:username",
        method = "get",
        tag = "ApiTags::Admin"
    )]
    async fn admin_get_account(
        &self,
        db: Data<&Arc<Database>>,
        _admin: AdminAuthenticatedUser,
        username: Path<String>,
    ) -> Json<ApiResponse<AdminAccountInfo>> {
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

        // Get keys for counts
        let keys = match db.get_account_keys(&account.id).await {
            Ok(k) => k,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        let active_keys = keys.iter().filter(|k| k.is_active).count() as i64;

        Json(ApiResponse {
            success: true,
            data: Some(AdminAccountInfo {
                id: hex::encode(&account.id),
                username: account.username,
                email: account.email,
                email_verified: account.email_verified,
                created_at: account.created_at,
                last_login_at: account.last_login_at,
                is_admin: account.is_admin,
                active_keys,
                total_keys: keys.len() as i64,
            }),
            error: None,
        })
    }

    /// Admin: Set email verification status
    ///
    /// Allows admin to manually set email verification status for an account.
    #[oai(
        path = "/admin/accounts/:username/email-verified",
        method = "post",
        tag = "ApiTags::Admin"
    )]
    async fn admin_set_email_verified(
        &self,
        db: Data<&Arc<Database>>,
        _admin: AdminAuthenticatedUser,
        username: Path<String>,
        req: Json<AdminSetEmailVerifiedRequest>,
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

        // Update email verification status
        match db.set_email_verified(&account.id, req.verified).await {
            Ok(()) => Json(ApiResponse {
                success: true,
                data: Some(format!(
                    "Email verification status set to {} for {}",
                    req.verified, username.0
                )),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Admin: Set or clear account email
    ///
    /// Allows admin to set a new email or clear the email for an account.
    /// Setting email resets email_verified to false.
    #[oai(
        path = "/admin/accounts/:username/email",
        method = "post",
        tag = "ApiTags::Admin"
    )]
    async fn admin_set_account_email(
        &self,
        db: Data<&Arc<Database>>,
        _admin: AdminAuthenticatedUser,
        username: Path<String>,
        req: Json<AdminSetAccountEmailRequest>,
    ) -> Json<ApiResponse<String>> {
        // Validate email format if provided
        if let Some(ref email) = req.email {
            if let Err(e) = email_utils::validate_email(email) {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid email: {}", e)),
                });
            }
        }

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

        // Update email
        match db
            .admin_set_account_email(&account.id, req.email.as_deref())
            .await
        {
            Ok(()) => {
                let message = match &req.email {
                    Some(email) => format!("Email set to {} for {}", email, username.0),
                    None => format!("Email cleared for {}", username.0),
                };
                Json(ApiResponse {
                    success: true,
                    data: Some(message),
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

    /// Admin: Delete account and all associated resources
    ///
    /// Permanently deletes an account and all its associated resources including:
    /// - All offerings
    /// - Provider profile
    /// - Public keys
    /// - Email verification tokens
    /// - OAuth accounts
    ///
    /// Contracts are preserved for historical records but account references are nullified.
    #[oai(
        path = "/admin/accounts/:username",
        method = "delete",
        tag = "ApiTags::Admin"
    )]
    async fn admin_delete_account(
        &self,
        db: Data<&Arc<Database>>,
        _admin: AdminAuthenticatedUser,
        username: Path<String>,
    ) -> Json<ApiResponse<AdminAccountDeletionSummary>> {
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

        // Prevent deleting admin accounts (safety check)
        if account.is_admin {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Cannot delete admin accounts".to_string()),
            });
        }

        // Delete account
        match db.admin_delete_account(&account.id).await {
            Ok(summary) => {
                tracing::info!("Admin deleted account '{}': {:?}", username.0, summary);
                Json(ApiResponse {
                    success: true,
                    data: Some(AdminAccountDeletionSummary {
                        offerings_deleted: summary.offerings_deleted,
                        contracts_as_requester: summary.contracts_as_requester,
                        contracts_as_provider: summary.contracts_as_provider,
                        public_keys_deleted: summary.public_keys_deleted,
                        provider_profile_deleted: summary.provider_profile_deleted,
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

    /// Admin: List all accounts
    ///
    /// Returns a paginated list of all accounts with their admin status.
    #[oai(path = "/admin/accounts", method = "get", tag = "ApiTags::Admin")]
    async fn admin_list_accounts(
        &self,
        db: Data<&Arc<Database>>,
        _admin: AdminAuthenticatedUser,
        limit: Query<Option<i64>>,
        offset: Query<Option<i64>>,
    ) -> Json<ApiResponse<AdminAccountListResponse>> {
        let limit = limit.0.unwrap_or(50).min(200); // Cap at 200
        let offset = offset.0.unwrap_or(0);

        // Get total count
        let total = match db.count_accounts().await {
            Ok(t) => t,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        // Get accounts
        let accounts = match db.list_all_accounts(limit, offset).await {
            Ok(a) => a,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        // Convert to AdminAccountInfo (without key counts for efficiency)
        let account_infos: Vec<AdminAccountInfo> = accounts
            .into_iter()
            .map(|a| AdminAccountInfo {
                id: hex::encode(&a.id),
                username: a.username,
                email: a.email,
                email_verified: a.email_verified,
                created_at: a.created_at,
                last_login_at: a.last_login_at,
                is_admin: a.is_admin,
                active_keys: 0, // Not fetched for efficiency
                total_keys: 0,  // Not fetched for efficiency
            })
            .collect();

        Json(ApiResponse {
            success: true,
            data: Some(AdminAccountListResponse {
                accounts: account_infos,
                total,
                limit,
                offset,
            }),
            error: None,
        })
    }

    /// Admin: Set admin status for an account
    ///
    /// Promotes or demotes a user's admin privileges.
    #[oai(
        path = "/admin/accounts/:username/admin-status",
        method = "post",
        tag = "ApiTags::Admin"
    )]
    async fn admin_set_admin_status(
        &self,
        db: Data<&Arc<Database>>,
        admin: AdminAuthenticatedUser,
        username: Path<String>,
        req: Json<AdminSetAdminStatusRequest>,
    ) -> Json<ApiResponse<String>> {
        // Get account to check it exists and get current state
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

        // Prevent admin from demoting themselves
        if !req.is_admin && account.id == admin.account_id {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Cannot remove your own admin privileges".to_string()),
            });
        }

        // Update admin status
        match db.set_admin_status(&username.0, req.is_admin).await {
            Ok(()) => {
                let action = if req.is_admin { "granted" } else { "revoked" };
                tracing::info!(
                    "Admin {} {} admin privileges for {}",
                    hex::encode(&admin.pubkey),
                    action,
                    username.0
                );
                Json(ApiResponse {
                    success: true,
                    data: Some(format!("Admin privileges {} for {}", action, username.0)),
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

    /// Admin: List refund requests
    ///
    /// Returns a paginated list of refund requests, optionally filtered by status.
    /// Default filter is `pending` (the queue needing review); pass `status=all` for everything.
    #[oai(path = "/admin/refund-requests", method = "get", tag = "ApiTags::Admin")]
    async fn admin_list_refund_requests(
        &self,
        db: Data<&Arc<Database>>,
        _admin: AdminAuthenticatedUser,
        status: Query<Option<String>>,
        limit: Query<Option<i64>>,
        offset: Query<Option<i64>>,
    ) -> Json<ApiResponse<AdminRefundRequestListResponse>> {
        let limit = limit.0.unwrap_or(50).clamp(1, 200);
        let offset = offset.0.unwrap_or(0).max(0);

        // Normalize status filter: "all" (case-insensitive) → None, otherwise pass through.
        let status_filter = status
            .0
            .as_deref()
            .filter(|s| !s.eq_ignore_ascii_case("all"));

        let total = match db.count_refund_requests(status_filter).await {
            Ok(t) => t,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        let rows = match db.list_refund_requests(status_filter, limit, offset).await {
            Ok(r) => r,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        let requests: Vec<AdminRefundRequestInfo> = rows
            .into_iter()
            .map(refund_request_to_info)
            .collect();

        Json(ApiResponse {
            success: true,
            data: Some(AdminRefundRequestListResponse {
                requests,
                total,
                limit,
                offset,
            }),
            error: None,
        })
    }

    /// Admin: Approve a pending refund request
    ///
    /// Issues the Stripe refund for a pending refund request. The DB layer
    /// atomically flips status pending→approved and then calls Stripe.
    #[oai(
        path = "/admin/refund-requests/:id/approve",
        method = "post",
        tag = "ApiTags::Admin"
    )]
    async fn admin_approve_refund_request(
        &self,
        db: Data<&Arc<Database>>,
        admin: AdminAuthenticatedUser,
        id: Path<i64>,
        req: Json<AdminRefundReviewRequest>,
    ) -> Json<ApiResponse<AdminRefundRequestInfo>> {
        let stripe_client = crate::stripe_client::stripe_client_or_warn();
        let row = match db
            .approve_refund_request(
                id.0,
                &admin.pubkey,
                req.note.as_deref(),
                stripe_client.as_ref(),
            )
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        tracing::info!(
            request_id = row.id,
            admin_pubkey = %hex::encode(&admin.pubkey),
            "Admin approved refund request {}",
            row.id
        );

        Json(ApiResponse {
            success: true,
            data: Some(refund_request_to_info(row)),
            error: None,
        })
    }

    /// Admin: Decline a pending refund request
    ///
    /// Marks the refund request as declined. No Stripe refund is issued.
    #[oai(
        path = "/admin/refund-requests/:id/decline",
        method = "post",
        tag = "ApiTags::Admin"
    )]
    async fn admin_decline_refund_request(
        &self,
        db: Data<&Arc<Database>>,
        admin: AdminAuthenticatedUser,
        id: Path<i64>,
        req: Json<AdminRefundReviewRequest>,
    ) -> Json<ApiResponse<AdminRefundRequestInfo>> {
        let row = match db
            .decline_refund_request(id.0, &admin.pubkey, req.note.as_deref())
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        tracing::info!(
            request_id = row.id,
            admin_pubkey = %hex::encode(&admin.pubkey),
            "Admin declined refund request {}",
            row.id
        );

        Json(ApiResponse {
            success: true,
            data: Some(refund_request_to_info(row)),
            error: None,
        })
    }
}

/// Convert a `RefundRequest` DB row to the API-facing `AdminRefundRequestInfo`,
/// hex-encoding the byte fields.
fn refund_request_to_info(r: RefundRequest) -> AdminRefundRequestInfo {
    AdminRefundRequestInfo {
        id: r.id,
        contract_id: hex::encode(&r.contract_id),
        requester_pubkey: hex::encode(&r.requester_pubkey),
        refund_amount_e9s: r.refund_amount_e9s,
        reason: r.reason,
        status: r.status,
        user_latest_payment_e9s: r.user_latest_payment_e9s,
        cap_exceeded: r.cap_exceeded,
        payment_intent_id: r.payment_intent_id,
        currency: r.currency,
        stripe_dispute_id: r.stripe_dispute_id,
        stripe_refund_id: r.stripe_refund_id,
        created_at_ns: r.created_at_ns,
        reviewed_at_ns: r.reviewed_at_ns,
        reviewed_by: r.reviewed_by.as_ref().map(hex::encode),
        review_note: r.review_note,
    }
}

#[cfg(test)]
mod tests {
    use super::{AdminAccountInfo, AdminAccountListResponse, AdminRefundRequestInfo, AdminRefundRequestListResponse, refund_request_to_info};
    use crate::database::email::{EmailQueueEntry, EmailStats};
    use crate::database::refund_requests::RefundRequest;
    use crate::openapi::common::{
        AdminAddRecoveryKeyRequest, AdminDisableKeyRequest, AdminRefundReviewRequest,
        AdminSendTestEmailRequest, AdminSetAccountEmailRequest, AdminSetAdminStatusRequest,
        AdminSetEmailVerifiedRequest, ApiResponse,
    };

    // ---- AdminDisableKeyRequest ----

    #[test]
    fn test_admin_disable_key_request_serialization() {
        let req = AdminDisableKeyRequest {
            reason: "security incident".to_string(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["reason"], "security incident");
    }

    #[test]
    fn test_admin_disable_key_request_deserialization() {
        let json = r#"{"reason":"compromised device"}"#;
        let req: AdminDisableKeyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.reason, "compromised device");
    }

    // ---- AdminAddRecoveryKeyRequest ----

    #[test]
    fn test_admin_add_recovery_key_request_camel_case() {
        let json = r#"{"publicKey":"aabbccddeeff","reason":"lost all keys"}"#;
        let req: AdminAddRecoveryKeyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.public_key, "aabbccddeeff");
        assert_eq!(req.reason, "lost all keys");
    }

    #[test]
    fn test_admin_add_recovery_key_request_serialization() {
        let req = AdminAddRecoveryKeyRequest {
            public_key: "cafebabe".to_string(),
            reason: "admin recovery".to_string(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["publicKey"], "cafebabe");
        assert_eq!(json["reason"], "admin recovery");
    }

    // ---- AdminSendTestEmailRequest ----

    #[test]
    fn test_admin_send_test_email_request_camel_case() {
        let json = r#"{"toEmail":"test@example.com"}"#;
        let req: AdminSendTestEmailRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.to_email, "test@example.com");
    }

    // ---- AdminSetEmailVerifiedRequest ----

    #[test]
    fn test_admin_set_email_verified_request_true() {
        let json = r#"{"verified":true}"#;
        let req: AdminSetEmailVerifiedRequest = serde_json::from_str(json).unwrap();
        assert!(req.verified);
    }

    #[test]
    fn test_admin_set_email_verified_request_false() {
        let json = r#"{"verified":false}"#;
        let req: AdminSetEmailVerifiedRequest = serde_json::from_str(json).unwrap();
        assert!(!req.verified);
    }

    // ---- AdminSetAccountEmailRequest ----

    #[test]
    fn test_admin_set_account_email_request_with_email() {
        let json = r#"{"email":"admin@example.com"}"#;
        let req: AdminSetAccountEmailRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.email.as_deref(), Some("admin@example.com"));
    }

    #[test]
    fn test_admin_set_account_email_request_clear_email() {
        let json = r#"{"email":null}"#;
        let req: AdminSetAccountEmailRequest = serde_json::from_str(json).unwrap();
        assert!(req.email.is_none());
    }

    // ---- AdminSetAdminStatusRequest ----

    #[test]
    fn test_admin_set_admin_status_request_grant() {
        let json = r#"{"isAdmin":true}"#;
        let req: AdminSetAdminStatusRequest = serde_json::from_str(json).unwrap();
        assert!(req.is_admin);
    }

    #[test]
    fn test_admin_set_admin_status_request_revoke() {
        let json = r#"{"isAdmin":false}"#;
        let req: AdminSetAdminStatusRequest = serde_json::from_str(json).unwrap();
        assert!(!req.is_admin);
    }

    // ---- AdminAccountInfo ----

    fn sample_admin_account_info() -> AdminAccountInfo {
        AdminAccountInfo {
            id: "hex-id-here".to_string(),
            username: "alice".to_string(),
            email: Some("alice@example.com".to_string()),
            email_verified: true,
            created_at: 1_700_000_000,
            last_login_at: Some(1_700_100_000),
            is_admin: false,
            active_keys: 2,
            total_keys: 3,
        }
    }

    #[test]
    fn test_admin_account_info_camel_case_serialization() {
        let info = sample_admin_account_info();
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["id"], "hex-id-here");
        assert_eq!(json["username"], "alice");
        assert_eq!(json["email"], "alice@example.com");
        assert_eq!(json["emailVerified"], true);
        assert_eq!(json["createdAt"], 1_700_000_000_i64);
        assert_eq!(json["lastLoginAt"], 1_700_100_000_i64);
        assert_eq!(json["isAdmin"], false);
        assert_eq!(json["activeKeys"], 2_i64);
        assert_eq!(json["totalKeys"], 3_i64);
    }

    #[test]
    fn test_admin_account_info_no_email_no_login() {
        let info = AdminAccountInfo {
            email: None,
            last_login_at: None,
            ..sample_admin_account_info()
        };
        let json = serde_json::to_value(&info).unwrap();
        assert!(json["email"].is_null());
        assert!(json["lastLoginAt"].is_null());
    }

    // ---- AdminAccountListResponse ----

    #[test]
    fn test_admin_account_list_response_serialization() {
        let resp = AdminAccountListResponse {
            accounts: vec![sample_admin_account_info()],
            total: 42,
            limit: 50,
            offset: 0,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["total"], 42_i64);
        assert_eq!(json["limit"], 50_i64);
        assert_eq!(json["offset"], 0_i64);
        let accounts = json["accounts"].as_array().unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0]["username"], "alice");
    }

    // ---- ApiResponse<AdminAccountInfo> ----

    #[test]
    fn test_api_response_admin_account_info_success() {
        let resp = ApiResponse {
            success: true,
            data: Some(sample_admin_account_info()),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["username"], "alice");
        assert!(json.get("error").is_none());
    }

    #[test]
    fn test_api_response_admin_account_info_not_found() {
        let resp: ApiResponse<AdminAccountInfo> = ApiResponse {
            success: false,
            data: None,
            error: Some("Account not found".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], false);
        assert_eq!(json["error"], "Account not found");
        assert!(json.get("data").is_none());
    }

    // ---- ApiResponse<AdminAccountListResponse> ----

    #[test]
    fn test_api_response_admin_account_list_pagination_fields() {
        let list = AdminAccountListResponse {
            accounts: vec![],
            total: 100,
            limit: 25,
            offset: 50,
        };
        let resp = ApiResponse {
            success: true,
            data: Some(list),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["data"]["total"], 100_i64);
        assert_eq!(json["data"]["limit"], 25_i64);
        assert_eq!(json["data"]["offset"], 50_i64);
    }

    // ---- EmailStats ----

    #[test]
    fn test_email_stats_serialization_camel_case() {
        let stats = EmailStats {
            pending: 5,
            sent: 100,
            failed: 3,
            total: 108,
        };
        let json = serde_json::to_value(&stats).unwrap();
        assert_eq!(json["pending"], 5_i64);
        assert_eq!(json["sent"], 100_i64);
        assert_eq!(json["failed"], 3_i64);
        assert_eq!(json["total"], 108_i64);
    }

    #[test]
    fn test_api_response_email_stats_success() {
        let stats = EmailStats {
            pending: 0,
            sent: 50,
            failed: 0,
            total: 50,
        };
        let resp = ApiResponse {
            success: true,
            data: Some(stats),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["sent"], 50_i64);
    }

    // ---- ApiResponse<Vec<EmailQueueEntry>> ----

    fn sample_email_queue_entry() -> EmailQueueEntry {
        EmailQueueEntry {
            id: vec![0u8; 16],
            to_addr: "user@example.com".to_string(),
            from_addr: "noreply@decent-cloud.org".to_string(),
            subject: "Test".to_string(),
            body: "Hello".to_string(),
            is_html: false,
            email_type: "general".to_string(),
            status: "failed".to_string(),
            attempts: 12,
            max_attempts: 12,
            last_error: Some("SMTP timeout".to_string()),
            created_at: 1_700_000_000,
            last_attempted_at: Some(1_700_001_000),
            sent_at: None,
            related_account_id: None,
            user_notified_retry: true,
            user_notified_gave_up: true,
        }
    }

    #[test]
    fn test_email_queue_entry_camel_case_serialization() {
        let entry = sample_email_queue_entry();
        let json = serde_json::to_value(&entry).unwrap();
        // id and related_account_id are #[serde(skip)]
        assert!(json.get("id").is_none());
        assert_eq!(json["toAddr"], "user@example.com");
        assert_eq!(json["fromAddr"], "noreply@decent-cloud.org");
        assert_eq!(json["subject"], "Test");
        assert_eq!(json["status"], "failed");
        assert_eq!(json["attempts"], 12_i64);
        assert_eq!(json["lastError"], "SMTP timeout");
        assert_eq!(json["userNotifiedGaveUp"], true);
    }

    #[test]
    fn test_api_response_failed_emails_list() {
        let entries = vec![sample_email_queue_entry()];
        let resp = ApiResponse {
            success: true,
            data: Some(entries),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        let data = json["data"].as_array().unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0]["toAddr"], "user@example.com");
    }

    // ---- ApiResponse<u64> for retry-all-failed ----

    #[test]
    fn test_api_response_u64_retry_count() {
        let resp = ApiResponse {
            success: true,
            data: Some(7u64),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"], 7_u64);
    }

    // ---- AdminAccountDeletionSummary ----

    #[test]
    fn test_admin_account_deletion_summary_serialization() {
        use crate::openapi::common::AdminAccountDeletionSummary;
        let summary = AdminAccountDeletionSummary {
            offerings_deleted: 3,
            contracts_as_requester: 1,
            contracts_as_provider: 2,
            public_keys_deleted: 5,
            provider_profile_deleted: true,
        };
        let json = serde_json::to_value(&summary).unwrap();
        assert_eq!(json["offeringsDeleted"], 3_i64);
        assert_eq!(json["contractsAsRequester"], 1_i64);
        assert_eq!(json["contractsAsProvider"], 2_i64);
        assert_eq!(json["publicKeysDeleted"], 5_i64);
        assert_eq!(json["providerProfileDeleted"], true);
    }

    #[test]
    fn test_api_response_deletion_summary_success() {
        use crate::openapi::common::AdminAccountDeletionSummary;
        let resp = ApiResponse {
            success: true,
            data: Some(AdminAccountDeletionSummary {
                offerings_deleted: 0,
                contracts_as_requester: 0,
                contracts_as_provider: 0,
                public_keys_deleted: 1,
                provider_profile_deleted: false,
            }),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["publicKeysDeleted"], 1_i64);
    }

    // ---- Admin action: prevent deleting admin account logic ----

    #[test]
    fn test_admin_account_info_is_admin_flag() {
        let info = AdminAccountInfo {
            is_admin: true,
            ..sample_admin_account_info()
        };
        let json = serde_json::to_value(&info).unwrap();
        // Handler checks account.is_admin before deletion - verify the flag serializes
        assert_eq!(json["isAdmin"], true);
    }

    // ---- hex::decode used in handlers - validate the pattern directly ----

    #[test]
    fn test_hex_decode_valid_key_id() {
        // Simulates the key_id decoding in admin_disable_key / admin_retry_failed_email
        let hex_str = "aabbccddeeff00112233445566778899";
        let result = hex::decode(hex_str);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 16);
    }

    #[test]
    fn test_hex_decode_invalid_key_id_returns_error() {
        let result = hex::decode("not-valid-hex!");
        assert!(result.is_err());
    }

    #[test]
    fn test_hex_decode_provider_pubkey_32_bytes() {
        // Simulates admin_process_payout provider_pubkey decode
        let hex_str = "a".repeat(64); // 32 bytes
        let result = hex::decode(&hex_str);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 32);
    }

    // ---- AdminRefundReviewRequest ----

    #[test]
    fn test_admin_refund_review_request_with_note() {
        let json = r#"{"note":"approved — verified dispute"}"#;
        let req: AdminRefundReviewRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.note.as_deref(), Some("approved — verified dispute"));
    }

    #[test]
    fn test_admin_refund_review_request_without_note() {
        let json = r#"{}"#;
        let req: AdminRefundReviewRequest = serde_json::from_str(json).unwrap();
        assert!(req.note.is_none());
    }

    #[test]
    fn test_admin_refund_review_request_serialization_camel_case() {
        let req = AdminRefundReviewRequest {
            note: Some("declined — fraud".to_string()),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["note"], "declined — fraud");
    }

    // ---- AdminRefundRequestInfo (via refund_request_to_info) ----

    fn sample_refund_request() -> RefundRequest {
        RefundRequest {
            id: 42,
            contract_id: vec![0xde, 0xad, 0xbe, 0xef],
            requester_pubkey: vec![0xab; 32],
            refund_amount_e9s: 5_000_000_000, // 500 cents
            reason: "cancel".to_string(),
            status: "pending".to_string(),
            user_latest_payment_e9s: 1_000_000_000, // 100 cents
            cap_exceeded: true,
            payment_intent_id: "pi_test_123".to_string(),
            currency: "usd".to_string(),
            stripe_dispute_id: None,
            stripe_refund_id: None,
            idempotency_key: "cancel:deadbeef".to_string(),
            created_at_ns: 1_700_000_000_000_000_000,
            reviewed_at_ns: None,
            reviewed_by: None,
            review_note: None,
        }
    }

    #[test]
    fn test_refund_request_to_info_hex_encoding() {
        let info = refund_request_to_info(sample_refund_request());
        assert_eq!(info.id, 42);
        assert_eq!(info.contract_id, "deadbeef");
        assert_eq!(info.requester_pubkey, hex::encode([0xab; 32]));
        assert_eq!(info.refund_amount_e9s, 5_000_000_000);
        assert_eq!(info.reason, "cancel");
        assert_eq!(info.status, "pending");
        assert_eq!(info.user_latest_payment_e9s, 1_000_000_000);
        assert!(info.cap_exceeded);
        assert_eq!(info.payment_intent_id, "pi_test_123");
        assert_eq!(info.currency, "usd");
        assert!(info.stripe_dispute_id.is_none());
        assert!(info.stripe_refund_id.is_none());
        assert_eq!(info.created_at_ns, 1_700_000_000_000_000_000);
        assert!(info.reviewed_at_ns.is_none());
        assert!(info.reviewed_by.is_none());
        assert!(info.review_note.is_none());
    }

    #[test]
    fn test_refund_request_to_info_camel_case_json() {
        let info = refund_request_to_info(sample_refund_request());
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["id"], 42_i64);
        assert_eq!(json["contractId"], "deadbeef");
        assert_eq!(json["refundAmountE9s"], 5_000_000_000_i64);
        assert_eq!(json["userLatestPaymentE9s"], 1_000_000_000_i64);
        assert_eq!(json["capExceeded"], true);
        assert_eq!(json["paymentIntentId"], "pi_test_123");
        assert_eq!(json["stripeDisputeId"], serde_json::Value::Null);
        assert_eq!(json["reviewedAtNs"], serde_json::Value::Null);
        assert_eq!(json["reviewedBy"], serde_json::Value::Null);
    }

    #[test]
    fn test_refund_request_to_info_with_review_fields() {
        let mut req = sample_refund_request();
        req.status = "approved".to_string();
        req.stripe_refund_id = Some("re_abc".to_string());
        req.reviewed_at_ns = Some(1_800_000_000_000_000_000);
        req.reviewed_by = Some(vec![0xcd; 32]);
        req.review_note = Some("LGTM".to_string());
        let info = refund_request_to_info(req);
        assert_eq!(info.status, "approved");
        assert_eq!(info.stripe_refund_id.as_deref(), Some("re_abc"));
        assert_eq!(info.reviewed_at_ns, Some(1_800_000_000_000_000_000));
        assert_eq!(info.reviewed_by.as_deref(), Some(hex::encode([0xcd; 32]).as_str()));
        assert_eq!(info.review_note.as_deref(), Some("LGTM"));
    }

    #[test]
    fn test_refund_request_to_info_with_dispute_id() {
        let mut req = sample_refund_request();
        req.reason = "dispute_lost".to_string();
        req.stripe_dispute_id = Some("dp_xyz".to_string());
        let info = refund_request_to_info(req);
        assert_eq!(info.reason, "dispute_lost");
        assert_eq!(info.stripe_dispute_id.as_deref(), Some("dp_xyz"));
    }

    // ---- AdminRefundRequestListResponse ----

    #[test]
    fn test_admin_refund_request_list_response_pagination() {
        let resp = AdminRefundRequestListResponse {
            requests: vec![refund_request_to_info(sample_refund_request())],
            total: 3,
            limit: 10,
            offset: 0,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["total"], 3_i64);
        assert_eq!(json["limit"], 10_i64);
        assert_eq!(json["offset"], 0_i64);
        let arr = json["requests"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["contractId"], "deadbeef");
    }

    #[test]
    fn test_admin_refund_request_list_response_empty() {
        let resp = AdminRefundRequestListResponse {
            requests: vec![],
            total: 0,
            limit: 50,
            offset: 0,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["requests"].as_array().unwrap().len(), 0);
        assert_eq!(json["total"], 0_i64);
    }

    // ---- ApiResponse<AdminRefundRequestInfo> ----

    #[test]
    fn test_api_response_refund_request_info_success() {
        let resp = ApiResponse {
            success: true,
            data: Some(refund_request_to_info(sample_refund_request())),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["id"], 42_i64);
        assert!(json.get("error").is_none());
    }

    #[test]
    fn test_api_response_refund_request_info_error() {
        let resp: ApiResponse<AdminRefundRequestInfo> = ApiResponse {
            success: false,
            data: None,
            error: Some("Refund request not found".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], false);
        assert_eq!(json["error"], "Refund request not found");
        assert!(json.get("data").is_none());
    }
}
