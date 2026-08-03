//! Per-offering statistics endpoints for the authenticated provider
//! (read-only offering analytics: contract stats, weekly history, conversion,
//! tenant satisfaction).
//!
//! Extracted from `providers.rs` (#444 large-file split). These four handlers
//! carry the `ApiTags::Providers` tag and return DB-layer types
//! (`crate::database::users::{OfferingStats, OfferingStatsWeek}`,
//! `crate::database::stats::{OfferingConversionStats, OfferingSatisfactionStats}`)
//! — there are no local request/response types to move. Registration is
//! unchanged from the consumer's perspective: `OfferingStatsApi` is combined
//! with the other `*Api` types in `openapi::create_combined_api`, and every
//! path, method, tag, and schema below is identical to the pre-split API (verified
//! byte-identical via `openapi::spec_snapshot`).

use super::common::{check_authorization, decode_pubkey, default_weeks, ApiResponse, ApiTags};
use crate::auth::ApiAuthenticatedUser;
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, OpenApi};
use std::sync::Arc;

pub struct OfferingStatsApi;

#[OpenApi]
impl OfferingStatsApi {
    /// Get per-offering contract statistics for a provider
    ///
    /// Returns aggregated contract counts and revenue broken down by offering.
    /// Requires provider authentication — only the provider can access their own stats.
    #[oai(
        path = "/providers/:pubkey/offering-stats",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_offering_stats(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::users::OfferingStats>>> {
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                });
            }
        };

        if let Err(e) = check_authorization(&provider_pubkey, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db.get_offering_stats(&provider_pubkey).await {
            Ok(stats) => Json(ApiResponse {
                success: true,
                data: Some(stats),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to get offering stats: {e:#}")),
            }),
        }
    }

    /// Get weekly offering stats history for a provider
    ///
    /// Returns per-offering weekly contract counts and revenue for the last N weeks.
    /// Requires provider authentication — only the provider can access their own stats.
    #[oai(
        path = "/providers/:pubkey/offering-stats-history",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_offering_stats_history(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        #[oai(default = "default_weeks")] weeks: poem_openapi::param::Query<i32>,
    ) -> Json<ApiResponse<Vec<crate::database::users::OfferingStatsWeek>>> {
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                });
            }
        };

        if let Err(e) = check_authorization(&provider_pubkey, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        let weeks = weeks.0.clamp(1, 52);
        match db.get_offering_stats_history(&provider_pubkey, weeks).await {
            Ok(rows) => Json(ApiResponse {
                success: true,
                data: Some(rows),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to get offering stats history: {e:#}")),
            }),
        }
    }

    /// Get per-offering conversion stats for a provider
    ///
    /// Returns views vs rentals breakdown per offering for the last 7 and 30 days.
    /// Requires provider authentication — only the provider can access their own stats.
    #[oai(
        path = "/providers/:pubkey/offering-conversion-stats",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_offering_conversion_stats(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::stats::OfferingConversionStats>>> {
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                });
            }
        };

        if let Err(e) = check_authorization(&provider_pubkey, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db.get_offering_conversion_stats(&provider_pubkey).await {
            Ok(stats) => Json(ApiResponse {
                success: true,
                data: Some(stats),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to get offering conversion stats: {e:#}")),
            }),
        }
    }

    /// Get per-offering tenant satisfaction stats for the authenticated provider
    #[oai(
        path = "/providers/:pubkey/offering-satisfaction-stats",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_offering_satisfaction_stats(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::stats::OfferingSatisfactionStats>>> {
        let provider_pubkey = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e),
                });
            }
        };

        if let Err(e) = check_authorization(&provider_pubkey, &auth) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e),
            });
        }

        match db.get_offering_satisfaction_stats(&provider_pubkey).await {
            Ok(stats) => Json(ApiResponse {
                success: true,
                data: Some(stats),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to get offering satisfaction stats: {e:#}")),
            }),
        }
    }
}
