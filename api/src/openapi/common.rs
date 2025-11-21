use crate::auth::ApiAuthenticatedUser;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

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

/// Decode a hex-encoded ID (contract, key, etc.) with detailed error messages
pub fn decode_hex_id(id_hex: &str, id_type: &str) -> Result<Vec<u8>, String> {
    hex::decode(id_hex).map_err(|e| format!("Invalid {} hex: {} (value: {})", id_type, e, id_hex))
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
pub struct DuplicateOfferingRequest {
    pub new_offering_id: String,
}

#[derive(Debug, Deserialize, Object)]
pub struct BulkUpdateStatusRequest {
    pub offering_ids: Vec<i64>,
    pub stock_status: String,
}

#[derive(Debug, Serialize, Object)]
pub struct CsvImportResult {
    pub success_count: usize,
    pub errors: Vec<CsvImportError>,
}

#[derive(Debug, Serialize, Object)]
pub struct CsvImportError {
    pub row: usize,
    pub message: String,
}

// Request types for contracts
#[derive(Debug, Serialize, Object)]
pub struct RentalRequestResponse {
    pub contract_id: String,
    pub message: String,
}

#[derive(Debug, Deserialize, Object)]
pub struct RentalResponseRequest {
    pub accept: bool,
    pub memo: Option<String>,
}

#[derive(Debug, Deserialize, Object)]
pub struct ProvisioningStatusRequest {
    pub status: String,
    pub instance_details: Option<String>,
}

#[derive(Debug, Deserialize, Object)]
pub struct ExtendContractRequest {
    pub extension_hours: i64,
    pub memo: Option<String>,
}

#[derive(Debug, Serialize, Object)]
pub struct ExtendContractResponse {
    pub extension_payment_e9s: i64,
    pub new_end_timestamp_ns: i64,
    pub message: String,
}

#[derive(Debug, Deserialize, Object)]
pub struct CancelContractRequest {
    pub memo: Option<String>,
}

// Request types for users
#[derive(Debug, Deserialize, Object)]
pub struct UpdateUserProfileRequest {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize, Object)]
pub struct UpsertContactRequest {
    pub contact_type: String,
    pub contact_value: String,
    #[serde(default)]
    pub verified: bool,
}

#[derive(Debug, Deserialize, Object)]
pub struct UpsertSocialRequest {
    pub platform: String,
    pub username: String,
    pub profile_url: Option<String>,
}

#[derive(Debug, Deserialize, Object)]
pub struct AddPublicKeyRequest {
    pub key_type: String,
    pub key_data: String,
    pub key_fingerprint: Option<String>,
    pub label: Option<String>,
}

// Request types for accounts
#[derive(Debug, Serialize, Deserialize, Object)]
pub struct RegisterAccountRequest {
    pub username: String,
    #[serde(rename = "publicKey")]
    pub public_key: String,
}

#[derive(Debug, Deserialize, Object)]
pub struct AddAccountKeyRequest {
    #[serde(rename = "newPublicKey")]
    pub new_public_key: String,
}

#[derive(Debug, Deserialize, Object)]
pub struct UpdateDeviceNameRequest {
    #[serde(rename = "deviceName")]
    pub device_name: Option<String>,
}

// Request types for admin
#[derive(Debug, Serialize, Deserialize, Object)]
pub struct AdminDisableKeyRequest {
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct AdminAddRecoveryKeyRequest {
    #[serde(rename = "publicKey")]
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
