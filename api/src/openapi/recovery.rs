//! Account recovery endpoints.
//!
//! Extracted from `accounts.rs` as part of #444 large-file splits. These two
//! handlers are fully decoupled from `AccountsApi`: they depend only on the
//! `Database` recovery methods (`api/src/database/recovery.rs`), the shared
//! email queue, and the shared recovery DTOs in `openapi::common`. Behavior is
//! identical — every path/method/tag/schema is unchanged (verified via
//! byte-identical `/api/v1/openapi` spec).

use super::common::{
    decode_hex_path, decode_pubkey, ApiResponse, ApiTags, CompleteRecoveryRequest,
    RequestRecoveryRequest,
};
use crate::{database::email::EmailType, database::Database};
use poem::web::Data;
use poem_openapi::{payload::Json, OpenApi};
use std::sync::Arc;

pub struct RecoveryApi;

#[OpenApi]
impl RecoveryApi {
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
        let token = match decode_hex_path(&req.token, "recovery token") {
            Ok(t) => t,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
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
}

#[cfg(test)]
mod tests {
    use crate::openapi::common::{CompleteRecoveryRequest, RequestRecoveryRequest};

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
}
