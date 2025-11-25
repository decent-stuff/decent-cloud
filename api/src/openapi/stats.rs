use super::common::{ApiResponse, ApiTags};
use crate::{database::Database, metadata_cache::MetadataCache};
use poem::web::Data;
use poem_openapi::{param::Path, param::Query, payload::Json, OpenApi};
use std::sync::Arc;

pub struct StatsApi;

#[OpenApi]
impl StatsApi {
    /// Get platform stats
    ///
    /// Returns overall platform statistics including provider, offering, and contract counts
    #[oai(path = "/stats", method = "get", tag = "ApiTags::Stats")]
    async fn get_platform_stats(
        &self,
        db: Data<&Arc<Database>>,
        metadata_cache: Data<&Arc<MetadataCache>>,
    ) -> Json<ApiResponse<crate::api_handlers::PlatformOverview>> {
        use std::collections::BTreeMap;

        let base_stats = match db.get_platform_stats().await {
            Ok(stats) => stats,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        // Count providers who checked in within last 24 hours
        let cutoff_24h =
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) - 24 * 3600 * 1_000_000_000;
        let validator_count: (i64,) = match sqlx::query_as(
            "SELECT COUNT(DISTINCT pubkey) FROM provider_check_ins WHERE block_timestamp_ns > ?",
        )
        .bind(cutoff_24h)
        .fetch_one(&db.pool)
        .await
        {
            Ok(count) => count,
            Err(e) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })
            }
        };

        // Get latest block timestamp from database
        let latest_block_timestamp_ns = match db.get_latest_block_timestamp_ns().await {
            Ok(Some(ts)) if ts > 0 => Some(ts as u64),
            _ => None,
        };

        // Get all metadata from cache as JSON
        let metadata_map = match metadata_cache.get() {
            Ok(m) => m.to_json_map(),
            Err(_) => BTreeMap::new(),
        };

        let response = crate::api_handlers::PlatformOverview {
            total_providers: base_stats.total_providers,
            active_providers: base_stats.active_providers,
            total_offerings: base_stats.total_offerings,
            total_contracts: base_stats.total_contracts,
            total_transfers: base_stats.total_transfers,
            total_volume_e9s: base_stats.total_volume_e9s,
            validator_count_24h: validator_count.0,
            latest_block_timestamp_ns,
            metadata: metadata_map,
        };

        Json(ApiResponse {
            success: true,
            data: Some(response),
            error: None,
        })
    }

    /// Get reputation
    ///
    /// Returns reputation information for a specific public key
    #[oai(path = "/reputation/:pubkey", method = "get", tag = "ApiTags::Stats")]
    async fn get_reputation(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<crate::database::stats::ReputationInfo>> {
        let pubkey_bytes = match hex::decode(&pubkey.0) {
            Ok(pk) => pk,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid pubkey format".to_string()),
                })
            }
        };

        match db.get_reputation(&pubkey_bytes).await {
            Ok(Some(reputation)) => Json(ApiResponse {
                success: true,
                data: Some(reputation),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Reputation not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Search accounts
    ///
    /// Search for accounts by username, display name, or public key.
    /// Returns accounts with reputation and activity stats.
    #[oai(path = "/reputation/search", method = "get", tag = "ApiTags::Stats")]
    async fn search_reputation(
        &self,
        db: Data<&Arc<Database>>,
        #[oai(name = "q")] query: Query<String>,
        #[oai(name = "limit")] limit: Query<Option<i64>>,
    ) -> Json<ApiResponse<Vec<crate::database::stats::AccountSearchResult>>> {
        let search_limit = limit.0.unwrap_or(50).min(100);

        if query.0.is_empty() {
            return Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Search query cannot be empty".to_string()),
            });
        }

        match db.search_accounts(&query.0, search_limit).await {
            Ok(results) => Json(ApiResponse {
                success: true,
                data: Some(results),
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
