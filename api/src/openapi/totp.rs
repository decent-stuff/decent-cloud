//! TOTP 2FA endpoints (ticket #80).
//!
//! Extracted from `accounts.rs` as part of #444 large-file splits. These five
//! handlers are fully decoupled from `AccountsApi`: they depend only on the
//! `Database` TOTP methods (`api/src/database/totp.rs`) and the shared TOTP
//! DTOs in `openapi::common`. Behavior is identical — every path/method/tag/
//! schema is unchanged (verified via byte-identical `/api/v1/openapi` spec).

use super::common::{
    ApiResponse, ApiTags, TotpCodeRequest, TotpEnableRequest, TotpEnableResponse,
    TotpSetupResponse, TotpStatusResponse,
};
use crate::{auth::ApiAuthenticatedUser, database::Database};
use poem::web::Data;
use poem_openapi::{payload::Json, OpenApi};
use std::sync::Arc;

pub struct TotpApi;

#[OpenApi]
impl TotpApi {
    // ── TOTP 2FA endpoints (ticket #80) ──────────────────────────────────

    /// Get TOTP status
    ///
    /// Returns whether TOTP two-factor authentication is enabled for the
    /// authenticated account.
    #[oai(path = "/accounts/me/totp", method = "get", tag = "ApiTags::Accounts")]
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
    use crate::openapi::common::{
        TotpCodeRequest, TotpEnableRequest, TotpEnableResponse, TotpSetupResponse,
        TotpStatusResponse,
    };

    #[test]
    fn test_totp_setup_response_camel_case() {
        let resp = TotpSetupResponse {
            secret: "JBSWY3DPEHPK3PXP".to_string(),
            otpauth_uri: "otpauth://totp/DecentCloud:alice?secret=JBSWY3DPEHPK3PXP".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["secret"], "JBSWY3DPEHPK3PXP");
        assert_eq!(
            json["otpauthUri"],
            "otpauth://totp/DecentCloud:alice?secret=JBSWY3DPEHPK3PXP"
        );
    }

    #[test]
    fn test_totp_enable_request_deserialization() {
        let json = r#"{"code":"123456"}"#;
        let req: TotpEnableRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "123456");
    }

    #[test]
    fn test_totp_enable_response_camel_case() {
        let resp = TotpEnableResponse {
            backup_codes: vec!["abc123".to_string(), "def456".to_string()],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["backupCodes"].as_array().unwrap().len(), 2);
        assert_eq!(json["backupCodes"][0], "abc123");
    }

    #[test]
    fn test_totp_code_request_deserialization() {
        let json = r#"{"code":"654321"}"#;
        let req: TotpCodeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "654321");
    }

    #[test]
    fn test_totp_status_response_camel_case() {
        let resp = TotpStatusResponse {
            enabled: true,
            has_backup_codes: false,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["enabled"], true);
        assert_eq!(json["hasBackupCodes"], false);
    }
}
