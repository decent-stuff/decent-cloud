use super::common::ApiResponse;
use crate::auth::ApiAuthenticatedUser;
use crate::chatwoot::{generate_identity_hash, ChatwootPlatformClient};
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{payload::Json, Object, OpenApi};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ChatwootIdentityResponse {
    /// User identifier for Chatwoot (hex-encoded pubkey)
    pub identifier: String,
    /// HMAC hash for identity validation
    pub identifier_hash: String,
}

#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct SupportPortalStatus {
    /// Whether the user has a support portal account
    pub has_account: bool,
    /// Chatwoot user ID (if account exists)
    #[oai(skip_serializing_if_is_none)]
    pub user_id: Option<i64>,
    /// Email address used for support portal
    #[oai(skip_serializing_if_is_none)]
    pub email: Option<String>,
    /// Login URL for the support portal
    pub login_url: String,
    /// Help Center portal slug for this provider (if set up)
    #[oai(skip_serializing_if_is_none)]
    pub portal_slug: Option<String>,
    /// Provider's inbox ID for filtering conversations
    #[oai(skip_serializing_if_is_none)]
    pub inbox_id: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct PasswordResetResponse {
    /// The new password (display once, do not store)
    pub password: String,
    /// Login URL for the support portal
    pub login_url: String,
}

pub struct ChatwootApi;

