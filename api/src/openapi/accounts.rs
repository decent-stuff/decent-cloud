use super::common::{
    AddAccountContactRequest, AddAccountExternalKeyRequest, AddAccountKeyRequest,
    AddAccountSocialRequest, ApiResponse, ApiTags, CompleteRecoveryRequest, RegisterAccountRequest,
    RequestRecoveryRequest, UpdateAccountEmailRequest, UpdateAccountProfileRequest,
    UpdateDeviceNameRequest, VerifyEmailRequest,
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
                    tracing::warn!("Failed to insert audit record: {}", e);
                }

                // Create email verification token
                match db
                    .create_email_verification_token(&account.id, &body_data.email)
                    .await
                {
                    Ok(token) => {
                        // Build verification URL
                        let base_url = std::env::var("FRONTEND_URL")
                            .unwrap_or_else(|_| "http://localhost:59000".to_string());
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
                        tracing::warn!("Failed to create verification token: {}", e);
                    }
                }

                // Create Chatwoot agent for support (non-blocking)
                if crate::chatwoot::integration::is_configured() {
                    match crate::chatwoot::integration::create_provider_agent(&db, &public_key)
                        .await
                    {
                        Ok(()) => {
                            // Queue support portal welcome email
                            let support_url = std::env::var("CHATWOOT_FRONTEND_URL")
                                .unwrap_or_else(|_| "https://support.decent-cloud.org".to_string());

                            db.queue_email_safe(
                                Some(&body_data.email),
                                "noreply@decent-cloud.org",
                                "Your Support Portal is Ready",
                                &format!(
                                    r#"Hello {},

Your Decent Cloud support portal account has been created. You will receive a separate email from the support system to set your password.

SUPPORT PORTAL ACCESS
---------------------
Web: {}

MOBILE APP SETUP
----------------
Stay connected with your customers on the go by installing the Chatwoot mobile app:

iOS (iPhone/iPad):
1. Open the App Store
2. Search for "Chatwoot" or visit: https://apps.apple.com/app/chatwoot/id1495796682
3. Tap "Get" to install

Android:
1. Open the Google Play Store
2. Search for "Chatwoot" or visit: https://play.google.com/store/apps/details?id=com.chatwoot.app
3. Tap "Install"

CONNECTING THE APP
------------------
1. Open the Chatwoot app after installation
2. Enter the server URL: {}
3. Tap "Connect"
4. Sign in with your email and the password you set

With the mobile app, you can respond to customer inquiries in real-time, receive push notifications, and manage conversations from anywhere.

Best regards,
The Decent Cloud Team"#,
                                    username, support_url, support_url
                                ),
                                false,
                                EmailType::General,
                            )
                            .await;
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to create Chatwoot agent for {}: {}",
                                username,
                                e
                            );
                        }
                    }
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
        if email.is_empty() {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Email address is required".to_string()),
            });
        }
        let email_pattern = regex::Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap();
        if !email_pattern.is_match(email) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Invalid email address format".to_string()),
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
                tracing::error!("Failed to create verification token: {}", e);
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

    /// Get account contacts
    ///
    /// Returns contact information for an account (public, no authentication required)
    #[oai(
        path = "/accounts/:username/contacts",
        method = "get",
        tag = "ApiTags::Accounts"
    )]
    async fn get_account_contacts(
        &self,
        db: Data<&Arc<Database>>,
        username: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::users::AccountContact>>> {
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

        // Get contacts (public - anyone can view)
        match db.get_account_contacts(&account.id).await {
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

    /// Add account contact
    ///
    /// Adds a new contact to an account (requires authentication)
    #[oai(
        path = "/accounts/:username/contacts",
        method = "post",
        tag = "ApiTags::Accounts"
    )]
    async fn add_account_contact(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        username: Path<String>,
        req: Json<AddAccountContactRequest>,
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
                    error: Some("Unauthorized: Cannot modify another user's contacts".to_string()),
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

        // Validate contact type and value
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

        // Add contact
        match db
            .add_account_contact(
                &account.id,
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

    /// Delete account contact
    ///
    /// Deletes a contact from an account (requires authentication)
    #[oai(
        path = "/accounts/:username/contacts/:contact_id",
        method = "delete",
        tag = "ApiTags::Accounts"
    )]
    async fn delete_account_contact(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        username: Path<String>,
        contact_id: Path<i64>,
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
                    error: Some("Unauthorized: Cannot modify another user's contacts".to_string()),
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

        // Delete contact
        match db.delete_account_contact(&account.id, contact_id.0).await {
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

    /// Get account socials
    ///
    /// Returns social media accounts for an account (public endpoint)
    #[oai(
        path = "/accounts/:username/socials",
        method = "get",
        tag = "ApiTags::Accounts"
    )]
    async fn get_account_socials(
        &self,
        db: Data<&Arc<Database>>,
        username: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::users::AccountSocial>>> {
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

        // Get socials (public, no auth required)
        match db.get_account_socials(&account.id).await {
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

    /// Add account social
    ///
    /// Adds a social media account to an account (requires authentication)
    #[oai(
        path = "/accounts/:username/socials",
        method = "post",
        tag = "ApiTags::Accounts"
    )]
    async fn add_account_social(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        username: Path<String>,
        req: Json<AddAccountSocialRequest>,
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
                    error: Some("Unauthorized: Cannot modify another user's socials".to_string()),
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

        // Validate social platform and username
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

        // Add social
        match db
            .add_account_social(
                &account.id,
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

    /// Delete account social
    ///
    /// Deletes a social media account from an account (requires authentication)
    #[oai(
        path = "/accounts/:username/socials/:social_id",
        method = "delete",
        tag = "ApiTags::Accounts"
    )]
    async fn delete_account_social(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        username: Path<String>,
        social_id: Path<i64>,
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
                    error: Some("Unauthorized: Cannot modify another user's socials".to_string()),
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

        // Delete social
        match db.delete_account_social(&account.id, social_id.0).await {
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

    /// Request account recovery
    ///
    /// Sends a recovery link to the email address associated with an account.
    /// The recovery link expires after 24 hours.
    #[oai(
        path = "/accounts/recovery/request",
        method = "post",
        tag = "ApiTags::Accounts"
    )]
    async fn request_account_recovery(
        &self,
        db: Data<&Arc<Database>>,
        req: Json<RequestRecoveryRequest>,
    ) -> Json<ApiResponse<String>> {
        // Validate email
        if let Err(e) = crate::validation::validate_email(&req.email) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            });
        }

        // Create recovery token
        let token = match db.create_recovery_token(&req.email).await {
            Ok(t) => t,
            Err(e) => {
                // Don't reveal whether email exists for security
                tracing::warn!("Recovery token creation failed for {}: {}", req.email, e);
                return Json(ApiResponse {
                    success: true,
                    data: Some(
                        "If an account exists with this email, a recovery link has been sent."
                            .to_string(),
                    ),
                    error: None,
                });
            }
        };

        // Build recovery URL
        let base_url =
            std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:59000".to_string());
        let token_hex = hex::encode(&token);
        let recovery_url = format!("{}/recover?token={}", base_url, token_hex);

        // Queue recovery email
        let subject = "Decent Cloud Account Recovery";
        let body = format!(
            "Hello,\n\n\
            You requested account recovery for your Decent Cloud account.\n\n\
            Click the link below to recover your account:\n\
            {}\n\n\
            This link will expire in 24 hours.\n\n\
            If you did not request this recovery, please ignore this email.\n\n\
            Best regards,\n\
            The Decent Cloud Team",
            recovery_url
        );

        db.queue_email_safe(
            Some(&req.email),
            "noreply@decent-cloud.org",
            subject,
            &body,
            false,
            EmailType::Recovery, // Critical: account recovery with 24 attempts
        )
        .await;

        Json(ApiResponse {
            success: true,
            data: Some(
                "If an account exists with this email, a recovery link has been sent.".to_string(),
            ),
            error: None,
        })
    }

    /// Complete account recovery
    ///
    /// Completes the account recovery process by verifying the token and adding a new public key.
    /// This allows users to regain access to their account with a new key.
    #[oai(
        path = "/accounts/recovery/complete",
        method = "post",
        tag = "ApiTags::Accounts"
    )]
    async fn complete_account_recovery(
        &self,
        db: Data<&Arc<Database>>,
        req: Json<CompleteRecoveryRequest>,
    ) -> Json<ApiResponse<String>> {
        // Decode token
        let token = match hex::decode(&req.token) {
            Ok(t) => t,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid token format: {}", e)),
                })
            }
        };

        // Decode public key
        let public_key = match hex::decode(&req.public_key) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid public key format: {}", e)),
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

        // Complete recovery
        match db.complete_recovery(&token, &public_key).await {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Account recovery completed successfully. You can now sign in with your new key.".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Verify email address
    ///
    /// Verifies an email address using a token sent via email.
    /// This is a public endpoint (no authentication required).
    #[oai(
        path = "/accounts/verify-email",
        method = "post",
        tag = "ApiTags::Accounts"
    )]
    async fn verify_email(
        &self,
        db: Data<&Arc<Database>>,
        req: Json<VerifyEmailRequest>,
    ) -> Json<ApiResponse<String>> {
        // Decode token
        let token = match hex::decode(&req.token) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(
                    "Email verification failed: invalid hex format (len={}): {}",
                    req.token.len(),
                    e
                );
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid token format: {}", e)),
                });
            }
        };

        tracing::info!(
            "Email verification attempt: token_len={} bytes",
            token.len()
        );

        // Verify email
        match db.verify_email_token(&token).await {
            Ok(_) => Json(ApiResponse {
                success: true,
                data: Some("Email verified successfully.".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Resend verification email
    ///
    /// Sends a new verification email to the authenticated user.
    /// Rate limited to once per minute.
    #[oai(
        path = "/accounts/resend-verification",
        method = "post",
        tag = "ApiTags::Accounts"
    )]
    async fn resend_verification_email(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<String>> {
        // Get account by authenticated user's public key
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
                        error: Some(e.to_string()),
                    })
                }
            },
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

        // Check if already verified
        if account.email_verified != 0 {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Email already verified".to_string()),
            });
        }

        // Check if email is set
        let email = match account.email {
            Some(ref e) => e,
            None => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("No email address on account".to_string()),
                })
            }
        };

        // Check rate limit
        let now = chrono::Utc::now().timestamp();
        match db.get_latest_verification_token_time(&account.id).await {
            Ok(Some(last_time)) => {
                let elapsed = now - last_time;
                if elapsed < 60 {
                    let remaining = 60 - elapsed;
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(format!(
                            "Please wait {} seconds before requesting another email",
                            remaining
                        )),
                    });
                }
            }
            Ok(None) => {}
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        }

        // Create new verification token
        let token = match db.create_email_verification_token(&account.id, email).await {
            Ok(t) => t,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to create verification token: {}", e)),
                })
            }
        };

        // Build verification URL
        let base_url =
            std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:59000".to_string());
        let token_hex = hex::encode(&token);
        let verification_url = format!("{}/verify-email?token={}", base_url, token_hex);

        // Queue verification email
        let subject = "Verify Your Decent Cloud Email";
        let body = format!(
            "Hello {},\n\n\
            Thank you for registering with Decent Cloud!\n\n\
            Please verify your email address by clicking the link below:\n\
            {}\n\n\
            This link will expire in 24 hours.\n\n\
            If you did not request this verification email, please ignore this message.\n\n\
            Best regards,\n\
            The Decent Cloud Team",
            account.username, verification_url
        );

        db.queue_email_safe(
            Some(email),
            "noreply@decent-cloud.org",
            subject,
            &body,
            false,
            EmailType::Welcome,
        )
        .await;

        Json(ApiResponse {
            success: true,
            data: Some("Verification email sent successfully".to_string()),
            error: None,
        })
    }
}
