use super::common::{
    AdminAccountDeletionSummary, AdminAddRecoveryKeyRequest, AdminDisableKeyRequest,
    AdminProcessPayoutRequest, AdminSendTestEmailRequest, AdminSetAccountEmailRequest,
    AdminSetAdminStatusRequest, AdminSetEmailVerifiedRequest, ApiResponse, ApiTags,
};
use crate::{
    auth::AdminAuthenticatedUser,
    database::email::{EmailQueueEntry, EmailStats},
    database::Database,
    email_service::EmailService,
    icpay_client::IcpayClient,
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

/// Response type for pending payment releases, with provider pubkey as hex string
#[derive(Debug, Serialize, poem_openapi::Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct PendingReleaseInfo {
    pub provider_pubkey_hex: String,
    pub total_pending_e9s: i64,
    pub release_count: i64,
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
                    tracing::warn!("Failed to insert admin audit record: {:#}", e);
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
                    tracing::warn!("Failed to insert admin audit record: {:#}", e);
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

    /// Admin: List pending payment releases
    ///
    /// Returns all providers with pending releases ready for payout, aggregated by provider.
    #[oai(
        path = "/admin/payment-releases",
        method = "get",
        tag = "ApiTags::Admin"
    )]
    async fn admin_list_pending_releases(
        &self,
        db: Data<&Arc<Database>>,
        _admin: AdminAuthenticatedUser,
    ) -> Json<ApiResponse<Vec<PendingReleaseInfo>>> {
        match db.get_providers_with_pending_releases().await {
            Ok(providers) => Json(ApiResponse {
                success: true,
                data: Some(
                    providers
                        .into_iter()
                        .map(|p| PendingReleaseInfo {
                            provider_pubkey_hex: hex::encode(&p.provider_pubkey),
                            total_pending_e9s: p.total_pending_e9s,
                            release_count: p.release_count,
                        })
                        .collect(),
                ),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Admin: Process provider payout
    ///
    /// Aggregates all released funds for a provider and triggers payout to their wallet.
    #[oai(path = "/admin/payouts", method = "post", tag = "ApiTags::Admin")]
    async fn admin_process_payout(
        &self,
        db: Data<&Arc<Database>>,
        _admin: AdminAuthenticatedUser,
        req: Json<AdminProcessPayoutRequest>,
    ) -> Json<ApiResponse<String>> {
        // Decode provider pubkey
        let provider_pubkey = match hex::decode(&req.provider_pubkey) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid provider_pubkey format".to_string()),
                })
            }
        };

        // Get pending releases for provider
        let releases = match db.get_provider_pending_releases(&provider_pubkey).await {
            Ok(r) => r,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to get pending releases: {}", e)),
                })
            }
        };

        if releases.is_empty() {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("No pending releases for this provider".to_string()),
            });
        }

        // Calculate total amount
        let total_amount_e9s: i64 = releases.iter().map(|r| r.amount_e9s).sum();

        // Try to create payout via ICPay
        let payout_id = match IcpayClient::new() {
            Ok(icpay_client) => {
                match icpay_client
                    .create_payout(&req.wallet_address, total_amount_e9s)
                    .await
                {
                    Ok(id) => id,
                    Err(e) => {
                        // Log error but don't fail - mark with generated ID
                        tracing::error!("Failed to create ICPay payout: {:#}", e);
                        format!("pending_{}", uuid::Uuid::new_v4())
                    }
                }
            }
            Err(e) => {
                // ICPay client not configured - mark as pending
                tracing::warn!("ICPay client not configured: {:#}", e);
                format!("pending_{}", uuid::Uuid::new_v4())
            }
        };

        // Mark releases as paid out
        let release_ids: Vec<i64> = releases.iter().map(|r| r.id).collect();
        match db.mark_releases_paid_out(&release_ids, &payout_id).await {
            Ok(()) => Json(ApiResponse {
                success: true,
                data: Some(format!(
                    "Payout {} created for provider {} (amount: {} e9s, {} releases)",
                    payout_id,
                    req.provider_pubkey,
                    total_amount_e9s,
                    release_ids.len()
                )),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to mark releases as paid out: {}", e)),
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
}

#[cfg(test)]
mod tests {
    use super::{AdminAccountInfo, AdminAccountListResponse};
    use crate::database::email::{EmailQueueEntry, EmailStats};
    use crate::openapi::common::{
        AdminAddRecoveryKeyRequest, AdminDisableKeyRequest, AdminProcessPayoutRequest,
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

    // ---- AdminProcessPayoutRequest ----

    #[test]
    fn test_admin_process_payout_request_camel_case() {
        let json = r#"{"providerPubkey":"aabb1122","walletAddress":"wallet-xyz"}"#;
        let req: AdminProcessPayoutRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.provider_pubkey, "aabb1122");
        assert_eq!(req.wallet_address, "wallet-xyz");
    }

    // ---- PendingReleaseInfo ----

    #[test]
    fn test_pending_release_info_serialization() {
        let info = super::PendingReleaseInfo {
            provider_pubkey_hex: "deadbeef".to_string(),
            total_pending_e9s: 1_000_000_000,
            release_count: 3,
        };
        let v = serde_json::to_value(&info).unwrap();
        assert_eq!(v["providerPubkeyHex"], "deadbeef");
        assert_eq!(v["totalPendingE9s"], 1_000_000_000i64);
        assert_eq!(v["releaseCount"], 3);
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
        assert!(json["error"].is_null());
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
        assert!(json["data"].is_null());
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
}
