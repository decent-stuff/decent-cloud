use super::common::ApiResponse;
use crate::auth::ApiAuthenticatedUser;
use crate::chatwoot::{generate_identity_hash, ChatwootPlatformClient};
use crate::database::email::EmailType;
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

    /// Reset support portal access
    ///
    /// Generates a new password for the authenticated user's Chatwoot support portal account.
    /// The new password is sent to the user's email address.
    #[oai(
        path = "/chatwoot/support-access/reset",
        method = "post",
        tag = "super::common::ApiTags::Chatwoot"
    )]
    async fn reset_support_access(
        &self,
        db: Data<&Arc<Database>>,
        user: ApiAuthenticatedUser,
    ) -> Json<ApiResponse<String>> {
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
                tracing::error!("Failed to get chatwoot_user_id: {}", e);
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Database error".to_string()),
                });
            }
        };

        // Get account email
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
                tracing::error!("Failed to get account: {}", e);
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Database error".to_string()),
                });
            }
        };

        let email = match &account.email {
            Some(e) => e.clone(),
            None => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("No email address on account".to_string()),
                })
            }
        };

        // Create Platform client and generate new password
        let client = match ChatwootPlatformClient::from_env() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to create Chatwoot client: {}", e);
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
            tracing::error!("Failed to update Chatwoot password: {}", e);
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Failed to reset password. Please try again later.".to_string()),
            });
        }

        // Queue email with new password
        let support_url = std::env::var("CHATWOOT_FRONTEND_URL")
            .unwrap_or_else(|_| "https://support.decent-cloud.org".to_string());

        db.queue_email_safe(
            Some(&email),
            "noreply@decent-cloud.org",
            "Your Support Portal Password Has Been Reset",
            &format!(
                r#"Hello {},

Your Decent Cloud support portal password has been reset.

NEW CREDENTIALS
---------------
Email: {}
Password: {}
Login: {}/app/login

IMPORTANT: For security, please change this password after logging in.
Go to Profile → Password to set a new password of your choice.

If you did not request this reset, please contact us immediately.

Best regards,
The Decent Cloud Team"#,
                account.username, email, new_password, support_url
            ),
            false,
            EmailType::General,
        )
        .await;

        tracing::info!(
            "Support portal password reset for user {} (chatwoot_user_id: {})",
            account.username,
            chatwoot_user_id
        );

        Json(ApiResponse {
            success: true,
            data: Some("New password sent to your email address.".to_string()),
            error: None,
        })
    }
}