#[OpenApi]
impl ChatwootApi {
    /// Get Chatwoot identity hash
    ///
    /// Returns the identifier and HMAC hash for Chatwoot widget authentication.
    /// Used by the frontend to authenticate users in the Chatwoot widget.
    #[oai(
        path = "/chatwoot/identity",
        method = "get",
        tag = "super::common::ApiTags::Chatwoot"
    )]
    async fn get_identity(
        &self,
        user: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<ChatwootIdentityResponse>> {
        let hmac_secret = match std::env::var("CHATWOOT_HMAC_SECRET") {
            Ok(secret) => secret,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Chatwoot not configured".to_string()),
                })
            }
        };

        let identifier = hex::encode(&user.pubkey);
        let identifier_hash = generate_identity_hash(&identifier, &hmac_secret);

        Json(ApiResponse {
            success: true,
            data: Some(ChatwootIdentityResponse {
                identifier,
                identifier_hash,
            }),
            error: None,
        })
    }

    /// Get support portal status
    ///
    /// Returns the user's support portal account status including user ID and login URL.
    #[oai(
        path = "/chatwoot/support-access",
        method = "get",
        tag = "super::common::ApiTags::Chatwoot"
    )]
    async fn get_support_access_status(
        &self,
        db: Data<&Arc<Database>>,
        user: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<SupportPortalStatus>> {
        let support_url =
            std::env::var("CHATWOOT_FRONTEND_URL").expect("CHATWOOT_FRONTEND_URL must be set");

        // Get chatwoot_user_id for this account
        let chatwoot_user_id = match db.get_chatwoot_user_id_by_public_key(&user.pubkey).await {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("Failed to get chatwoot_user_id: {:#}", e);
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Database error".to_string()),
                });
            }
        };

        // Get account email
        let email = match db.get_account_with_keys_by_public_key(&user.pubkey).await {
            Ok(Some(acc)) => acc.email,
            Ok(None) => None,
            Err(e) => {
                tracing::error!("Failed to get account: {:#}", e);
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Database error".to_string()),
                });
            }
        };

        // Get provider's Chatwoot resources (inbox_id, portal_slug)
        let (inbox_id, portal_slug) = match db.get_provider_chatwoot_resources(&user.pubkey).await {
            Ok(Some((inbox, _team, slug))) => (Some(inbox), Some(slug)),
            Ok(None) => (None, None),
            Err(e) => {
                tracing::warn!("Failed to get provider Chatwoot resources: {:#}", e);
                (None, None)
            }
        };

        Json(ApiResponse {
            success: true,
            data: Some(SupportPortalStatus {
                has_account: chatwoot_user_id.is_some(),
                user_id: chatwoot_user_id,
                email,
                login_url: format!("{}/app/login", support_url),
                portal_slug,
                inbox_id,
            }),
            error: None,
        })
    }

    /// Create support portal account
    ///
    /// Creates a new Chatwoot support portal account for users who don't have one yet.
    /// Returns the initial password directly - display it once and do not store it.
    #[oai(
        path = "/chatwoot/support-access",
        method = "post",
        tag = "super::common::ApiTags::Chatwoot"
    )]
    async fn create_support_access(
        &self,
        db: Data<&Arc<Database>>,
        user: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<PasswordResetResponse>> {
        // Check if Platform API is configured
        if !ChatwootPlatformClient::is_configured() {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Chatwoot Platform API not configured".to_string()),
            });
        }

        // Check if user already has an account
        match db.get_chatwoot_user_id_by_public_key(&user.pubkey).await {
            Ok(Some(_)) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Support portal account already exists".to_string()),
                })
            }
            Ok(None) => {} // No account, proceed with creation
            Err(e) => {
                tracing::error!("Failed to check chatwoot_user_id: {:#}", e);
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Database error".to_string()),
                });
            }
        }

        // Create the account
        let password =
            match crate::chatwoot::integration::create_provider_agent(&db, &user.pubkey).await {
                Ok(pwd) => pwd,
                Err(e) => {
                    tracing::error!("Failed to create support portal account: {:#}", e);
                    return Json(ApiResponse {
                        success: false,
                        data: None,
                        error: Some(format!("{:#}", e)),
                    });
                }
            };

        let support_url =
            std::env::var("CHATWOOT_FRONTEND_URL").expect("CHATWOOT_FRONTEND_URL must be set");

        Json(ApiResponse {
            success: true,
            data: Some(PasswordResetResponse {
                password,
                login_url: format!("{}/app/login", support_url),
            }),
            error: None,
        })
    }

    /// Reset support portal password
    ///
    /// Generates a new password for the authenticated user's Chatwoot support portal account.
    /// Returns the new password directly - display it once and do not store it.
    #[oai(
        path = "/chatwoot/support-access/reset",
        method = "post",
        tag = "super::common::ApiTags::Chatwoot"
    )]
    async fn reset_support_access(
        &self,
        db: Data<&Arc<Database>>,
        user: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<PasswordResetResponse>> {
        // Check if Platform API is configured
        if !ChatwootPlatformClient::is_configured() {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Chatwoot Platform API not configured".to_string()),
            });
        }

        // Get chatwoot_user_id for this account
        let chatwoot_user_id = match db.get_chatwoot_user_id_by_public_key(&user.pubkey).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(
                        "No support portal account found. Please contact support.".to_string(),
                    ),
                })
            }
            Err(e) => {
                tracing::error!("Failed to get chatwoot_user_id: {:#}", e);
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Database error".to_string()),
                });
            }
        };

        // Get account for logging
        let account = match db.get_account_with_keys_by_public_key(&user.pubkey).await {
            Ok(Some(acc)) => acc,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Account not found".to_string()),
                })
            }
            Err(e) => {
                tracing::error!("Failed to get account: {:#}", e);
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Database error".to_string()),
                });
            }
        };

        // Create Platform client and generate new password
        let client = match ChatwootPlatformClient::from_env() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to create Chatwoot client: {:#}", e);
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Chatwoot configuration error".to_string()),
                });
            }
        };

        let new_password = crate::chatwoot::integration::generate_secure_password();

        // Update password via Platform API
        if let Err(e) = client
            .update_user_password(chatwoot_user_id, &new_password)
            .await
        {
            tracing::error!("Failed to update Chatwoot password: {:#}", e);
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("{:#}", e)),
            });
        }

        let support_url =
            std::env::var("CHATWOOT_FRONTEND_URL").expect("CHATWOOT_FRONTEND_URL must be set");

        tracing::info!(
            "Support portal password reset for user {} (chatwoot_user_id: {})",
            account.username,
            chatwoot_user_id
        );

        Json(ApiResponse {
            success: true,
            data: Some(PasswordResetResponse {
                password: new_password,
                login_url: format!("{}/app/login", support_url),
            }),
            error: None,
        })
    }
}
