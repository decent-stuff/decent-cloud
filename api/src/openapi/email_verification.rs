//! Email-verification endpoints.
//!
//! Extracted from `accounts.rs` as part of #444 large-file splits. These two
//! handlers are fully decoupled from `AccountsApi`: they depend only on the
//! `Database` email-verification methods, the shared email queue, and the
//! shared `VerifyEmailRequest` DTO in `openapi::common`. Behavior is identical
//! — every path/method/tag/schema is unchanged (verified via byte-identical
//! `/api/v1/openapi` spec).

use super::common::{decode_hex_path, ApiResponse, ApiTags, VerifyEmailRequest};
use crate::{auth::ApiAuthenticatedUser, database::email::EmailType, database::Database};
use poem::web::Data;
use poem_openapi::{payload::Json, OpenApi};
use std::sync::Arc;

pub struct EmailVerificationApi;

#[OpenApi]
impl EmailVerificationApi {
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
        let token = match decode_hex_path(&req.token, "email verification token") {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("Email verification failed: {e}");
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
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
}

#[cfg(test)]
mod tests {
    use crate::openapi::common::VerifyEmailRequest;

    // ---- VerifyEmailRequest ----

    #[test]
    fn test_verify_email_request_deserialization() {
        let json = r#"{"token":"deadbeefcafe"}"#;
        let req: VerifyEmailRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.token, "deadbeefcafe");
    }
}
