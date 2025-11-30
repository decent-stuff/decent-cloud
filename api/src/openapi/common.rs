use crate::auth::ApiAuthenticatedUser;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub success: bool,
    pub message: String,
    pub environment: String,
}

#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
#[oai(skip_serializing_if_is_none)]
pub struct ApiResponse<T: poem_openapi::types::ParseFromJSON + poem_openapi::types::ToJSON> {
    pub success: bool,
    #[oai(skip_serializing_if_is_none)]
    pub data: Option<T>,
    #[oai(skip_serializing_if_is_none)]
    pub error: Option<String>,
}

pub fn default_limit() -> i64 {
    50
}

pub fn default_false() -> bool {
    false
}

/// Decode a hex-encoded public key with detailed error messages
pub fn decode_pubkey(pubkey_hex: &str) -> Result<Vec<u8>, String> {
    let bytes = hex::decode(pubkey_hex)
        .map_err(|e| format!("Invalid pubkey hex: {} (value: {})", e, pubkey_hex))?;
    if bytes.len() != 32 {
        return Err(format!(
            "Public key must be 32 bytes, got {} bytes (value: {})",
            bytes.len(),
            pubkey_hex
        ));
    }
    Ok(bytes)
}

pub fn check_authorization(pubkey: &[u8], user: &ApiAuthenticatedUser) -> Result<(), String> {
    if pubkey != user.pubkey {
        Err(format!(
            "Unauthorized: request pubkey {} does not match authenticated user {}",
            hex::encode(pubkey),
            hex::encode(&user.pubkey)
        ))
    } else {
        Ok(())
    }
}

// Request types for offerings
#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct DuplicateOfferingRequest {
    pub new_offering_id: String,
}

#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct BulkUpdateStatusRequest {
    pub offering_ids: Vec<i64>,
    pub stock_status: String,
}

#[derive(Debug, Serialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct CsvImportResult {
    pub success_count: usize,
    pub errors: Vec<CsvImportError>,
}

#[derive(Debug, Serialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct CsvImportError {
    pub row: usize,
    pub message: String,
}

// Request types for contracts
#[derive(Debug, Serialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
#[oai(skip_serializing_if_is_none)]
pub struct RentalRequestResponse {
    pub contract_id: String,
    pub message: String,
    #[oai(skip_serializing_if_is_none)]
    pub client_secret: Option<String>,
}

#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct RentalResponseRequest {
    pub accept: bool,
    pub memo: Option<String>,
}

#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ProvisioningStatusRequest {
    pub status: String,
    pub instance_details: Option<String>,
}

#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ExtendContractRequest {
    pub extension_hours: i64,
    pub memo: Option<String>,
}

#[derive(Debug, Serialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ExtendContractResponse {
    pub extension_payment_e9s: i64,
    pub new_end_timestamp_ns: i64,
    pub message: String,
}

#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct CancelContractRequest {
    pub memo: Option<String>,
}

// Request types for accounts
#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct RegisterAccountRequest {
    pub username: String,
    pub public_key: String,
    pub email: String,
}

#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AddAccountKeyRequest {
    pub new_public_key: String,
}

#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct UpdateDeviceNameRequest {
    pub device_name: Option<String>,
}

// Request types for account profile
#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct UpdateAccountProfileRequest {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AddAccountContactRequest {
    pub contact_type: String,
    pub contact_value: String,
    #[oai(default = "default_false")]
    pub verified: bool,
}

#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AddAccountSocialRequest {
    pub platform: String,
    pub username: String,
    pub profile_url: Option<String>,
}

#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AddAccountExternalKeyRequest {
    pub key_type: String,
    pub key_data: String,
    pub key_fingerprint: Option<String>,
    pub label: Option<String>,
}

// Request types for account recovery
#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct RequestRecoveryRequest {
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct CompleteRecoveryRequest {
    pub token: String,
    pub public_key: String,
}

// Request types for admin
#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AdminDisableKeyRequest {
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AdminAddRecoveryKeyRequest {
    pub public_key: String,
    pub reason: String,
}

#[derive(poem_openapi::Tags)]
pub enum ApiTags {
    /// System endpoints
    System,
    /// Account management endpoints
    Accounts,
    /// Admin operations endpoints
    Admin,
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
