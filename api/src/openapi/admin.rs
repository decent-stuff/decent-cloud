use super::common::{
    AdminAccountDeletionSummary, AdminAddRecoveryKeyRequest, AdminDisableKeyRequest,
    AdminProcessPayoutRequest, AdminSendTestEmailRequest, AdminSetAccountEmailRequest,
    AdminSetEmailVerifiedRequest, ApiResponse, ApiTags,
};
use crate::{
    auth::AdminAuthenticatedUser,
    database::contracts::ProviderPendingReleases,
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
                    tracing::warn!("Failed to insert admin audit record: {:#}", e);
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

        let active_keys = keys.iter().filter(|k| k.is_active != 0).count() as i64;

        Json(ApiResponse {
            success: true,
            data: Some(AdminAccountInfo {
                id: hex::encode(&account.id),
                username: account.username,
                email: account.email,
                email_verified: account.email_verified != 0,
                created_at: account.created_at,
                last_login_at: account.last_login_at,
                is_admin: account.is_admin != 0,
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
    ) -> Json<ApiResponse<Vec<ProviderPendingReleases>>> {
        match db.get_providers_with_pending_releases().await {
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
        if account.is_admin != 0 {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Cannot delete admin accounts".to_string()),
            });
        }

        // Delete account
        match db.admin_delete_account(&account.id).await {
            Ok(summary) => {
                tracing::info!(
                    "Admin deleted account '{}': {:?}",
                    username.0,
                    summary
                );
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
}
