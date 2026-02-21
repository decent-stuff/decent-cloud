use super::common::{decode_pubkey, ApiResponse, ApiTags};
use crate::{database::Database, metadata_cache::MetadataCache};
use poem::web::Data;
use poem_openapi::{param::Path, param::Query, payload::Json, OpenApi};
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Serialize, ts_rs::TS, poem_openapi::Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(skip_serializing_if_is_none)]
pub struct PlatformOverview {
    // Database-derived statistics (always available, reliable)
    #[ts(type = "number")]
    pub total_providers: i64,
    #[ts(type = "number")]
    pub active_providers: i64,
    #[ts(type = "number")]
    pub total_offerings: i64,
    #[ts(type = "number")]
    pub total_contracts: i64,
    #[ts(type = "number")]
    pub total_transfers: i64,
    #[ts(type = "number")]
    pub total_volume_e9s: i64,
    #[ts(type = "number")]
    pub validator_count_24h: i64,
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub latest_block_timestamp_ns: Option<u64>,
    // All canister metadata (flexible, future-proof)
    #[ts(type = "Record<string, any>")]
    pub metadata: BTreeMap<String, JsonValue>,
}

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
    ) -> Json<ApiResponse<PlatformOverview>> {
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
            "SELECT COUNT(DISTINCT pubkey) FROM provider_check_ins WHERE block_timestamp_ns > $1",
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

        let response = PlatformOverview {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::stats::AccountSearchResult;

    #[test]
    fn test_platform_overview_serialization_with_timestamp() {
        let overview = PlatformOverview {
            total_providers: 10,
            active_providers: 5,
            total_offerings: 20,
            total_contracts: 100,
            total_transfers: 50,
            total_volume_e9s: 1_000_000_000_000,
            validator_count_24h: 3,
            latest_block_timestamp_ns: Some(1_700_000_000_000_000_000),
            metadata: BTreeMap::new(),
        };
        let json = serde_json::to_value(&overview).unwrap();
        assert_eq!(json["latest_block_timestamp_ns"], 1_700_000_000_000_000_000u64);
        assert_eq!(json["total_providers"], 10);
    }

    #[test]
    fn test_platform_overview_serialization_without_timestamp() {
        let overview = PlatformOverview {
            total_providers: 0,
            active_providers: 0,
            total_offerings: 0,
            total_contracts: 0,
            total_transfers: 0,
            total_volume_e9s: 0,
            validator_count_24h: 0,
            latest_block_timestamp_ns: None,
            metadata: BTreeMap::new(),
        };
        let json = serde_json::to_value(&overview).unwrap();
        assert!(json.get("latest_block_timestamp_ns").is_none());
    }

    #[test]
    fn test_platform_overview_metadata_serialization() {
        let mut metadata = BTreeMap::new();
        metadata.insert("version".to_string(), JsonValue::String("1.0".to_string()));
        metadata.insert("block_height".to_string(), JsonValue::Number(42.into()));

        let overview = PlatformOverview {
            total_providers: 1,
            active_providers: 1,
            total_offerings: 1,
            total_contracts: 0,
            total_transfers: 0,
            total_volume_e9s: 0,
            validator_count_24h: 0,
            latest_block_timestamp_ns: None,
            metadata,
        };
        let json = serde_json::to_value(&overview).unwrap();
        assert_eq!(json["metadata"]["version"], "1.0");
        assert_eq!(json["metadata"]["block_height"], 42);
    }

    /// Helper that mirrors the handler's limit clamping: `limit.unwrap_or(50).min(100)`
    fn clamp_search_limit(limit: Option<i64>) -> i64 {
        limit.unwrap_or(50).min(100)
    }

    #[test]
    fn test_search_limit_clamping_default() {
        assert_eq!(clamp_search_limit(None), 50);
    }

    #[test]
    fn test_search_limit_clamping_within_bounds() {
        assert_eq!(clamp_search_limit(Some(75)), 75);
    }

    #[test]
    fn test_search_limit_clamping_above_max() {
        assert_eq!(clamp_search_limit(Some(200)), 100);
    }

    #[test]
    fn test_account_search_result_with_display_name() {
        let result = AccountSearchResult {
            username: "alice".to_string(),
            display_name: Some("Alice Wonderland".to_string()),
            pubkey: "ab".repeat(32),
            reputation_score: 100,
            contract_count: 5,
            offering_count: 2,
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["username"], "alice");
        assert_eq!(json["display_name"], "Alice Wonderland");
        assert_eq!(json["reputation_score"], 100);
    }

    #[test]
    fn test_account_search_result_without_display_name() {
        let result = AccountSearchResult {
            username: "bob".to_string(),
            display_name: None,
            pubkey: "cd".repeat(32),
            reputation_score: 0,
            contract_count: 0,
            offering_count: 0,
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["username"], "bob");
        assert!(json.get("display_name").is_none());
        assert_eq!(json["contract_count"], 0);
    }
}
