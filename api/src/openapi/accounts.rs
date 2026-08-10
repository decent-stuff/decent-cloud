use super::common::{
    ApiResponse, ApiTags, decode_hex_path, decode_pubkey, RegisterAccountRequest,
    UpdateAccountEmailRequest, UpdateAccountProfileRequest,
};
use crate::{auth::ApiAuthenticatedUser, database::email::EmailType, database::Database};
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

        // Validate email
        if let Err(e) = crate::validation::validate_email(&body_data.email) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
        }

        // Decode public key
        let public_key = match decode_pubkey(&body_data.public_key) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

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
        let signature_bytes = match decode_hex_path(&signature_header.0, "signature") {
            Ok(sig) => sig,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
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
            None,
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
        match db
            .create_account(&username, &public_key, &body_data.email)
            .await
        {
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
                    tracing::warn!("Failed to insert audit record: {:#}", e);
                }

                // Create email verification token
                match db
                    .create_email_verification_token(&account.id, &body_data.email)
                    .await
                {
                    Ok(token) => {
                        // Build verification URL
                        let base_url = std::env::var("FRONTEND_URL")
                            .unwrap_or_else(|_| "http://localhost:59010".to_string());
                        let token_hex = hex::encode(&token);
                        let verification_url =
                            format!("{}/verify-email?token={}", base_url, token_hex);

                        // Queue verification email
                        let subject = "Verify Your Decent Cloud Email";
                        let body = format!(
                            "Hello {},\n\n\
                            Thank you for registering with Decent Cloud!\n\n\
                            Please verify your email address by clicking the link below:\n\
                            {}\n\n\
                            This link will expire in 24 hours.\n\n\
                            If you did not create this account, please ignore this email.\n\n\
                            Best regards,\n\
                            The Decent Cloud Team",
                            username, verification_url
                        );

                        db.queue_email_safe(
                            Some(&body_data.email),
                            "noreply@decent-cloud.org",
                            subject,
                            &body,
                            false,
                            EmailType::Welcome, // Welcome emails: 12 attempts
                        )
                        .await;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create verification token: {:#}", e);
                    }
                }

                // Note: Chatwoot agent (inbox/team/portal) is created when provider
                // creates their first offering, not on general user registration.
                // See providers.rs::create_offering for provider onboarding logic.

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
        let public_key_bytes = match decode_pubkey(&public_key.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

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

    /// Get account profile
    ///
    /// Returns the public profile information for an account (public endpoint)
    #[oai(
        path = "/accounts/:username/profile",
        method = "get",
        tag = "ApiTags::Accounts"
    )]
    async fn get_account_profile(
        &self,
        db: Data<&Arc<Database>>,
        username: Path<String>,
    ) -> Json<ApiResponse<crate::database::accounts::AccountProfile>> {
        match db.get_account_by_username(&username.0).await {
            Ok(Some(account)) => Json(ApiResponse {
                success: true,
                data: Some(account.into()),
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

    /// Update account profile
    ///
    /// Updates profile information (requires authentication)
    #[oai(
        path = "/accounts/:username/profile",
        method = "put",
        tag = "ApiTags::Accounts"
    )]
    async fn update_account_profile(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        username: Path<String>,
        req: Json<UpdateAccountProfileRequest>,
    ) -> Json<ApiResponse<crate::database::accounts::AccountProfile>> {
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
                    error: Some("Unauthorized: Cannot modify another user's profile".to_string()),
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

        // Update profile
        match db
            .update_account_profile(
                &account.id,
                req.display_name.as_deref(),
                req.bio.as_deref(),
                req.avatar_url.as_deref(),
            )
            .await
        {
            Ok(updated_account) => Json(ApiResponse {
                success: true,
                data: Some(updated_account.into()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update account email
    ///
    /// Updates the account's email address. Resets email verification status.
    /// A verification email will be sent to the new address.
    #[oai(
        path = "/accounts/:username/email",
        method = "put",
        tag = "ApiTags::Accounts"
    )]
    async fn update_account_email(
        &self,
        db: Data<&Arc<Database>>,
        username: Path<String>,
        auth: ApiAuthenticatedUser,
        req: Json<UpdateAccountEmailRequest>,
    ) -> Json<ApiResponse<crate::database::accounts::AccountWithKeys>> {
        // Validate email format
        let email = req.email.trim();
        if let Err(e) = crate::validation::validate_email(email) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
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

        // Verify authenticated user owns this account
        match db.get_account_id_by_public_key(&auth.pubkey).await {
            Ok(Some(acc_id)) if acc_id == account.id => {}
            Ok(Some(_)) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Unauthorized: Cannot modify another user's email".to_string()),
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

        // Update email
        let updated_account = match db.update_account_email(&account.id, email).await {
            Ok(acc) => acc,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        // Queue verification email (non-blocking)
        let token = match db.create_email_verification_token(&account.id, email).await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Failed to create verification token: {:#}", e);
                // Still return success as the email was updated
                return match db.get_account_with_keys(&username.0).await {
                    Ok(Some(acc)) => Json(ApiResponse {
                        success: true,
                        data: Some(acc),
                        error: None,
                    }),
                    _ => Json(ApiResponse {
                        success: true,
                        data: None,
                        error: None,
                    }),
                };
            }
        };

        let token_hex = hex::encode(&token);
        let verification_url = format!(
            "{}/verify-email?token={}",
            std::env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "https://decent-cloud.org".to_string()),
            token_hex
        );

        db.queue_email_safe(
            Some(email),
            "noreply@decent-cloud.org",
            "Verify your email address",
            &format!(
                "Hello {}!\n\n\
                Please verify your email address by clicking the link below:\n\n\
                {}\n\n\
                This link will expire in 24 hours.\n\n\
                If you did not request this, please ignore this email.\n\n\
                Best regards,\n\
                The Decent Cloud Team",
                updated_account.username, verification_url
            ),
            false,
            EmailType::Welcome,
        )
        .await;

        // Return updated account
        match db.get_account_with_keys(&username.0).await {
            Ok(Some(acc)) => Json(ApiResponse {
                success: true,
                data: Some(acc),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Account not found after update".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get billing settings
    ///
    /// Returns saved billing information for the authenticated user
    #[oai(path = "/accounts/billing", method = "get", tag = "ApiTags::Accounts")]
    async fn get_billing_settings(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<crate::database::accounts::BillingSettings>> {
        // Get account by authenticated user's public key
        let account_id = match db.get_account_id_by_public_key(&auth.pubkey).await {
            Ok(Some(id)) => id,
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

        // Get billing settings
        match db.get_billing_settings(&account_id).await {
            Ok(settings) => Json(ApiResponse {
                success: true,
                data: Some(settings),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update billing settings
    ///
    /// Updates saved billing information for the authenticated user
    #[oai(path = "/accounts/billing", method = "put", tag = "ApiTags::Accounts")]
    async fn update_billing_settings(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        req: Json<crate::database::accounts::BillingSettings>,
    ) -> Json<ApiResponse<crate::database::accounts::BillingSettings>> {
        // Get account by authenticated user's public key
        let account_id = match db.get_account_id_by_public_key(&auth.pubkey).await {
            Ok(Some(id)) => id,
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

        // Update billing settings
        match db.update_billing_settings(&account_id, &req.0).await {
            Ok(_) => {
                // Fetch updated settings
                match db.get_billing_settings(&account_id).await {
                    Ok(settings) => Json(ApiResponse {
                        success: true,
                        data: Some(settings),
                        error: None,
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
    /// Delete my account
    ///
    /// Permanently deletes the authenticated account and all associated data.
    /// Requires confirmation in request body: {"confirm": "DELETE"}
    /// Admin accounts cannot be self-deleted.
    #[oai(path = "/accounts/me", method = "delete", tag = "ApiTags::Accounts")]
    async fn delete_my_account(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        req: Json<crate::openapi::common::DeleteAccountRequest>,
    ) -> Json<ApiResponse<crate::openapi::common::AdminAccountDeletionSummary>> {
        if req.0.confirm != "DELETE" {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Confirmation must be exactly 'DELETE'".to_string()),
            });
        }
        // Resolve account from authenticated public key
        let account = match db.get_account_id_by_public_key(&auth.pubkey).await {
            Ok(Some(account_id)) => match db.get_account(&account_id).await {
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
                        error: Some(format!("Failed to fetch account: {:#?}", e)),
                    })
                }
            },
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("No account found for this key".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to look up account: {:#?}", e)),
                })
            }
        };
        if account.is_admin {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(
                    "Admin accounts cannot be self-deleted. Contact system administrator."
                        .to_string(),
                ),
            });
        }
        match db.admin_delete_account(&account.id).await {
            Ok(summary) => {
                tracing::info!("Account '{}' self-deleted: {:?}", account.username, summary);
                Json(ApiResponse {
                    success: true,
                    data: Some(crate::openapi::common::AdminAccountDeletionSummary {
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
                error: Some(format!("Failed to delete account: {:#?}", e)),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::database::accounts::{AccountWithKeys, BillingSettings, PublicKeyInfo};
    use crate::openapi::common::{
        ApiResponse, RegisterAccountRequest, UpdateAccountEmailRequest, UpdateAccountProfileRequest,
    };

    // ---- RegisterAccountRequest ----

    #[test]
    fn test_register_account_request_camel_case_deserialization() {
        let json = r#"{"username":"alice42","publicKey":"aabbcc","email":"alice@example.com"}"#;
        let req: RegisterAccountRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.username, "alice42");
        assert_eq!(req.public_key, "aabbcc");
        assert_eq!(req.email, "alice@example.com");
    }

    #[test]
    fn test_register_account_request_serialization_round_trip() {
        let req = RegisterAccountRequest {
            username: "bob99".to_string(),
            public_key: "deadbeef".to_string(),
            email: "bob@example.com".to_string(),
        };
        let json = serde_json::to_value(&req).unwrap();
        // serde uses camelCase
        assert_eq!(json["username"], "bob99");
        assert_eq!(json["publicKey"], "deadbeef");
        assert_eq!(json["email"], "bob@example.com");
    }

    // ---- UpdateAccountProfileRequest ----

    #[test]
    fn test_update_account_profile_request_all_fields() {
        let json =
            r#"{"displayName":"Alice","bio":"A bio","avatarUrl":"https://example.com/a.png"}"#;
        let req: UpdateAccountProfileRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.display_name.as_deref(), Some("Alice"));
        assert_eq!(req.bio.as_deref(), Some("A bio"));
        assert_eq!(req.avatar_url.as_deref(), Some("https://example.com/a.png"));
    }

    #[test]
    fn test_update_account_profile_request_all_none() {
        let json = r#"{"displayName":null,"bio":null,"avatarUrl":null}"#;
        let req: UpdateAccountProfileRequest = serde_json::from_str(json).unwrap();
        assert!(req.display_name.is_none());
        assert!(req.bio.is_none());
        assert!(req.avatar_url.is_none());
    }

    // ---- UpdateAccountEmailRequest ----

    #[test]
    fn test_update_account_email_request_deserialization() {
        let json = r#"{"email":"new@example.com"}"#;
        let req: UpdateAccountEmailRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.email, "new@example.com");
    }

    // ---- ApiResponse<AccountWithKeys> ----

    fn sample_account_with_keys() -> AccountWithKeys {
        AccountWithKeys {
            id: "aabbccdd".to_string(),
            username: "alice".to_string(),
            created_at: 1_700_000_000,
            updated_at: 1_700_000_001,
            display_name: Some("Alice".to_string()),
            bio: None,
            avatar_url: None,
            profile_updated_at: None,
            public_keys: vec![PublicKeyInfo {
                id: "key001".to_string(),
                public_key: "deadbeef".to_string(),
                added_at: 1_700_000_000,
                is_active: true,
                device_name: Some("Laptop".to_string()),
                disabled_at: None,
                disabled_by_key_id: None,
            }],
            is_admin: false,
            email_verified: true,
            email: Some("alice@example.com".to_string()),
        }
    }

    #[test]
    fn test_api_response_account_with_keys_success_serialization() {
        let resp = ApiResponse {
            success: true,
            data: Some(sample_account_with_keys()),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["username"], "alice");
        assert_eq!(json["data"]["emailVerified"], true);
        let keys = json["data"]["publicKeys"].as_array().unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0]["isActive"], true);
        assert_eq!(keys[0]["deviceName"], "Laptop");
    }

    #[test]
    fn test_api_response_account_with_keys_error() {
        let resp: ApiResponse<AccountWithKeys> = ApiResponse {
            success: false,
            data: None,
            error: Some("Account not found".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], false);
        assert_eq!(json["error"], "Account not found");
        assert!(json.get("data").is_none());
    }

    // ---- ApiResponse<()> for void endpoints ----

    #[test]
    fn test_api_response_string_success() {
        let resp = ApiResponse {
            success: true,
            data: Some("Contact added successfully".to_string()),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"], "Contact added successfully");
    }

    #[test]
    fn test_api_response_string_error() {
        let resp: ApiResponse<String> = ApiResponse {
            success: false,
            data: None,
            error: Some("Unauthorized: Cannot modify another user's contacts".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], false);
        assert!(json["error"].as_str().unwrap().contains("Unauthorized"));
    }

    // ---- BillingSettings ----

    #[test]
    fn test_billing_settings_serialization_camel_case() {
        let settings = BillingSettings {
            billing_address: Some("123 Main St".to_string()),
            billing_vat_id: Some("VAT123".to_string()),
            billing_country_code: Some("US".to_string()),
        };
        let json = serde_json::to_value(&settings).unwrap();
        assert_eq!(json["billingAddress"], "123 Main St");
        assert_eq!(json["billingVatId"], "VAT123");
        assert_eq!(json["billingCountryCode"], "US");
    }

    #[test]
    fn test_billing_settings_all_none() {
        let settings = BillingSettings {
            billing_address: None,
            billing_vat_id: None,
            billing_country_code: None,
        };
        let json = serde_json::to_value(&settings).unwrap();
        assert!(json.get("billingAddress").is_none());
        assert!(json.get("billingVatId").is_none());
        assert!(json.get("billingCountryCode").is_none());
    }

    // ---- Validation functions (called inline in handler, tested here) ----

    #[test]
    fn test_validate_account_username_valid() {
        let result = crate::validation::validate_account_username("alice99");
        assert!(result.is_ok(), "Valid username should pass: {:?}", result);
        assert_eq!(result.unwrap(), "alice99");
    }

    #[test]
    fn test_validate_account_username_too_short() {
        let result = crate::validation::validate_account_username("ab");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("at least 3 characters"));
    }

    #[test]
    fn test_validate_account_username_reserved() {
        let result = crate::validation::validate_account_username("admin");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("reserved"));
    }

    #[test]
    fn test_validate_email_valid() {
        let result = crate::validation::validate_email("user@example.com");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_email_invalid() {
        let result = crate::validation::validate_email("not-an-email");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_contact_type_valid() {
        for t in &["phone", "telegram", "discord", "signal"] {
            assert!(
                crate::validation::validate_contact_type(t).is_ok(),
                "Expected {} to be valid",
                t
            );
        }
    }

    #[test]
    fn test_validate_contact_type_email_is_invalid() {
        // email is explicitly NOT a valid contact type
        let result = crate::validation::validate_contact_type("email");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_social_platform_valid() {
        for p in &["twitter", "github", "discord", "linkedin", "reddit"] {
            assert!(
                crate::validation::validate_social_platform(p).is_ok(),
                "Expected {} to be valid",
                p
            );
        }
    }

    #[test]
    fn test_validate_social_platform_invalid() {
        let result = crate::validation::validate_social_platform("facebook");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_url_valid() {
        let result = crate::validation::validate_url("https://example.com/profile");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_url_missing_scheme() {
        let result = crate::validation::validate_url("example.com/profile");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_public_key_ssh_valid() {
        let result = crate::validation::validate_public_key(
            "ssh-ed25519",
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAA my-key",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_public_key_ssh_missing_prefix() {
        let result = crate::validation::validate_public_key("ssh-ed25519", "not-starting-with-ssh");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_public_key_gpg_valid() {
        let result = crate::validation::validate_public_key(
            "gpg",
            "-----BEGIN PGP PUBLIC KEY BLOCK-----\ndata\n-----END PGP PUBLIC KEY BLOCK-----",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_public_key_gpg_missing_header() {
        let result = crate::validation::validate_public_key("gpg", "just some random data");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("PGP public key block"));
    }
}
