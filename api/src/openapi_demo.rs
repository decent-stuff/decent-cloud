use crate::{database::Database, metadata_cache::MetadataCache};
use poem::web::Data;
use poem_openapi::{param::Path, payload::Json, Object, OpenApi};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct MainApi;

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct HealthResponse {
    pub success: bool,
    pub message: String,
    pub environment: String,
}

#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(skip_serializing_if_is_none)]
pub struct ApiResponse<T: poem_openapi::types::ParseFromJSON + poem_openapi::types::ToJSON> {
    pub success: bool,
    #[oai(skip_serializing_if_is_none)]
    pub data: Option<T>,
    #[oai(skip_serializing_if_is_none)]
    pub error: Option<String>,
}

fn default_limit() -> i64 {
    50
}

fn default_false() -> bool {
    false
}

#[OpenApi]
impl MainApi {
    /// Health check endpoint
    ///
    /// Returns the health status of the API server
    #[oai(path = "/health", method = "get", tag = "ApiTags::System")]
    async fn health(&self) -> Json<HealthResponse> {
        let environment =
            std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        Json(HealthResponse {
            success: true,
            message: "Decent Cloud API is running".to_string(),
            environment,
        })
    }

    /// List all providers
    ///
    /// Returns a paginated list of registered providers
    #[oai(path = "/providers", method = "get", tag = "ApiTags::Providers")]
    async fn list_providers(
        &self,
        db: Data<&Arc<Database>>,
        #[oai(default = "default_limit")] limit: poem_openapi::param::Query<i64>,
        #[oai(default)] offset: poem_openapi::param::Query<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::providers::ProviderProfile>>> {
        match db.list_providers(limit.0, offset.0).await {
            Ok(providers) => Json(ApiResponse {
                success: true,
                data: Some(providers),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get active providers
    ///
    /// Returns providers that have checked in within the specified number of days
    #[oai(
        path = "/providers/active/:days",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_active_providers(
        &self,
        db: Data<&Arc<Database>>,
        days: Path<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::providers::ProviderProfile>>> {
        match db.get_active_providers(days.0).await {
            Ok(providers) => Json(ApiResponse {
                success: true,
                data: Some(providers),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Search offerings
    ///
    /// Search for offerings with optional filters
    #[oai(path = "/offerings", method = "get", tag = "ApiTags::Offerings")]
    async fn search_offerings(
        &self,
        db: Data<&Arc<Database>>,
        #[oai(default = "default_limit")] limit: poem_openapi::param::Query<i64>,
        #[oai(default)] offset: poem_openapi::param::Query<i64>,
        product_type: poem_openapi::param::Query<Option<String>>,
        country: poem_openapi::param::Query<Option<String>>,
        #[oai(default = "default_false")] in_stock_only: poem_openapi::param::Query<bool>,
    ) -> Json<ApiResponse<Vec<crate::database::offerings::Offering>>> {
        let search_params = crate::database::offerings::SearchOfferingsParams {
            product_type: product_type.0.as_deref(),
            country: country.0.as_deref(),
            in_stock_only: in_stock_only.0,
            limit: limit.0,
            offset: offset.0,
        };

        match db.search_offerings(search_params).await {
            Ok(offerings) => Json(ApiResponse {
                success: true,
                data: Some(offerings),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get active validators
    ///
    /// Returns validators that have checked in within the specified number of days
    #[oai(
        path = "/validators/active/:days",
        method = "get",
        tag = "ApiTags::Validators"
    )]
    async fn get_active_validators(
        &self,
        db: Data<&Arc<Database>>,
        days: Path<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::providers::Validator>>> {
        match db.get_active_validators(days.0).await {
            Ok(validators) => Json(ApiResponse {
                success: true,
                data: Some(validators),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider profile
    ///
    /// Returns profile information for a specific provider
    #[oai(
        path = "/providers/:pubkey",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_profile(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<crate::database::providers::ProviderProfile>> {
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

        match db.get_provider_profile(&pubkey_bytes).await {
            Ok(Some(profile)) => Json(ApiResponse {
                success: true,
                data: Some(profile),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Provider not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider contacts
    ///
    /// Returns contact information for a specific provider
    #[oai(
        path = "/providers/:pubkey/contacts",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_contacts(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::providers::ProviderContact>>> {
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

        match db.get_provider_contacts(&pubkey_bytes).await {
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

    /// Get offering by ID
    ///
    /// Returns details of a specific offering
    #[oai(path = "/offerings/:id", method = "get", tag = "ApiTags::Offerings")]
    async fn get_offering(
        &self,
        db: Data<&Arc<Database>>,
        id: Path<i64>,
    ) -> Json<ApiResponse<crate::database::offerings::Offering>> {
        match db.get_offering(id.0).await {
            Ok(Some(offering)) => Json(ApiResponse {
                success: true,
                data: Some(offering),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Offering not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// List contracts
    ///
    /// Returns a paginated list of all contracts
    #[oai(path = "/contracts", method = "get", tag = "ApiTags::Contracts")]
    async fn list_contracts(
        &self,
        db: Data<&Arc<Database>>,
        #[oai(default = "default_limit")] limit: poem_openapi::param::Query<i64>,
        #[oai(default)] offset: poem_openapi::param::Query<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::Contract>>> {
        match db.list_contracts(limit.0, offset.0).await {
            Ok(contracts) => Json(ApiResponse {
                success: true,
                data: Some(contracts),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get contract by ID
    ///
    /// Returns details of a specific contract
    #[oai(path = "/contracts/:id", method = "get", tag = "ApiTags::Contracts")]
    async fn get_contract(
        &self,
        db: Data<&Arc<Database>>,
        id: Path<String>,
    ) -> Json<ApiResponse<crate::database::contracts::Contract>> {
        let contract_id = match hex::decode(&id.0) {
            Ok(id) => id,
            Err(_) => {
                return Json(ApiResponse {
                    success: false,
                    data: None,
                    error: Some("Invalid contract ID format".to_string()),
                })
            }
        };

        match db.get_contract(&contract_id).await {
            Ok(Some(contract)) => Json(ApiResponse {
                success: true,
                data: Some(contract),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("Contract not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get user contracts
    ///
    /// Returns contracts for a specific user (as requester)
    #[oai(
        path = "/users/:pubkey/contracts",
        method = "get",
        tag = "ApiTags::Users"
    )]
    async fn get_user_contracts(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::Contract>>> {
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

        match db.get_user_contracts(&pubkey_bytes).await {
            Ok(contracts) => Json(ApiResponse {
                success: true,
                data: Some(contracts),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get provider contracts
    ///
    /// Returns contracts for a specific provider
    #[oai(
        path = "/providers/:pubkey/contracts",
        method = "get",
        tag = "ApiTags::Providers"
    )]
    async fn get_provider_contracts(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::contracts::Contract>>> {
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

        match db.get_provider_contracts(&pubkey_bytes).await {
            Ok(contracts) => Json(ApiResponse {
                success: true,
                data: Some(contracts),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get user profile
    ///
    /// Returns profile information for a specific user
    #[oai(
        path = "/users/:pubkey/profile",
        method = "get",
        tag = "ApiTags::Users"
    )]
    async fn get_user_profile(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<crate::database::users::UserProfile>> {
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

        match db.get_user_profile(&pubkey_bytes).await {
            Ok(Some(profile)) => Json(ApiResponse {
                success: true,
                data: Some(profile),
                error: None,
            }),
            Ok(None) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some("User not found".to_string()),
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get user contacts
    ///
    /// Returns contact information for a specific user
    #[oai(
        path = "/users/:pubkey/contacts",
        method = "get",
        tag = "ApiTags::Users"
    )]
    async fn get_user_contacts(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::users::UserContact>>> {
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

        match db.get_user_contacts(&pubkey_bytes).await {
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

    /// Get user socials
    ///
    /// Returns social media profiles for a specific user
    #[oai(
        path = "/users/:pubkey/socials",
        method = "get",
        tag = "ApiTags::Users"
    )]
    async fn get_user_socials(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::users::UserSocial>>> {
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

        match db.get_user_socials(&pubkey_bytes).await {
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

    /// Get user public keys
    ///
    /// Returns public keys for a specific user
    #[oai(path = "/users/:pubkey/keys", method = "get", tag = "ApiTags::Users")]
    async fn get_user_public_keys(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<Vec<crate::database::users::UserPublicKey>>> {
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

        match db.get_user_public_keys(&pubkey_bytes).await {
            Ok(keys) => Json(ApiResponse {
                success: true,
                data: Some(keys),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get user activity
    ///
    /// Returns activity summary for a specific user
    #[oai(
        path = "/users/:pubkey/activity",
        method = "get",
        tag = "ApiTags::Users"
    )]
    async fn get_user_activity(
        &self,
        db: Data<&Arc<Database>>,
        pubkey: Path<String>,
    ) -> Json<ApiResponse<crate::database::users::UserActivity>> {
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

        match db.get_user_activity(&pubkey_bytes).await {
            Ok(activity) => Json(ApiResponse {
                success: true,
                data: Some(activity),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get recent transfers
    ///
    /// Returns a list of recent token transfers
    #[oai(path = "/transfers", method = "get", tag = "ApiTags::Transfers")]
    async fn get_recent_transfers(
        &self,
        db: Data<&Arc<Database>>,
        #[oai(default = "default_limit")] limit: poem_openapi::param::Query<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::tokens::TokenTransfer>>> {
        match db.get_recent_transfers(limit.0).await {
            Ok(transfers) => Json(ApiResponse {
                success: true,
                data: Some(transfers),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get account transfers
    ///
    /// Returns transfers for a specific account
    #[oai(
        path = "/accounts/:account/transfers",
        method = "get",
        tag = "ApiTags::Transfers"
    )]
    async fn get_account_transfers(
        &self,
        db: Data<&Arc<Database>>,
        account: Path<String>,
        #[oai(default = "default_limit")] limit: poem_openapi::param::Query<i64>,
    ) -> Json<ApiResponse<Vec<crate::database::tokens::TokenTransfer>>> {
        match db.get_account_transfers(&account.0, limit.0).await {
            Ok(transfers) => Json(ApiResponse {
                success: true,
                data: Some(transfers),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get account balance
    ///
    /// Returns the balance for a specific account
    #[oai(
        path = "/accounts/:account/balance",
        method = "get",
        tag = "ApiTags::Transfers"
    )]
    async fn get_account_balance(
        &self,
        db: Data<&Arc<Database>>,
        account: Path<String>,
    ) -> Json<ApiResponse<i64>> {
        match db.get_account_balance(&account.0).await {
            Ok(balance) => Json(ApiResponse {
                success: true,
                data: Some(balance),
                error: None,
            }),
            Err(e) => Json(ApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get platform stats
    ///
    /// Returns overall platform statistics
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
}

// API Tags for grouping endpoints
#[derive(poem_openapi::Tags)]
enum ApiTags {
    /// System endpoints
    System,
    /// Provider management endpoints
    Providers,
    /// Validator management endpoints
    Validators,
    /// Offering management endpoints
    Offerings,
    /// Contract management endpoints
    Contracts,
    /// User profile endpoints
    Users,
    /// Token transfer endpoints
    Transfers,
    /// Platform statistics endpoints
    Stats,
}
