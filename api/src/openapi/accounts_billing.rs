//! Account billing-settings endpoints.
//!
//! Extracted from `accounts.rs` (#444 large-file split). These handlers all
//! carry the `ApiTags::Accounts` tag and form a self-contained cluster with no
//! dependency on private helpers or local types defined in `accounts.rs`.
//! Registration is unchanged from the consumer's perspective:
//! `AccountBillingApi` is combined with the other `*Api` types in
//! `openapi::create_combined_api`, and every path, method, tag, and schema
//! below is identical to the pre-split API.

use super::common::{ApiResponse, ApiTags};
use crate::auth::ApiAuthenticatedUser;
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{payload::Json, OpenApi};
use std::sync::Arc;

pub struct AccountBillingApi;

#[OpenApi]
impl AccountBillingApi {
    // ==================== Account Billing ====================
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
}

#[cfg(test)]
mod tests {
    use crate::database::accounts::BillingSettings;

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
}
