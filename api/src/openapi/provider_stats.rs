//! Provider statistics, feedback, trust metrics, health summaries, and
//! response-metrics endpoints (read-only provider analytics).
//!
//! Extracted from `providers.rs` (#444 large-file split). These handlers carry
//! the `ApiTags::Providers` tag. The response-metrics handler calls the shared
//! `build_response_metrics` helper, which stays in `providers.rs` (also used by
//! the provider-dashboard handler there) and is referenced here as `pub(crate)`.
//! Registration is unchanged from the consumer's perspective: `ProviderStatsApi`
//! is combined with the other `*Api` types in `openapi::create_combined_api`,
//! and every path, method, tag, and schema below is identical to the pre-split
//! API.

use super::common::{
    check_authorization, decode_hex_path, decode_pubkey, ApiResponse, ApiTags,
    ResponseMetricsResponse,
};
use super::providers::build_response_metrics;
use crate::auth::ApiAuthenticatedUser;
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, OpenApi};
use std::sync::Arc;

pub struct ProviderStatsApi;

#[OpenApi]
impl ProviderStatsApi {
    /// Get provider stats
    ///
    /// Returns statistics for a specific provider
    #[oai(
        path = "/providers/:pubkey/stats",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_stats(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<crate::database::stats::ProviderStats>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        match db.get_provider_stats(&pubkey_bytes).await {
            Ok(stats) => Json(ApiResponse {
                success: true,
                data: Some(stats),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get monthly revenue breakdown for a provider (last 12 months)
    #[oai(
        path = "/providers/:pubkey/revenue-by-month",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_revenue_by_month(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::stats::RevenueByMonth>>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        match db.get_provider_revenue_by_month(&pubkey_bytes).await {
            Ok(data) => Json(ApiResponse {
                success: true,
                data: Some(data),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(format!("Failed to get revenue by month: {e:#}")),
            }),
        }
    }

    /// Get provider trust metrics
    ///
    /// Returns trust score and reliability metrics for a specific provider.
    /// Includes red flag detection for concerning patterns.
    #[oai(
        path = "/providers/:pubkey/trust-metrics",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_trust_metrics(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<crate::database::stats::ProviderTrustMetrics>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        match db.get_provider_trust_metrics(&pubkey_bytes).await {
            Ok(metrics) => Json(ApiResponse {
                success: true,
                data: Some(metrics),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider feedback stats
    ///
    /// Returns aggregated user feedback statistics for a provider.
    /// Shows the percentage of renters who said the service matched its description
    /// and would rent from this provider again.
    #[oai(
        path = "/providers/:pubkey/feedback-stats",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_feedback_stats(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<crate::database::stats::ProviderFeedbackStats>> {
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

        match db.get_provider_feedback_stats(&pubkey_bytes).await {
            Ok(stats) => Json(ApiResponse {
                success: true,
                data: Some(stats),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get all feedback for a provider's contracts
    ///
    /// Returns individual feedback entries for all of the authenticated provider's contracts.
    /// Only the provider identified by the pubkey path parameter may call this endpoint.
    #[oai(
        path = "/providers/:pubkey/feedback",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_feedback_list(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::stats::ProviderContractFeedback>>> {
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

        match db.get_provider_all_feedback(&pubkey_bytes).await {
            Ok(feedback) => Json(ApiResponse {
                success: true,
                data: Some(feedback),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider health summary
    ///
    /// Returns uptime metrics and health check statistics for a provider.
    /// Aggregates health check data across all contracts for the specified time window.
    /// Default period is last 30 days.
    #[oai(
        path = "/providers/:pubkey/health-summary",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_health_summary(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
        /// Number of days to look back (default: 30)
        #[oai(default)]
        days: poem_openapi::param::Query<Option<i64>>,
    ) -> Json<ApiResponse<crate::database::contracts::ProviderHealthSummary>> {
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

        match db.get_provider_health_summary(&pubkey_bytes, days.0).await {
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

    /// Get per-contract health summary (provider view)
    ///
    /// Returns aggregated uptime metrics for a single contract.
    /// Only the provider who owns the contract can access this endpoint.
    #[oai(
        path = "/providers/:pubkey/contracts/:contract_id/health",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_contract_health_summary(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        contract_id: Path<String>,
    ) -> Json<ApiResponse<crate::database::contracts::ContractHealthSummary>> {
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

        if auth.pubkey != pubkey_bytes {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: can only access your own contracts".to_string()),
            });
        }

        let contract_id_bytes = match decode_hex_path(&contract_id.0, "contract id") {
            Ok(id) => id,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        // Validate contract belongs to this provider
        let contract = match db.get_contract(&contract_id_bytes).await {
            Ok(Some(c)) => c,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Contract not found".to_string()),
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

        if contract.provider_pubkey != hex::encode(&pubkey_bytes) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: contract does not belong to this provider".to_string()),
            });
        }

        match db.get_contract_health_summary(&contract_id_bytes).await {
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

    /// Get per-contract health checks (provider view)
    ///
    /// Returns the last 50 health check records for a single contract.
    /// Only the provider who owns the contract can access this endpoint.
    #[oai(
        path = "/providers/:pubkey/contracts/:contract_id/health-checks",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_contract_health_checks(
        &self,
        db: Data<&Arc<Database>>,
        auth: ApiAuthenticatedUser,
        pubkey: Path<String>,
        contract_id: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::ContractHealthCheck>>> {
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

        if auth.pubkey != pubkey_bytes {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: can only access your own contracts".to_string()),
            });
        }

        let contract_id_bytes = match decode_hex_path(&contract_id.0, "contract id") {
            Ok(id) => id,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        // Validate contract belongs to this provider
        let contract = match db.get_contract(&contract_id_bytes).await {
            Ok(Some(c)) => c,
            Ok(None) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Contract not found".to_string()),
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

        if contract.provider_pubkey != hex::encode(&pubkey_bytes) {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Unauthorized: contract does not belong to this provider".to_string()),
            });
        }

        match db.get_recent_health_checks(&contract_id_bytes, 50).await {
            Ok(checks) => Json(ApiResponse {
                success: true,
                data: Some(checks),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider contract request response metrics
    ///
    /// Returns response-time and SLA compliance metrics for contract rental requests.
    /// Measures how quickly a provider accepts or rejects incoming requests.
    /// This endpoint is for contract request handling, not chat message thread replies.
    #[oai(
        path = "/providers/:pubkey/response-metrics",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_response_metrics(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<ResponseMetricsResponse>> {
        let pubkey_bytes = match decode_pubkey(&pubkey.0) {
            Ok(pk) => pk,
            Err(msg) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(msg),
                })
            }
        };

        match db.get_provider_response_metrics(&pubkey_bytes).await {
            Ok(metrics) => Json(ApiResponse {
                success: true,
                data: Some(build_response_metrics(metrics)),
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
    use crate::database::test_helpers::setup_test_db;
    use crate::openapi::common::{ResponseMetricsResponse, ResponseTimeDistributionResponse};
    use poem::web::Data;
    use poem_openapi::param::Path;
    use poem_openapi::payload::Json;
    use std::sync::Arc;

    // ── ResponseMetricsResponse ──────────────────────────────────────────────

    #[test]
    fn test_response_metrics_response_optional_fields_null() {
        let dist = ResponseTimeDistributionResponse {
            within_1h_pct: 50.0,
            within_4h_pct: 70.0,
            within_12h_pct: 85.0,
            within_24h_pct: 90.0,
            within_72h_pct: 95.0,
            total_responses: 42,
        };
        let metrics = ResponseMetricsResponse {
            avg_response_seconds: None,
            avg_response_hours: None,
            sla_compliance_percent: 88.5,
            breach_count_30d: 3,
            total_inquiries_30d: 100,
            distribution: dist,
        };
        let json = serde_json::to_value(&metrics).unwrap();
        assert!(json["avgResponseSeconds"].is_null());
        assert!(json["avgResponseHours"].is_null());
        assert_eq!(json["slaCompliancePercent"], 88.5_f64);
        assert_eq!(json["breachCount30d"], 3_i64);
        assert_eq!(json["distribution"]["within1hPct"], 50.0_f64);
        assert_eq!(json["distribution"]["totalResponses"], 42_i64);
    }

    #[test]
    fn test_response_metrics_response_with_values() {
        let dist = ResponseTimeDistributionResponse {
            within_1h_pct: 0.0,
            within_4h_pct: 0.0,
            within_12h_pct: 0.0,
            within_24h_pct: 0.0,
            within_72h_pct: 0.0,
            total_responses: 0,
        };
        let metrics = ResponseMetricsResponse {
            avg_response_seconds: Some(3600.0),
            avg_response_hours: Some(1.0),
            sla_compliance_percent: 100.0,
            breach_count_30d: 0,
            total_inquiries_30d: 0,
            distribution: dist,
        };
        let json = serde_json::to_value(&metrics).unwrap();
        assert_eq!(json["avgResponseSeconds"], 3600.0_f64);
        assert_eq!(json["avgResponseHours"], 1.0_f64);
    }

    #[tokio::test]
    async fn test_get_provider_response_metrics_success_with_empty_dataset() {
        let db = Arc::new(setup_test_db().await);
        let api = super::ProviderStatsApi;
        let pubkey = "0".repeat(64);

        let Json(response) = api
            .get_provider_response_metrics(Data(&db), Path(pubkey))
            .await;

        assert!(response.success);
        assert!(response.error.is_none());

        let metrics = response.data.expect("response data should be present");
        assert!(metrics.avg_response_seconds.is_none());
        assert!(metrics.avg_response_hours.is_none());
        assert_eq!(metrics.sla_compliance_percent, 100.0);
        assert_eq!(metrics.breach_count_30d, 0);
        assert_eq!(metrics.total_inquiries_30d, 0);
        assert_eq!(metrics.distribution.total_responses, 0);
    }

    #[tokio::test]
    async fn test_get_provider_response_metrics_invalid_pubkey() {
        let db = Arc::new(setup_test_db().await);
        let api = super::ProviderStatsApi;

        let Json(response) = api
            .get_provider_response_metrics(Data(&db), Path("invalid-pubkey".to_string()))
            .await;

        assert!(!response.success);
        assert!(response.data.is_none());
        assert!(
            response
                .error
                .as_ref()
                .unwrap()
                .starts_with("Invalid pubkey hex"),
            "got: {:?}",
            response.error
        );
    }

    #[test]
    fn test_provider_response_metrics_route_is_declared() {
        const PROVIDER_STATS_RS: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/openapi/provider_stats.rs"
        ));
        assert!(
            PROVIDER_STATS_RS.contains("path = \"/providers/:pubkey/response-metrics\""),
            "Provider stats API must declare /providers/:pubkey/response-metrics route"
        );
        assert!(
            PROVIDER_STATS_RS.contains("async fn get_provider_response_metrics"),
            "Provider stats API must keep get_provider_response_metrics handler"
        );
    }
}
