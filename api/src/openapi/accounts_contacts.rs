//! Account contact and social-media endpoints.
//!
//! Extracted from `accounts.rs` (#444 large-file split). These handlers all
//! carry the `ApiTags::Accounts` tag and form a self-contained cluster with no
//! dependency on private helpers or local types defined in `accounts.rs`.
//! Registration is unchanged from the consumer's perspective:
//! `AccountContactsApi` is combined with the other `*Api` types in
//! `openapi::create_combined_api`, and every path, method, tag, and schema
//! below is identical to the pre-split API.

use super::common::{AddAccountContactRequest, AddAccountSocialRequest, ApiResponse, ApiTags};
use crate::auth::ApiAuthenticatedUser;
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, OpenApi};
use std::sync::Arc;

pub struct AccountContactsApi;

#[OpenApi]
impl AccountContactsApi {
    // ==================== Account Contacts ====================
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
}

#[cfg(test)]
mod tests {
    use crate::openapi::common::{AddAccountContactRequest, AddAccountSocialRequest};

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
}
