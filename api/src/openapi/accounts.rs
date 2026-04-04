use super::common::{
    AddAccountContactRequest, AddAccountExternalKeyRequest, AddAccountKeyRequest,
    AddAccountSocialRequest, ApiResponse, ApiTags, CompleteRecoveryRequest, RegisterAccountRequest,
    RequestRecoveryRequest, TotpCodeRequest, TotpEnableRequest, TotpEnableResponse,
    TotpSetupResponse, TotpStatusResponse, UpdateAccountEmailRequest, UpdateAccountProfileRequest,
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
                tracing::warn!("Recovery token creation failed for {}: {:#}", req.email, e);
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
            std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:59010".to_string());
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
        if account.email_verified {
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
        let now = match crate::now_ns() {
            Ok(ns) => ns,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };
        match db.get_latest_verification_token_time(&account.id).await {
            Ok(Some(last_time)) => {
                let elapsed_secs = (now - last_time) / 1_000_000_000;
                if elapsed_secs < 60 {
                    let remaining = 60 - elapsed_secs;
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
            std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:59010".to_string());
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

    // ── TOTP 2FA endpoints (ticket #80) ──────────────────────────────────

    /// Get TOTP status
    ///
    /// Returns whether TOTP two-factor authentication is enabled for the
    /// authenticated account.
    #[oai(
        path = "/accounts/me/totp",
        method = "get",
        tag = "ApiTags::Accounts"
    )]
    async fn get_totp_status(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<TotpStatusResponse>> {
        let account_id = match db.get_account_id_by_public_key(&auth.pubkey).await {
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
                    error: Some(format!("Failed to look up account: {:#?}", e)),
                })
            }
        };

        match db.totp_status(&account_id).await {
            Ok(status) => Json(ApiResponse {
                success: true,
                data: Some(TotpStatusResponse {
                    enabled: status.enabled,
                    has_backup_codes: status.has_backup_codes,
                }),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to get TOTP status: {:#?}", e)),
            }),
        }
    }

    /// Begin TOTP enrollment
    ///
    /// Generates a TOTP secret and returns it as a base32 string and an
    /// `otpauth://` URI suitable for rendering as a QR code.  The secret is
    /// stored (unconfirmed) until `POST /accounts/me/totp/enable` is called.
    #[oai(
        path = "/accounts/me/totp/setup",
        method = "post",
        tag = "ApiTags::Accounts"
    )]
    async fn setup_totp(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<TotpSetupResponse>> {
        let account_id = match db.get_account_id_by_public_key(&auth.pubkey).await {
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
                    error: Some(format!("Failed to look up account: {:#?}", e)),
                })
            }
        };
        let username = match db.get_account(&account_id).await {
            Ok(Some(acc)) => acc.username,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Account record not found".to_string()),
                })
            }
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to load account: {:#?}", e)),
                })
            }
        };

        match db.setup_totp(&account_id, &username).await {
            Ok((secret, uri)) => Json(ApiResponse {
                success: true,
                data: Some(TotpSetupResponse {
                    secret,
                    otpauth_uri: uri,
                }),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to set up TOTP: {:#?}", e)),
            }),
        }
    }

    /// Confirm TOTP enrollment
    ///
    /// Verifies the first TOTP code entered by the user.  On success, enables
    /// TOTP for the account and returns one-time backup codes.  Store backup
    /// codes securely — they are shown once and not recoverable.
    #[oai(
        path = "/accounts/me/totp/enable",
        method = "post",
        tag = "ApiTags::Accounts"
    )]
    async fn enable_totp(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        body: Json<TotpEnableRequest>,
    ) -> Json<ApiResponse<TotpEnableResponse>> {
        let account_id = match db.get_account_id_by_public_key(&auth.pubkey).await {
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
                    error: Some(format!("Failed to look up account: {:#?}", e)),
                })
            }
        };

        match db.enable_totp(&account_id, &body.0.code).await {
            Ok(backup_codes) => Json(ApiResponse {
                success: true,
                data: Some(TotpEnableResponse { backup_codes }),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to enable TOTP: {:#?}", e)),
            }),
        }
    }

    /// Disable TOTP
    ///
    /// Disables TOTP for the account.  Requires a valid TOTP code (or backup
    /// code) to confirm the action.
    #[oai(
        path = "/accounts/me/totp",
        method = "delete",
        tag = "ApiTags::Accounts"
    )]
    async fn disable_totp(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        body: Json<TotpCodeRequest>,
    ) -> Json<ApiResponse<String>> {
        let account_id = match db.get_account_id_by_public_key(&auth.pubkey).await {
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
                    error: Some(format!("Failed to look up account: {:#?}", e)),
                })
            }
        };

        match db.disable_totp(&account_id, &body.0.code).await {
            Ok(()) => Json(ApiResponse {
                success: true,
                data: Some("TOTP disabled".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to disable TOTP: {:#?}", e)),
            }),
        }
    }

    /// Regenerate backup codes
    ///
    /// Invalidates all existing backup codes and generates new ones.
    /// Requires a valid TOTP code to authorise.
    #[oai(
        path = "/accounts/me/totp/backup-codes",
        method = "post",
        tag = "ApiTags::Accounts"
    )]
    async fn regenerate_backup_codes(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        body: Json<TotpCodeRequest>,
    ) -> Json<ApiResponse<TotpEnableResponse>> {
        let account_id = match db.get_account_id_by_public_key(&auth.pubkey).await {
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
                    error: Some(format!("Failed to look up account: {:#?}", e)),
                })
            }
        };

        match db.regenerate_backup_codes(&account_id, &body.0.code).await {
            Ok(backup_codes) => Json(ApiResponse {
                success: true,
                data: Some(TotpEnableResponse { backup_codes }),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to regenerate backup codes: {:#?}", e)),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::database::accounts::{AccountWithKeys, BillingSettings, PublicKeyInfo};
    use crate::openapi::common::{
        AddAccountContactRequest, AddAccountExternalKeyRequest, AddAccountKeyRequest,
        AddAccountSocialRequest, ApiResponse, CompleteRecoveryRequest, RegisterAccountRequest,
        RequestRecoveryRequest, UpdateAccountEmailRequest, UpdateAccountProfileRequest,
        UpdateDeviceNameRequest, VerifyEmailRequest,
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

    // ---- AddAccountSocialRequest ----

    #[test]
    fn test_add_account_social_request_with_url() {
        let json =
            r#"{"platform":"github","username":"alice","profileUrl":"https://github.com/alice"}"#;
        let req: AddAccountSocialRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.platform, "github");
        assert_eq!(req.username, "alice");
        assert_eq!(req.profile_url.as_deref(), Some("https://github.com/alice"));
    }

    #[test]
    fn test_add_account_social_request_no_url() {
        let json = r#"{"platform":"twitter","username":"alice_tw"}"#;
        let req: AddAccountSocialRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.platform, "twitter");
        assert!(req.profile_url.is_none());
    }

    // ---- AddAccountContactRequest ----

    #[test]
    fn test_add_account_contact_request_verified_default_false() {
        // `verified` has #[oai(default = "default_false")] but serde default is not set
        // so it must be provided explicitly in JSON
        let json = r#"{"contactType":"telegram","contactValue":"@alice","verified":false}"#;
        let req: AddAccountContactRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.contact_type, "telegram");
        assert_eq!(req.contact_value, "@alice");
        assert!(!req.verified);
    }

    #[test]
    fn test_add_account_contact_request_verified_true() {
        let json = r#"{"contactType":"phone","contactValue":"+1234567890","verified":true}"#;
        let req: AddAccountContactRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.contact_type, "phone");
        assert!(req.verified);
    }

    // ---- VerifyEmailRequest ----

    #[test]
    fn test_verify_email_request_deserialization() {
        let json = r#"{"token":"deadbeefcafe"}"#;
        let req: VerifyEmailRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.token, "deadbeefcafe");
    }

    // ---- RequestRecoveryRequest ----

    #[test]
    fn test_request_recovery_request_deserialization() {
        let json = r#"{"email":"user@example.com"}"#;
        let req: RequestRecoveryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.email, "user@example.com");
    }

    // ---- CompleteRecoveryRequest ----

    #[test]
    fn test_complete_recovery_request_deserialization() {
        let json = r#"{"token":"aabbccdd","publicKey":"eeff0011"}"#;
        let req: CompleteRecoveryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.token, "aabbccdd");
        assert_eq!(req.public_key, "eeff0011");
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
