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
}

#[derive(Debug, Serialize)]
pub struct PlatformOverview {
    // Database-derived statistics (always available, reliable)
    pub total_providers: i64,
    pub active_providers: i64,
    pub total_offerings: i64,
    pub total_contracts: i64,
    pub total_transfers: i64,
    pub total_volume_e9s: i64,
    pub validator_count_24h: i64,
    pub latest_block_timestamp_ns: Option<u64>,

    // All canister metadata (flexible, future-proof)
    // Includes: num_blocks, blocks_until_next_halving, current_block_validators,
    // current_block_rewards_e9s, reward_per_block_e9s, token_value_in_usd_e6,
    // latest_block_hash, and any future metadata fields
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
        .search_offerings(crate::database::offerings::SearchOfferingsParams {
            product_type: params.product_type.as_deref(),
            country: params.country.as_deref(),
            min_price_e9s: params.min_price_e9s,
            max_price_e9s: params.max_price_e9s,
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
    Json(params): Json<crate::database::offerings::CreateOfferingParams>,
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

    match db.create_offering(&pubkey, params).await {
        Ok(offering_id) => Ok(Json(ApiResponse::success(offering_id))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn update_provider_offering(
    db: Data<&Arc<Database>>,
    user: AuthenticatedUser,
    Path((pubkey_hex, offering_id)): Path<(String, i64)>,
    Json(params): Json<crate::database::offerings::CreateOfferingParams>,
) -> PoemResult<Json<ApiResponse<()>>> {
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

    match db.update_offering(&pubkey, offering_id, params).await {
        Ok(()) => Ok(Json(ApiResponse::success(()))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
}

#[handler]
pub async fn delete_provider_offering(
    db: Data<&Arc<Database>>,
    user: AuthenticatedUser,
    Path((pubkey_hex, offering_id)): Path<(String, i64)>,
) -> PoemResult<Json<ApiResponse<()>>> {
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

    match db.delete_offering(&pubkey, offering_id).await {
        Ok(()) => Ok(Json(ApiResponse::success(()))),
        Err(e) => Ok(Json(ApiResponse::error(e.to_string()))),
    }
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
