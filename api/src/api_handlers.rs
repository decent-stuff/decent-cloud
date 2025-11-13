use crate::{auth::AuthenticatedUser, database::Database, metadata_cache::MetadataCache};
use poem::{
    handler,
    http::StatusCode,
    web::{Data, Json, Path, Query},
    Result as PoemResult,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
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

    /// Convert Result<T, E> to ApiResponse<T>
    pub fn from_result<E: std::fmt::Display>(result: Result<T, E>) -> Self {
        match result {
            Ok(data) => Self::success(data),
            Err(e) => Self::error(e.to_string()),
        }
    }
}

// Helper functions for common patterns
fn decode_pubkey(pubkey_hex: &str) -> Result<Vec<u8>, String> {
    hex::decode(pubkey_hex).map_err(|_| "Invalid pubkey format".to_string())
}

fn check_authorization(pubkey: &[u8], user: &AuthenticatedUser) -> Result<(), String> {
    if pubkey != user.pubkey_hash {
        Err("Unauthorized".to_string())
    } else {
        Ok(())
    }
}

#[derive(Debug, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
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
    pub latest_block_timestamp_ns: Option<u64>,

    // All canister metadata (flexible, future-proof)
    // Includes: num_blocks, blocks_until_next_halving, current_block_validators,
    // current_block_rewards_e9s, reward_per_block_e9s, token_value_in_usd_e6,
    // latest_block_hash, and any future metadata fields
    #[ts(type = "Record<string, any>")]
    pub metadata: BTreeMap<String, JsonValue>,
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
pub async fn get_active_validators(
    db: Data<&Arc<Database>>,
    Path(days): Path<i64>,
) -> PoemResult<Json<ApiResponse<Vec<crate::database::providers::Validator>>>> {
    match db.get_active_validators(days).await {
        Ok(validators) => Ok(Json(ApiResponse::success(validators))),
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
        .search_offerings(crate::database::offerings::SearchOfferingsParams {
            product_type: params.product_type.as_deref(),
            country: params.country.as_deref(),
            in_stock_only: params.in_stock_only,
            limit: params.limit,
            offset: params.offset,
        })
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

#[handler]
pub async fn create_provider_offering(
    db: Data<&Arc<Database>>,
    user: AuthenticatedUser,
    Path(pubkey_hex): Path<String>,
    Json(mut params): Json<crate::database::offerings::Offering>,
) -> PoemResult<Json<ApiResponse<i64>>> {
    let pubkey = match decode_pubkey(&pubkey_hex) {
        Ok(pk) => pk,
        Err(e) => return Ok(Json(ApiResponse::error(e))),
    };

    if let Err(e) = check_authorization(&pubkey, &user) {
        return Ok(Json(ApiResponse::error(e)));
    }

    // Ensure id is None for creation and set pubkey_hash
    params.id = None;
    params.pubkey_hash = pubkey.clone();

    let result = db.create_offering(&pubkey, params).await;
    Ok(Json(ApiResponse::from_result(result)))
}

#[handler]
pub async fn update_provider_offering(
    db: Data<&Arc<Database>>,
    user: AuthenticatedUser,
    Path((pubkey_hex, offering_id)): Path<(String, i64)>,
    Json(mut params): Json<crate::database::offerings::Offering>,
) -> PoemResult<Json<ApiResponse<()>>> {
    let pubkey = match decode_pubkey(&pubkey_hex) {
        Ok(pk) => pk,
        Err(e) => return Ok(Json(ApiResponse::error(e))),
    };

    if let Err(e) = check_authorization(&pubkey, &user) {
        return Ok(Json(ApiResponse::error(e)));
    }

    // Set pubkey_hash from authenticated user
    params.pubkey_hash = pubkey.clone();

    let result = db.update_offering(&pubkey, offering_id, params).await;
    Ok(Json(ApiResponse::from_result(result)))
}

#[handler]
pub async fn delete_provider_offering(
    db: Data<&Arc<Database>>,
    user: AuthenticatedUser,
    Path((pubkey_hex, offering_id)): Path<(String, i64)>,
) -> PoemResult<Json<ApiResponse<()>>> {
    let pubkey = match decode_pubkey(&pubkey_hex) {
        Ok(pk) => pk,
        Err(e) => return Ok(Json(ApiResponse::error(e))),
    };

    if let Err(e) = check_authorization(&pubkey, &user) {
        return Ok(Json(ApiResponse::error(e)));
    }

    let result = db.delete_offering(&pubkey, offering_id).await;
    Ok(Json(ApiResponse::from_result(result)))
}

#[derive(Debug, serde::Deserialize)]
pub struct DuplicateOfferingRequest {
    pub new_offering_id: String,
}

#[handler]
pub async fn duplicate_provider_offering(
    db: Data<&Arc<Database>>,
    user: AuthenticatedUser,
    Path((pubkey_hex, offering_id)): Path<(String, i64)>,
    Json(req): Json<DuplicateOfferingRequest>,
) -> PoemResult<Json<ApiResponse<i64>>> {
    let pubkey = match hex::decode(&pubkey_hex) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(Json(ApiResponse::error(
                "Invalid pubkey format".to_string(),
            )))
        }
    };

    if pubkey != user.pubkey_hash {
        return Ok(Json(ApiResponse::error("Unauthorized".to_string())));
    }

    match db
        .duplicate_offering(&pubkey, offering_id, req.new_offering_id)
        .await
    {
        Ok(new_id) => Ok(Json(ApiResponse::success(new_id))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct BulkUpdateStatusRequest {
    pub offering_ids: Vec<i64>,
    pub stock_status: String,
}

#[handler]
pub async fn bulk_update_provider_offerings_status(
    db: Data<&Arc<Database>>,
    user: AuthenticatedUser,
    Path(pubkey_hex): Path<String>,
    Json(req): Json<BulkUpdateStatusRequest>,
) -> PoemResult<Json<ApiResponse<usize>>> {
    let pubkey = match hex::decode(&pubkey_hex) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(Json(ApiResponse::error(
                "Invalid pubkey format".to_string(),
            )))
        }
    };

    if pubkey != user.pubkey_hash {
        return Ok(Json(ApiResponse::error("Unauthorized".to_string())));
    }

    match db
        .bulk_update_stock_status(&pubkey, &req.offering_ids, &req.stock_status)
        .await
    {
        Ok(count) => Ok(Json(ApiResponse::success(count))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn export_provider_offerings_csv(
    db: Data<&Arc<Database>>,
    user: AuthenticatedUser,
    Path(pubkey_hex): Path<String>,
) -> PoemResult<poem::Response> {
    let pubkey = match hex::decode(&pubkey_hex) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(poem::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body("Invalid pubkey format"))
        }
    };

    if pubkey != user.pubkey_hash {
        return Ok(poem::Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body("Unauthorized"));
    }

    match db.get_provider_offerings(&pubkey).await {
        Ok(offerings) => {
            let mut csv_writer = csv::Writer::from_writer(vec![]);

            // Write header
            csv_writer
                .write_record([
                    "offering_id",
                    "offer_name",
                    "description",
                    "product_page_url",
                    "currency",
                    "monthly_price",
                    "setup_fee",
                    "visibility",
                    "product_type",
                    "virtualization_type",
                    "billing_interval",
                    "stock_status",
                    "processor_brand",
                    "processor_amount",
                    "processor_cores",
                    "processor_speed",
                    "processor_name",
                    "memory_error_correction",
                    "memory_type",
                    "memory_amount",
                    "hdd_amount",
                    "total_hdd_capacity",
                    "ssd_amount",
                    "total_ssd_capacity",
                    "unmetered_bandwidth",
                    "uplink_speed",
                    "traffic",
                    "datacenter_country",
                    "datacenter_city",
                    "datacenter_latitude",
                    "datacenter_longitude",
                    "control_panel",
                    "gpu_name",
                    "min_contract_hours",
                    "max_contract_hours",
                    "payment_methods",
                    "features",
                    "operating_systems",
                ])
                .map_err(|e| {
                    poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
                })?;

            // Write data rows
            for offering in offerings {
                csv_writer
                    .write_record([
                        &offering.offering_id,
                        &offering.offer_name,
                        &offering.description.unwrap_or_default(),
                        &offering.product_page_url.unwrap_or_default(),
                        &offering.currency,
                        &offering.monthly_price.to_string(),
                        &offering.setup_fee.to_string(),
                        &offering.visibility,
                        &offering.product_type,
                        &offering.virtualization_type.unwrap_or_default(),
                        &offering.billing_interval,
                        &offering.stock_status,
                        &offering.processor_brand.unwrap_or_default(),
                        &offering
                            .processor_amount
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering
                            .processor_cores
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering.processor_speed.unwrap_or_default(),
                        &offering.processor_name.unwrap_or_default(),
                        &offering.memory_error_correction.unwrap_or_default(),
                        &offering.memory_type.unwrap_or_default(),
                        &offering.memory_amount.unwrap_or_default(),
                        &offering
                            .hdd_amount
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering.total_hdd_capacity.unwrap_or_default(),
                        &offering
                            .ssd_amount
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering.total_ssd_capacity.unwrap_or_default(),
                        &offering.unmetered_bandwidth.to_string(),
                        &offering.uplink_speed.unwrap_or_default(),
                        &offering.traffic.map(|v| v.to_string()).unwrap_or_default(),
                        &offering.datacenter_country,
                        &offering.datacenter_city,
                        &offering
                            .datacenter_latitude
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering
                            .datacenter_longitude
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering.control_panel.unwrap_or_default(),
                        &offering.gpu_name.unwrap_or_default(),
                        &offering
                            .min_contract_hours
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering
                            .max_contract_hours
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        &offering.payment_methods.unwrap_or_default(),
                        &offering.features.unwrap_or_default(),
                        &offering.operating_systems.unwrap_or_default(),
                    ])
                    .map_err(|e| {
                        poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
                    })?;
            }

            let csv_data = csv_writer.into_inner().map_err(|e| {
                poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
            })?;

            Ok(poem::Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/csv")
                .header(
                    "Content-Disposition",
                    "attachment; filename=\"offerings.csv\"",
                )
                .body(csv_data))
        }
        Err(e) => Ok(poem::Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(e.to_string())),
    }
}

#[handler]
pub async fn generate_csv_template(db: Data<&Arc<Database>>) -> PoemResult<poem::Response> {
    let mut csv_writer = csv::Writer::from_writer(vec![]);

    // Write header row
    csv_writer
        .write_record([
            "offering_id",
            "offer_name",
            "description",
            "product_page_url",
            "currency",
            "monthly_price",
            "setup_fee",
            "visibility",
            "product_type",
            "virtualization_type",
            "billing_interval",
            "stock_status",
            "processor_brand",
            "processor_amount",
            "processor_cores",
            "processor_speed",
            "processor_name",
            "memory_error_correction",
            "memory_type",
            "memory_amount",
            "hdd_amount",
            "total_hdd_capacity",
            "ssd_amount",
            "total_ssd_capacity",
            "unmetered_bandwidth",
            "uplink_speed",
            "traffic",
            "datacenter_country",
            "datacenter_city",
            "datacenter_latitude",
            "datacenter_longitude",
            "control_panel",
            "gpu_name",
            "min_contract_hours",
            "max_contract_hours",
            "payment_methods",
            "features",
            "operating_systems",
        ])
        .map_err(|e| poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

    // Get example offerings from database
    let example_offerings = match db.get_example_offerings().await {
        Ok(offerings) => offerings,
        Err(e) => {
            return Ok(poem::Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("Failed to retrieve example offerings: {}", e)))
        }
    };

    // Write example offerings from database
    for offering in example_offerings {
        // Get related data for each offering
        let payment_methods = offering.payment_methods.as_deref().unwrap_or("");
        let features = offering.features.as_deref().unwrap_or("");
        let operating_systems = offering.operating_systems.as_deref().unwrap_or("");

        csv_writer
            .write_record([
                &offering.offering_id,
                &offering.offer_name,
                &offering.description.unwrap_or_default(),
                &offering.product_page_url.unwrap_or_default(),
                &offering.currency,
                &offering.monthly_price.to_string(),
                &offering.setup_fee.to_string(),
                &offering.visibility,
                &offering.product_type,
                &offering.virtualization_type.unwrap_or_default(),
                &offering.billing_interval,
                &offering.stock_status,
                &offering.processor_brand.unwrap_or_default(),
                &offering
                    .processor_amount
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                &offering
                    .processor_cores
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                &offering.processor_speed.unwrap_or_default(),
                &offering.processor_name.unwrap_or_default(),
                &offering.memory_error_correction.unwrap_or_default(),
                &offering.memory_type.unwrap_or_default(),
                &offering.memory_amount.unwrap_or_default(),
                &offering
                    .hdd_amount
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                &offering.total_hdd_capacity.unwrap_or_default(),
                &offering
                    .ssd_amount
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                &offering.total_ssd_capacity.unwrap_or_default(),
                &offering.unmetered_bandwidth.to_string(),
                &offering.uplink_speed.unwrap_or_default(),
                &offering.traffic.map(|v| v.to_string()).unwrap_or_default(),
                &offering.datacenter_country,
                &offering.datacenter_city,
                &offering
                    .datacenter_latitude
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                &offering
                    .datacenter_longitude
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                &offering.control_panel.unwrap_or_default(),
                &offering.gpu_name.unwrap_or_default(),
                &offering
                    .min_contract_hours
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                &offering
                    .max_contract_hours
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                payment_methods,
                features,
                operating_systems,
            ])
            .map_err(|e| {
                poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
            })?;
    }

    let csv_data = csv_writer
        .into_inner()
        .map_err(|e| poem::Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok(poem::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/csv")
        .header(
            "Content-Disposition",
            "attachment; filename=\"offerings-template.csv\"",
        )
        .body(csv_data))
}

#[derive(Debug, Serialize)]
pub struct CsvImportResult {
    pub success_count: usize,
    pub errors: Vec<CsvImportError>,
}

#[derive(Debug, Serialize)]
pub struct CsvImportError {
    pub row: usize,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct ImportOfferingsQuery {
    #[serde(default)]
    pub upsert: bool,
}

#[handler]
pub async fn import_provider_offerings_csv(
    db: Data<&Arc<Database>>,
    user: AuthenticatedUser,
    Path(pubkey_hex): Path<String>,
    Query(query): Query<ImportOfferingsQuery>,
    body: String,
) -> PoemResult<Json<ApiResponse<CsvImportResult>>> {
    let pubkey = match hex::decode(&pubkey_hex) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(Json(ApiResponse::error(
                "Invalid pubkey format".to_string(),
            )))
        }
    };

    if pubkey != user.pubkey_hash {
        return Ok(Json(ApiResponse::error("Unauthorized".to_string())));
    }

    match db.import_offerings_csv(&pubkey, &body, query.upsert).await {
        Ok((success_count, errors)) => {
            let result = CsvImportResult {
                success_count,
                errors: errors
                    .into_iter()
                    .map(|(row, message)| CsvImportError { row, message })
                    .collect(),
            };
            Ok(Json(ApiResponse::success(result)))
        }
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

#[derive(Debug, Serialize)]
pub struct RentalRequestResponse {
    pub contract_id: String,
    pub message: String,
}

#[handler]
pub async fn create_rental_request(
    auth: AuthenticatedUser,
    db: Data<&Arc<Database>>,
    Json(params): Json<crate::database::contracts::RentalRequestParams>,
) -> PoemResult<Json<ApiResponse<RentalRequestResponse>>> {
    match db.create_rental_request(&auth.pubkey_hash, params).await {
        Ok(contract_id) => {
            let contract_id_hex = hex::encode(&contract_id);
            Ok(Json(ApiResponse::success(RentalRequestResponse {
                contract_id: contract_id_hex,
                message: "Rental request created successfully".to_string(),
            })))
        }
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

// ============ Provider Rental Management Endpoints ============

#[handler]
pub async fn get_pending_rental_requests(
    auth: AuthenticatedUser,
    db: Data<&Arc<Database>>,
) -> PoemResult<Json<ApiResponse<Vec<crate::database::contracts::Contract>>>> {
    match db.get_pending_provider_contracts(&auth.pubkey_hash).await {
        Ok(contracts) => Ok(Json(ApiResponse::success(contracts))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[derive(Debug, Deserialize)]
pub struct RentalResponseRequest {
    pub accept: bool,
    pub memo: Option<String>,
}

#[handler]
pub async fn respond_to_rental_request(
    auth: AuthenticatedUser,
    db: Data<&Arc<Database>>,
    Path(contract_id_hex): Path<String>,
    Json(req): Json<RentalResponseRequest>,
) -> PoemResult<Json<ApiResponse<String>>> {
    let contract_id = match hex::decode(&contract_id_hex) {
        Ok(id) => id,
        Err(_) => {
            return Ok(Json(ApiResponse::error(
                "Invalid contract ID format".to_string(),
            )))
        }
    };

    let new_status = if req.accept { "accepted" } else { "rejected" };

    match db
        .update_contract_status(
            &contract_id,
            new_status,
            &auth.pubkey_hash,
            req.memo.as_deref(),
        )
        .await
    {
        Ok(_) => Ok(Json(ApiResponse::success(format!(
            "Contract {}",
            new_status
        )))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[derive(Debug, Deserialize)]
pub struct ProvisioningStatusRequest {
    pub status: String,
    pub instance_details: Option<String>,
}

fn normalize_provisioning_details(
    status: &str,
    details: Option<String>,
) -> Result<Option<String>, String> {
    let sanitized = details.and_then(|raw| {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });

    if status == "provisioned" && sanitized.is_none() {
        return Err(
            "Instance details are required when marking a contract as provisioned".to_string(),
        );
    }

    Ok(sanitized)
}

#[handler]
pub async fn update_provisioning_status(
    auth: AuthenticatedUser,
    db: Data<&Arc<Database>>,
    Path(contract_id_hex): Path<String>,
    Json(req): Json<ProvisioningStatusRequest>,
) -> PoemResult<Json<ApiResponse<String>>> {
    let contract_id = match hex::decode(&contract_id_hex) {
        Ok(id) => id,
        Err(_) => {
            return Ok(Json(ApiResponse::error(
                "Invalid contract ID format".to_string(),
            )))
        }
    };

    let ProvisioningStatusRequest {
        status,
        instance_details,
    } = req;

    let sanitized_details = match normalize_provisioning_details(&status, instance_details) {
        Ok(details) => details,
        Err(msg) => return Ok(Json(ApiResponse::error(msg))),
    };

    // Update status
    match db
        .update_contract_status(&contract_id, &status, &auth.pubkey_hash, None)
        .await
    {
        Ok(_) => {
            // If provisioned status and instance details provided, add details
            if status == "provisioned" {
                if let Some(details) = sanitized_details.as_deref() {
                    if let Err(e) = db.add_provisioning_details(&contract_id, details).await {
                        return Ok(Json(ApiResponse::error(format!(
                            "Status updated but failed to save details: {}",
                            e
                        ))));
                    }
                }
            }
            Ok(Json(ApiResponse::success(
                "Provisioning status updated".to_string(),
            )))
        }
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[derive(Debug, Deserialize)]
pub struct ExtendContractRequest {
    pub extension_hours: i64,
    pub memo: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExtendContractResponse {
    pub extension_payment_e9s: i64,
    pub new_end_timestamp_ns: i64,
    pub message: String,
}

#[handler]
pub async fn extend_contract(
    auth: AuthenticatedUser,
    db: Data<&Arc<Database>>,
    Path(contract_id_hex): Path<String>,
    Json(req): Json<ExtendContractRequest>,
) -> PoemResult<Json<ApiResponse<ExtendContractResponse>>> {
    let contract_id = match hex::decode(&contract_id_hex) {
        Ok(id) => id,
        Err(_) => {
            return Ok(Json(ApiResponse::error(
                "Invalid contract ID format".to_string(),
            )))
        }
    };

    match db
        .extend_contract(
            &contract_id,
            &auth.pubkey_hash,
            req.extension_hours,
            req.memo,
        )
        .await
    {
        Ok(extension_payment_e9s) => {
            // Get updated contract to return new end timestamp
            match db.get_contract(&contract_id).await {
                Ok(Some(contract)) => {
                    let new_end_timestamp_ns = contract.end_timestamp_ns.unwrap_or(0);
                    Ok(Json(ApiResponse::success(ExtendContractResponse {
                        extension_payment_e9s,
                        new_end_timestamp_ns,
                        message: format!("Contract extended by {} hours", req.extension_hours),
                    })))
                }
                _ => Ok(Json(ApiResponse::error(
                    "Contract extended but failed to retrieve updated details".to_string(),
                ))),
            }
        }
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn get_contract_extensions(
    db: Data<&Arc<Database>>,
    Path(contract_id_hex): Path<String>,
) -> PoemResult<Json<ApiResponse<Vec<crate::database::contracts::ContractExtension>>>> {
    let contract_id = match hex::decode(&contract_id_hex) {
        Ok(id) => id,
        Err(_) => {
            return Ok(Json(ApiResponse::error(
                "Invalid contract ID format".to_string(),
            )))
        }
    };

    match db.get_contract_extensions(&contract_id).await {
        Ok(extensions) => Ok(Json(ApiResponse::success(extensions))),
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

// ============ User Endpoints ============

#[handler]
pub async fn get_user_profile(
    db: Data<&Arc<Database>>,
    Path(pubkey_hex): Path<String>,
) -> PoemResult<Json<ApiResponse<crate::database::users::UserProfile>>> {
    let pubkey = match hex::decode(&pubkey_hex) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(Json(ApiResponse::error(
                "Invalid pubkey format".to_string(),
            )))
        }
    };

    match db.get_user_profile(&pubkey).await {
        Ok(Some(profile)) => Ok(Json(ApiResponse::success(profile))),
        Ok(None) => Ok(Json(ApiResponse::error(
            "User profile not found".to_string(),
        ))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn get_user_contacts(
    db: Data<&Arc<Database>>,
    Path(pubkey_hex): Path<String>,
) -> PoemResult<Json<ApiResponse<Vec<crate::database::users::UserContact>>>> {
    let pubkey = match hex::decode(&pubkey_hex) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(Json(ApiResponse::error(
                "Invalid pubkey format".to_string(),
            )))
        }
    };

    match db.get_user_contacts(&pubkey).await {
        Ok(contacts) => Ok(Json(ApiResponse::success(contacts))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn get_user_socials(
    db: Data<&Arc<Database>>,
    Path(pubkey_hex): Path<String>,
) -> PoemResult<Json<ApiResponse<Vec<crate::database::users::UserSocial>>>> {
    let pubkey = match hex::decode(&pubkey_hex) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(Json(ApiResponse::error(
                "Invalid pubkey format".to_string(),
            )))
        }
    };

    match db.get_user_socials(&pubkey).await {
        Ok(socials) => Ok(Json(ApiResponse::success(socials))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn get_user_public_keys(
    db: Data<&Arc<Database>>,
    Path(pubkey_hex): Path<String>,
) -> PoemResult<Json<ApiResponse<Vec<crate::database::users::UserPublicKey>>>> {
    let pubkey = match hex::decode(&pubkey_hex) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(Json(ApiResponse::error(
                "Invalid pubkey format".to_string(),
            )))
        }
    };

    match db.get_user_public_keys(&pubkey).await {
        Ok(keys) => Ok(Json(ApiResponse::success(keys))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn get_user_activity(
    db: Data<&Arc<Database>>,
    Path(pubkey_hex): Path<String>,
) -> PoemResult<Json<ApiResponse<crate::database::users::UserActivity>>> {
    let pubkey = match hex::decode(&pubkey_hex) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(Json(ApiResponse::error(
                "Invalid pubkey format".to_string(),
            )))
        }
    };

    match db.get_user_activity(&pubkey).await {
        Ok(activity) => Ok(Json(ApiResponse::success(activity))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

// ============ Authenticated User Update Endpoints ============

/// Verify that the authenticated user owns the target resource
fn verify_user_authorization(auth: &AuthenticatedUser, pubkey_hex: &str) -> PoemResult<()> {
    let target_pubkey = hex::decode(pubkey_hex)
        .map_err(|_| poem::Error::from_string("Invalid pubkey format", StatusCode::BAD_REQUEST))?;

    if auth.pubkey_hash != target_pubkey {
        return Err(poem::Error::from_string(
            "Unauthorized: cannot modify another user's resource",
            StatusCode::FORBIDDEN,
        ));
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserProfileRequest {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
}

#[handler]
pub async fn update_user_profile(
    auth: AuthenticatedUser,
    db: Data<&Arc<Database>>,
    Path(pubkey_hex): Path<String>,
    Json(req): Json<UpdateUserProfileRequest>,
) -> PoemResult<Json<ApiResponse<String>>> {
    verify_user_authorization(&auth, &pubkey_hex)?;

    match db
        .upsert_user_profile(
            &auth.pubkey_hash,
            req.display_name.as_deref(),
            req.bio.as_deref(),
            req.avatar_url.as_deref(),
        )
        .await
    {
        Ok(_) => Ok(Json(ApiResponse::success(
            "Profile updated successfully".to_string(),
        ))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpsertContactRequest {
    pub contact_type: String,
    pub contact_value: String,
    #[serde(default)]
    pub verified: bool,
}

#[handler]
pub async fn upsert_user_contact(
    auth: AuthenticatedUser,
    db: Data<&Arc<Database>>,
    Path(pubkey_hex): Path<String>,
    Json(req): Json<UpsertContactRequest>,
) -> PoemResult<Json<ApiResponse<String>>> {
    verify_user_authorization(&auth, &pubkey_hex)?;

    if let Err(e) = crate::validation::validate_contact_type(&req.contact_type) {
        return Ok(Json(ApiResponse::error(e.to_string())));
    }

    if let Err(e) = crate::validation::validate_contact_value(&req.contact_type, &req.contact_value)
    {
        return Ok(Json(ApiResponse::error(e.to_string())));
    }

    match db
        .upsert_user_contact(
            &auth.pubkey_hash,
            &req.contact_type,
            &req.contact_value,
            req.verified,
        )
        .await
    {
        Ok(_) => Ok(Json(ApiResponse::success(
            "Contact added successfully".to_string(),
        ))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn delete_user_contact(
    auth: AuthenticatedUser,
    db: Data<&Arc<Database>>,
    Path((pubkey_hex, contact_id)): Path<(String, i64)>,
) -> PoemResult<Json<ApiResponse<String>>> {
    verify_user_authorization(&auth, &pubkey_hex)?;

    match db.delete_user_contact(&auth.pubkey_hash, contact_id).await {
        Ok(_) => Ok(Json(ApiResponse::success(
            "Contact deleted successfully".to_string(),
        ))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpsertSocialRequest {
    pub platform: String,
    pub username: String,
    pub profile_url: Option<String>,
}

#[handler]
pub async fn upsert_user_social(
    auth: AuthenticatedUser,
    db: Data<&Arc<Database>>,
    Path(pubkey_hex): Path<String>,
    Json(req): Json<UpsertSocialRequest>,
) -> PoemResult<Json<ApiResponse<String>>> {
    verify_user_authorization(&auth, &pubkey_hex)?;

    if let Err(e) = crate::validation::validate_social_platform(&req.platform) {
        return Ok(Json(ApiResponse::error(e.to_string())));
    }

    if let Err(e) = crate::validation::validate_social_username(&req.username) {
        return Ok(Json(ApiResponse::error(e.to_string())));
    }

    if let Some(ref url) = req.profile_url {
        if let Err(e) = crate::validation::validate_url(url) {
            return Ok(Json(ApiResponse::error(e.to_string())));
        }
    }

    match db
        .upsert_user_social(
            &auth.pubkey_hash,
            &req.platform,
            &req.username,
            req.profile_url.as_deref(),
        )
        .await
    {
        Ok(_) => Ok(Json(ApiResponse::success(
            "Social account added successfully".to_string(),
        ))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn delete_user_social(
    auth: AuthenticatedUser,
    db: Data<&Arc<Database>>,
    Path((pubkey_hex, social_id)): Path<(String, i64)>,
) -> PoemResult<Json<ApiResponse<String>>> {
    verify_user_authorization(&auth, &pubkey_hex)?;

    match db.delete_user_social(&auth.pubkey_hash, social_id).await {
        Ok(_) => Ok(Json(ApiResponse::success(
            "Social account deleted successfully".to_string(),
        ))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[derive(Debug, Deserialize)]
pub struct AddPublicKeyRequest {
    pub key_type: String,
    pub key_data: String,
    pub key_fingerprint: Option<String>,
    pub label: Option<String>,
}

#[handler]
pub async fn add_user_public_key(
    auth: AuthenticatedUser,
    db: Data<&Arc<Database>>,
    Path(pubkey_hex): Path<String>,
    Json(req): Json<AddPublicKeyRequest>,
) -> PoemResult<Json<ApiResponse<String>>> {
    verify_user_authorization(&auth, &pubkey_hex)?;

    if let Err(e) = crate::validation::validate_public_key(&req.key_type, &req.key_data) {
        return Ok(Json(ApiResponse::error(e.to_string())));
    }

    match db
        .add_user_public_key(
            &auth.pubkey_hash,
            &req.key_type,
            &req.key_data,
            req.key_fingerprint.as_deref(),
            req.label.as_deref(),
        )
        .await
    {
        Ok(_) => Ok(Json(ApiResponse::success(
            "Public key added successfully".to_string(),
        ))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn delete_user_public_key(
    auth: AuthenticatedUser,
    db: Data<&Arc<Database>>,
    Path((pubkey_hex, key_id)): Path<(String, i64)>,
) -> PoemResult<Json<ApiResponse<String>>> {
    verify_user_authorization(&auth, &pubkey_hex)?;

    match db.delete_user_public_key(&auth.pubkey_hash, key_id).await {
        Ok(_) => Ok(Json(ApiResponse::success(
            "Public key deleted successfully".to_string(),
        ))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

// ============ Stats Endpoints ============

#[handler]
pub async fn get_platform_stats(
    db: Data<&Arc<Database>>,
    metadata_cache: Data<&Arc<MetadataCache>>,
) -> PoemResult<Json<ApiResponse<PlatformOverview>>> {
    let base_stats = match db.get_platform_stats().await {
        Ok(stats) => stats,
        Err(e) => return Ok(Json(ApiResponse::error(e.to_string()))),
    };

    // Count providers who checked in within last 24 hours
    let cutoff_24h =
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) - 24 * 3600 * 1_000_000_000;
    let validator_count: (i64,) = match sqlx::query_as(
        "SELECT COUNT(DISTINCT pubkey_hash) FROM provider_check_ins WHERE block_timestamp_ns > ?",
    )
    .bind(cutoff_24h)
    .fetch_one(&db.pool)
    .await
    {
        Ok(count) => count,
        Err(e) => return Ok(Json(ApiResponse::error(e.to_string()))),
    };

    // Get latest block timestamp from database
    let latest_block_timestamp_ns = match db.get_latest_block_timestamp_ns().await {
        Ok(Some(ts)) if ts > 0 => Some(ts as u64),
        _ => None,
    };

    // Get all metadata from cache as JSON (fetched periodically from canister)
    let metadata_map = match metadata_cache.get() {
        Ok(m) => {
            tracing::debug!("Metadata cache has {} entries", m.data.len());
            m.to_json_map()
        }
        Err(e) => {
            tracing::warn!("Failed to get metadata from cache: {}", e);
            BTreeMap::new()
        }
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

    Ok(Json(ApiResponse::success(response)))
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

#[cfg(test)]
mod tests {
    use super::*;
    use ts_rs::TS;

    #[test]
    fn export_typescript_types() {
        PlatformOverview::export().expect("Failed to export PlatformOverview type");
    }

    #[test]
    fn normalize_provisioning_details_trims_value() {
        let result = normalize_provisioning_details(
            "provisioning",
            Some("   ip:1.2.3.4\nuser:root   ".to_string()),
        )
        .unwrap();
        assert_eq!(result.as_deref(), Some("ip:1.2.3.4\nuser:root"));
    }

    #[test]
    fn normalize_provisioning_details_requires_data_when_provisioned() {
        assert!(normalize_provisioning_details("provisioned", None).is_err());
        assert!(normalize_provisioning_details("provisioned", Some("   ".to_string())).is_err());
    }
}
