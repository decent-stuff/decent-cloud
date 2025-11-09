use crate::database::Database;
use poem::{
    handler,
    web::{Data, Json, Path, Query},
    Result as PoemResult,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Common response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(msg: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg),
        }
    }
}

// Query parameters for pagination
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

// Query parameters for search
#[derive(Debug, Deserialize)]
pub struct OfferingSearchQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub product_type: Option<String>,
    pub country: Option<String>,
    pub min_price_e9s: Option<i64>,
    pub max_price_e9s: Option<i64>,
    #[serde(default)]
    pub in_stock_only: bool,
}

// ============ Provider Endpoints ============

#[handler]
pub async fn list_providers(
    db: Data<&Arc<Database>>,
    Query(params): Query<PaginationQuery>,
) -> PoemResult<Json<ApiResponse<Vec<crate::database::providers::ProviderProfile>>>> {
    match db.list_providers(params.limit, params.offset).await {
        Ok(providers) => Ok(Json(ApiResponse::success(providers))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn get_active_providers(
    db: Data<&Arc<Database>>,
    Path(hours): Path<i64>,
) -> PoemResult<Json<ApiResponse<Vec<crate::database::providers::ProviderProfile>>>> {
    match db.get_active_providers(hours).await {
        Ok(providers) => Ok(Json(ApiResponse::success(providers))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn get_provider_profile(
    db: Data<&Arc<Database>>,
    Path(pubkey_hex): Path<String>,
) -> PoemResult<Json<ApiResponse<crate::database::providers::ProviderProfile>>> {
    let pubkey = match hex::decode(&pubkey_hex) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(Json(ApiResponse::error(
                "Invalid pubkey format".to_string(),
            )))
        }
    };

    match db.get_provider_profile(&pubkey).await {
        Ok(Some(profile)) => Ok(Json(ApiResponse::success(profile))),
        Ok(None) => Ok(Json(ApiResponse::error("Provider not found".to_string()))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn get_provider_contacts(
    db: Data<&Arc<Database>>,
    Path(pubkey_hex): Path<String>,
) -> PoemResult<Json<ApiResponse<Vec<crate::database::providers::ProviderContact>>>> {
    let pubkey = match hex::decode(&pubkey_hex) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(Json(ApiResponse::error(
                "Invalid pubkey format".to_string(),
            )))
        }
    };

    match db.get_provider_contacts(&pubkey).await {
        Ok(contacts) => Ok(Json(ApiResponse::success(contacts))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

// ============ Offering Endpoints ============

#[handler]
pub async fn search_offerings(
    db: Data<&Arc<Database>>,
    Query(params): Query<OfferingSearchQuery>,
) -> PoemResult<Json<ApiResponse<Vec<crate::database::offerings::Offering>>>> {
    match db
        .search_offerings(
            params.product_type.as_deref(),
            params.country.as_deref(),
            params.min_price_e9s,
            params.max_price_e9s,
            params.in_stock_only,
            params.limit,
            params.offset,
        )
        .await
    {
        Ok(offerings) => Ok(Json(ApiResponse::success(offerings))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn get_provider_offerings(
    db: Data<&Arc<Database>>,
    Path(pubkey_hex): Path<String>,
) -> PoemResult<Json<ApiResponse<Vec<crate::database::offerings::Offering>>>> {
    let pubkey = match hex::decode(&pubkey_hex) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(Json(ApiResponse::error(
                "Invalid pubkey format".to_string(),
            )))
        }
    };

    match db.get_provider_offerings(&pubkey).await {
        Ok(offerings) => Ok(Json(ApiResponse::success(offerings))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn get_offering(
    db: Data<&Arc<Database>>,
    Path(offering_id): Path<i64>,
) -> PoemResult<Json<ApiResponse<crate::database::offerings::Offering>>> {
    match db.get_offering(offering_id).await {
        Ok(Some(offering)) => Ok(Json(ApiResponse::success(offering))),
        Ok(None) => Ok(Json(ApiResponse::error("Offering not found".to_string()))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

// ============ Contract Endpoints ============

#[handler]
pub async fn list_contracts(
    db: Data<&Arc<Database>>,
    Query(params): Query<PaginationQuery>,
) -> PoemResult<Json<ApiResponse<Vec<crate::database::contracts::Contract>>>> {
    match db.list_contracts(params.limit, params.offset).await {
        Ok(contracts) => Ok(Json(ApiResponse::success(contracts))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn get_user_contracts(
    db: Data<&Arc<Database>>,
    Path(pubkey_hex): Path<String>,
) -> PoemResult<Json<ApiResponse<Vec<crate::database::contracts::Contract>>>> {
    let pubkey = match hex::decode(&pubkey_hex) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(Json(ApiResponse::error(
                "Invalid pubkey format".to_string(),
            )))
        }
    };

    match db.get_user_contracts(&pubkey).await {
        Ok(contracts) => Ok(Json(ApiResponse::success(contracts))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn get_provider_contracts(
    db: Data<&Arc<Database>>,
    Path(pubkey_hex): Path<String>,
) -> PoemResult<Json<ApiResponse<Vec<crate::database::contracts::Contract>>>> {
    let pubkey = match hex::decode(&pubkey_hex) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(Json(ApiResponse::error(
                "Invalid pubkey format".to_string(),
            )))
        }
    };

    match db.get_provider_contracts(&pubkey).await {
        Ok(contracts) => Ok(Json(ApiResponse::success(contracts))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn get_contract(
    db: Data<&Arc<Database>>,
    Path(contract_id_hex): Path<String>,
) -> PoemResult<Json<ApiResponse<crate::database::contracts::Contract>>> {
    let contract_id = match hex::decode(&contract_id_hex) {
        Ok(id) => id,
        Err(_) => {
            return Ok(Json(ApiResponse::error(
                "Invalid contract ID format".to_string(),
            )))
        }
    };

    match db.get_contract(&contract_id).await {
        Ok(Some(contract)) => Ok(Json(ApiResponse::success(contract))),
        Ok(None) => Ok(Json(ApiResponse::error("Contract not found".to_string()))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

// ============ Token Endpoints ============

#[handler]
pub async fn get_recent_transfers(
    db: Data<&Arc<Database>>,
    Query(params): Query<PaginationQuery>,
) -> PoemResult<Json<ApiResponse<Vec<crate::database::tokens::TokenTransfer>>>> {
    match db.get_recent_transfers(params.limit).await {
        Ok(transfers) => Ok(Json(ApiResponse::success(transfers))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn get_account_transfers(
    db: Data<&Arc<Database>>,
    Path(account): Path<String>,
    Query(params): Query<PaginationQuery>,
) -> PoemResult<Json<ApiResponse<Vec<crate::database::tokens::TokenTransfer>>>> {
    match db.get_account_transfers(&account, params.limit).await {
        Ok(transfers) => Ok(Json(ApiResponse::success(transfers))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn get_account_balance(
    db: Data<&Arc<Database>>,
    Path(account): Path<String>,
) -> PoemResult<Json<ApiResponse<i64>>> {
    match db.get_account_balance(&account).await {
        Ok(balance) => Ok(Json(ApiResponse::success(balance))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

// ============ Stats Endpoints ============

#[handler]
pub async fn get_platform_stats(
    db: Data<&Arc<Database>>,
) -> PoemResult<Json<ApiResponse<crate::database::stats::PlatformStats>>> {
    match db.get_platform_stats().await {
        Ok(stats) => Ok(Json(ApiResponse::success(stats))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn get_provider_stats(
    db: Data<&Arc<Database>>,
    Path(pubkey_hex): Path<String>,
) -> PoemResult<Json<ApiResponse<crate::database::stats::ProviderStats>>> {
    let pubkey = match hex::decode(&pubkey_hex) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(Json(ApiResponse::error(
                "Invalid pubkey format".to_string(),
            )))
        }
    };

    match db.get_provider_stats(&pubkey).await {
        Ok(stats) => Ok(Json(ApiResponse::success(stats))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn get_reputation(
    db: Data<&Arc<Database>>,
    Path(pubkey_hex): Path<String>,
) -> PoemResult<Json<ApiResponse<crate::database::stats::ReputationInfo>>> {
    let pubkey = match hex::decode(&pubkey_hex) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(Json(ApiResponse::error(
                "Invalid pubkey format".to_string(),
            )))
        }
    };

    match db.get_reputation(&pubkey).await {
        Ok(Some(rep)) => Ok(Json(ApiResponse::success(rep))),
        Ok(None) => Ok(Json(ApiResponse::error("No reputation found".to_string()))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}
