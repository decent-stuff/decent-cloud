//! Offering visibility allowlist (get / add / remove) endpoints.
//!
//! Extracted from `providers.rs` (#444 large-file split). These handlers carry
//! the `ApiTags::Offerings` tag and form a self-contained cluster with no
//! dependency on private helpers or local types defined in `providers.rs`.
//! Registration is unchanged from the consumer's perspective: `AllowlistApi`
//! is combined with the other `*Api` types in `openapi::create_combined_api`,
//! and every path, method, tag, and schema below is identical to the pre-split
//! API.

use super::common::{check_authorization, decode_pubkey, AllowlistAddRequest, ApiResponse, ApiTags};
use crate::auth::ApiAuthenticatedUser;
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, OpenApi};
use std::sync::Arc;

pub struct AllowlistApi;

#[OpenApi]
impl AllowlistApi {
    /// Get offering allowlist
    ///
    /// Returns the visibility allowlist for a shared offering (requires authentication as owner)
    #[oai(
        path = "/providers/:pubkey/offerings/:id/allowlist",
        method = "get",
        tag = "ApiTags::Offerings"
    )]
    async fn get_offering_allowlist(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        id: Path<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::visibility_allowlist::AllowlistEntry>>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db.get_allowlist(id.0, &pubkey_bytes).await {
            Ok(entries) => Json(ApiResponse {
                success: true,
                data: Some(entries),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Add to offering allowlist
    ///
    /// Adds a pubkey to the visibility allowlist for a shared offering (requires authentication as owner)
    #[oai(
        path = "/providers/:pubkey/offerings/:id/allowlist",
        method = "post",
        tag = "ApiTags::Offerings"
    )]
    async fn add_to_offering_allowlist(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        id: Path<i64>,
        req: Json<AllowlistAddRequest>,
    ) -> Json<ApiResponse<i64>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        let allowed_pubkey_bytes = match decode_pubkey(&req.0.allowed_pubkey) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid allowed_pubkey: {}", e)),
                })
            }
        };

        match db
            .add_to_allowlist(id.0, &allowed_pubkey_bytes, &pubkey_bytes)
            .await
        {
            Ok(entry_id) => Json(ApiResponse {
                success: true,
                data: Some(entry_id),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Remove from offering allowlist
    ///
    /// Removes a pubkey from the visibility allowlist for a shared offering (requires authentication as owner)
    #[oai(
        path = "/providers/:pubkey/offerings/:id/allowlist/:allowed_pubkey",
        method = "delete",
        tag = "ApiTags::Offerings"
    )]
    async fn remove_from_offering_allowlist(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        id: Path<i64>,
        allowed_pubkey: Path<String>,
    ) -> Json<ApiResponse<bool>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                })
            }
        };

        if let Err(e) = check_authorization(&pubkey_bytes, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        let allowed_pubkey_bytes = match decode_pubkey(&allowed_pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid allowed_pubkey: {}", e)),
                })
            }
        };

        match db
            .remove_from_allowlist(id.0, &allowed_pubkey_bytes, &pubkey_bytes)
            .await
        {
            Ok(removed) => Json(ApiResponse {
                success: true,
                data: Some(removed),
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
