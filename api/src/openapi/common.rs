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

/// Response time distribution across time buckets
#[derive(Debug, Serialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ResponseTimeDistributionResponse {
    /// Percentage of inquiries responded to within 1 hour
    pub within_1h_pct: f64,
    /// Percentage of inquiries responded to within 4 hours
    pub within_4h_pct: f64,
    /// Percentage of inquiries responded to within 12 hours
    pub within_12h_pct: f64,
    /// Percentage of inquiries responded to within 24 hours
    pub within_24h_pct: f64,
    /// Percentage of inquiries responded to within 72 hours
    pub within_72h_pct: f64,
    /// Total number of responses measured
    pub total_responses: i64,
}

/// Response metrics for provider support response times
#[derive(Debug, Serialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ResponseMetricsResponse {
    /// Average response time in seconds (None if no data)
    pub avg_response_seconds: Option<f64>,
    /// Average response time in hours (None if no data)
    pub avg_response_hours: Option<f64>,
    /// SLA compliance percentage (0-100)
    pub sla_compliance_percent: f64,
    /// Number of SLA breaches in last 30 days
    pub breach_count_30d: i64,
    /// Total customer inquiries in last 30 days
    pub total_inquiries_30d: i64,
    /// Response time distribution across time buckets
    pub distribution: ResponseTimeDistributionResponse,
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
    pub checkout_url: Option<String>,
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

#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct UpdateIcpayTransactionRequest {
    pub transaction_id: String,
}

#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct VerifyCheckoutSessionRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct VerifyCheckoutSessionResponse {
    pub contract_id: String,
    pub payment_status: String,
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
pub struct UpdateAccountEmailRequest {
    pub email: String,
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

#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct VerifyEmailRequest {
    pub token: String,
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
pub struct AdminSendTestEmailRequest {
    pub to_email: String,
}

#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AdminSetEmailVerifiedRequest {
    pub verified: bool,
}

#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AdminAddRecoveryKeyRequest {
    pub public_key: String,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AdminProcessPayoutRequest {
    pub provider_pubkey: String,
    pub wallet_address: String,
}

// Request/Response types for user notification config
#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
#[oai(skip_serializing_if_is_none)]
pub struct NotificationConfigResponse {
    pub notify_telegram: bool,
    pub notify_email: bool,
    pub notify_sms: bool,
    #[oai(skip_serializing_if_is_none)]
    pub telegram_chat_id: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub notify_phone: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub notify_email_address: Option<String>,
}

#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct UpdateNotificationConfigRequest {
    pub notify_telegram: bool,
    pub notify_email: bool,
    pub notify_sms: bool,
    pub telegram_chat_id: Option<String>,
    pub notify_phone: Option<String>,
    pub notify_email_address: Option<String>,
}

#[derive(Debug, Serialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct NotificationUsageResponse {
    pub telegram_count: i64,
    pub sms_count: i64,
    pub email_count: i64,
    pub telegram_limit: i64,
    pub sms_limit: i64,
}

/// Request to test a notification channel
#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct TestNotificationRequest {
    /// Channel to test: "telegram", "email", or "sms"
    pub channel: String,
}

/// Response from notification test
#[derive(Debug, Serialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct TestNotificationResponse {
    /// Whether the test was sent successfully
    pub sent: bool,
    /// Details about what was sent or why it failed
    pub message: String,
}

/// Response from updating provider onboarding
#[derive(Debug, Serialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct OnboardingUpdateResponse {
    /// Timestamp when onboarding was completed
    #[serde(rename = "onboarding_completed_at")]
    #[oai(rename = "onboarding_completed_at")]
    pub onboarding_completed_at: i64,
}

/// Response from syncing provider help center
#[derive(Debug, Serialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct HelpcenterSyncResponse {
    /// URL to view the article in the help center
    pub article_url: String,
    /// Action performed: "created" or "updated"
    pub action: String,
}

// Request types for reseller operations
#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct CreateResellerRelationshipRequest {
    pub external_provider_pubkey: String,
    pub commission_percent: i64,
}

#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct UpdateResellerRelationshipRequest {
    pub commission_percent: Option<i64>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct FulfillResellerOrderRequest {
    pub external_order_id: String,
    pub external_order_details: Option<String>,
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
    /// Chatwoot integration endpoints
    Chatwoot,
    /// Reseller operations endpoints
    Resellers,
}
