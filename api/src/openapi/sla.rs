//! Provider SLA uptime alert configuration and SLA summary endpoints.
//!
//! Extracted from `providers.rs` (#444 large-file split). These handlers all
//! carry the `ApiTags::Providers` tag and form a self-contained cluster with no
//! dependency on private helpers or local types defined in `providers.rs` other
//! than `default_sla_days`, which is moved here verbatim (it was used only by
//! `get_provider_sla_summary`). Registration is unchanged from the consumer's
//! perspective: `SlaApi` is combined with the other `*Api` types in
//! `openapi::create_combined_api`, and every path, method, tag, and schema
//! below is identical to the pre-split API.

use super::common::{check_authorization, decode_pubkey, ApiResponse, ApiTags, UpdateSlaUptimeConfigRequest};
use crate::auth::ApiAuthenticatedUser;
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, OpenApi};
use std::sync::Arc;

/// Default lookback window (in days) for `get_provider_sla_summary`.
///
/// Moved verbatim from `providers.rs`; this was its only call site.
fn default_sla_days() -> i64 {
    30
}

pub struct SlaApi;

#[OpenApi]
impl SlaApi {
    /// Get provider SLA uptime alert configuration
    ///
    /// Returns the authenticated provider's uptime threshold and alert window settings.
    /// Defaults to 95% threshold and 24-hour window if no config row exists yet.
    #[oai(
        path = "/providers/:pubkey/sla-uptime-config",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_sla_uptime_config(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<crate::database::providers::SlaUptimeConfig>> {
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

        match db.get_provider_sla_uptime_config(&pubkey_bytes).await {
            Ok(Some(config)) => Json(ApiResponse {
                success: true,
                data: Some(config),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: true,
                data: Some(crate::database::providers::SlaUptimeConfig {
                    uptime_threshold_percent: 95,
                    sla_alert_window_hours: 24,
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

    /// Update provider SLA uptime alert configuration
    ///
    /// Sets the uptime threshold (1–100%) and alert window (hours) for the authenticated provider.
    /// You'll receive a notification when any contract's uptime drops below the threshold.
    #[oai(
        path = "/providers/:pubkey/sla-uptime-config",
        method = "put",
        tag = "ApiTags::Providers"
    )]
    async fn update_provider_sla_uptime_config(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        req: Json<UpdateSlaUptimeConfigRequest>,
    ) -> Json<ApiResponse<String>> {
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

        if req.uptime_threshold_percent < 1 || req.uptime_threshold_percent > 100 {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("uptime_threshold_percent must be between 1 and 100".to_string()),
            });
        }
        if req.sla_alert_window_hours < 1 || req.sla_alert_window_hours > 168 {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("sla_alert_window_hours must be between 1 and 168".to_string()),
            });
        }

        match db
            .upsert_provider_sla_uptime_config(
                &pubkey_bytes,
                req.uptime_threshold_percent,
                req.sla_alert_window_hours,
            )
            .await
        {
            Ok(()) => Json(ApiResponse {
                success: true,
                data: Some("SLA uptime configuration updated successfully".to_string()),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider-reported SLA summary across all tracked offerings
    #[oai(
        path = "/providers/:pubkey/sla-summary",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_sla_summary(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
        #[oai(default = "default_sla_days")] days: poem_openapi::param::Query<i64>,
    ) -> Json<ApiResponse<crate::database::offering_sla::ProviderSlaSummary>> {
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

        match db
            .get_provider_sla_summary(&pubkey_bytes, days.0.clamp(1, 90))
            .await
        {
            Ok(summary) => Json(ApiResponse {
                success: true,
                data: Some(summary),
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
