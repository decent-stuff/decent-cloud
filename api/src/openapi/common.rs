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
pub struct EmptyResponse {}

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

/// Request to add a pubkey to an offering's visibility allowlist
#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AllowlistAddRequest {
    /// Hex-encoded public key of the user to allow
    pub allowed_pubkey: String,
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
pub struct UpdatePasswordRequest {
    pub new_password: String,
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

/// Request to record a usage event for a contract
#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct RecordUsageRequest {
    /// Type of event: "heartbeat", "session_start", "session_end"
    pub event_type: String,
    /// Optional units delta (e.g., hours of usage)
    pub units_delta: Option<f64>,
    /// Optional timestamp override (Unix seconds)
    pub heartbeat_at: Option<i64>,
    /// Optional source identifier (e.g., agent ID)
    pub source: Option<String>,
    /// Optional metadata JSON
    pub metadata: Option<String>,
}

/// Request to record a health check result for a contract
#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct RecordHealthCheckRequest {
    /// Timestamp when the check was performed (nanoseconds since epoch)
    pub checked_at: i64,
    /// Health status: "healthy", "unhealthy", or "unknown"
    pub status: String,
    /// Optional latency measurement in milliseconds
    pub latency_ms: Option<i32>,
    /// Optional JSON with additional diagnostic details
    pub details: Option<String>,
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

#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AdminSetAccountEmailRequest {
    /// New email address, or null to clear
    pub email: Option<String>,
}

/// Summary of resources deleted when deleting an account
#[derive(Debug, Serialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AdminAccountDeletionSummary {
    pub offerings_deleted: i64,
    pub contracts_as_requester: i64,
    pub contracts_as_provider: i64,
    pub public_keys_deleted: i64,
    pub provider_profile_deleted: bool,
}

#[derive(Debug, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AdminSetAdminStatusRequest {
    pub is_admin: bool,
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

/// Request to update auto-accept rentals setting
#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AutoAcceptRequest {
    /// Whether to auto-accept new rental contracts
    pub auto_accept_rentals: bool,
}

/// Response with auto-accept rentals setting
#[derive(Debug, Serialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AutoAcceptResponse {
    /// Whether auto-accept rentals is enabled
    pub auto_accept_rentals: bool,
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

// VM Reconciliation types for dc-agent

/// Running instance reported by dc-agent
#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ReconcileRunningInstance {
    /// External ID of the VM (e.g., Proxmox VMID)
    pub external_id: String,
    /// Contract ID this VM is associated with (extracted from VM name/tags)
    pub contract_id: Option<String>,
}

/// Request body for reconciliation
#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ReconcileRequest {
    /// List of VMs currently running on this agent
    pub running_instances: Vec<ReconcileRunningInstance>,
}

// Re-export shared types from dcc-common
pub use dcc_common::api_types::{
    LockResponse, ReconcileKeepInstance, ReconcileResponse, ReconcileTerminateInstance,
    ReconcileUnknownInstance,
};

// Agent Pool request/response types

/// Request to create an agent pool
#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct CreatePoolRequest {
    /// Human-readable name (e.g., "eu-proxmox")
    pub name: String,
    /// Location/region identifier (e.g., "europe", "na", "apac")
    pub location: String,
    /// Provisioner type (e.g., "proxmox", "script", "manual")
    pub provisioner_type: String,
}

/// Request to update an agent pool
#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct UpdatePoolRequest {
    pub name: Option<String>,
    pub location: Option<String>,
    pub provisioner_type: Option<String>,
}

/// Request to create a setup token for agent registration
#[derive(Debug, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct CreateSetupTokenRequest {
    /// Optional label for the agent using this token
    pub label: Option<String>,
    /// Token expiry in hours (default: 24)
    #[oai(default)]
    pub expires_in_hours: Option<u32>,
}

/// Response for offering suggestions based on pool capabilities
#[derive(Debug, Serialize, poem_openapi::Object, ts_rs::TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct OfferingSuggestionsResponse {
    /// Aggregated pool capabilities
    pub pool_capabilities: crate::database::agent_pools::PoolCapabilities,
    /// Suggested offerings based on capabilities
    pub suggested_offerings: Vec<crate::database::offerings::OfferingSuggestion>,
    /// Tiers that are unavailable due to insufficient resources
    pub unavailable_tiers: Vec<crate::database::offerings::UnavailableTier>,
}

/// Pricing configuration for a single tier
#[derive(Debug, Clone, Deserialize, poem_openapi::Object, ts_rs::TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct TierPricing {
    /// Monthly price for this tier
    pub monthly_price: f64,
    /// Currency code (e.g., "USD", "EUR")
    pub currency: String,
}

/// Request to generate offerings from pool capabilities
#[derive(Debug, Deserialize, poem_openapi::Object, ts_rs::TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct GenerateOfferingsRequest {
    /// Specific tier names to generate (if empty, generates all applicable tiers)
    #[serde(default)]
    pub tiers: Vec<String>,
    /// Pricing for each tier (key = tier name, e.g., "small", "medium")
    pub pricing: std::collections::HashMap<String, TierPricing>,
    /// Visibility for generated offerings (default: "public")
    #[serde(default = "default_visibility")]
    pub visibility: String,
    /// If true, only preview what would be created without actually creating
    #[serde(default)]
    pub dry_run: bool,
}

fn default_visibility() -> String {
    "public".to_string()
}

/// Response from offering generation
#[derive(Debug, Serialize, poem_openapi::Object, ts_rs::TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct GenerateOfferingsResponse {
    /// Offerings that were created (or would be created in dry_run mode)
    pub created_offerings: Vec<crate::database::offerings::Offering>,
    /// Tiers that were skipped (no pricing provided or other reasons)
    pub skipped_tiers: Vec<crate::database::offerings::UnavailableTier>,
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
    /// Agent pool operations endpoints
    Pools,
    /// Subscription management endpoints
    Subscriptions,
    /// Cloud self-provisioning endpoints
    Cloud,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_pubkey_valid_32_bytes() {
        let result = decode_pubkey(&"a".repeat(64));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 32);
    }

    #[test]
    fn test_decode_pubkey_invalid_hex() {
        let result = decode_pubkey("not-valid-hex!");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Invalid pubkey hex"), "Got: {err}");
    }

    #[test]
    fn test_decode_pubkey_wrong_length() {
        let result = decode_pubkey("abcd"); // 2 bytes, not 32
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("must be 32 bytes"), "Got: {err}");
    }

    #[test]
    fn test_check_authorization_matching() {
        let pubkey = vec![1u8; 32];
        let user = ApiAuthenticatedUser {
            pubkey: pubkey.clone(),
        };
        assert!(check_authorization(&pubkey, &user).is_ok());
    }

    #[test]
    fn test_check_authorization_mismatch() {
        let pubkey = vec![1u8; 32];
        let user = ApiAuthenticatedUser {
            pubkey: vec![2u8; 32],
        };
        let result = check_authorization(&pubkey, &user);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Unauthorized"), "Got: {err}");
    }

    #[test]
    fn test_default_limit_returns_50() {
        assert_eq!(default_limit(), 50);
    }

    #[test]
    fn test_default_false_returns_false() {
        assert!(!default_false());
    }

    #[test]
    fn test_api_response_success_serialization() {
        let resp = ApiResponse::<String> {
            success: true,
            data: Some("hello".to_string()),
            error: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"], "hello");
    }

    #[test]
    fn test_api_response_error_serialization() {
        let resp = ApiResponse::<String> {
            success: false,
            data: None,
            error: Some("something failed".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], false);
        assert_eq!(json["error"], "something failed");
    }

    #[test]
    fn test_health_response_camel_case() {
        let resp = HealthResponse {
            success: true,
            message: "ok".to_string(),
            environment: "test".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"success\""));
        assert!(json.contains("\"message\""));
        assert!(json.contains("\"environment\""));
    }

    #[test]
    fn test_register_account_request_deserialization() {
        let json = r#"{"username":"alice","publicKey":"abc123","email":"a@b.com"}"#;
        let req: RegisterAccountRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.username, "alice");
        assert_eq!(req.public_key, "abc123");
        assert_eq!(req.email, "a@b.com");
    }

    #[test]
    fn test_record_health_check_request_deserialization() {
        let json = r#"{"checkedAt":1234567890,"status":"healthy","latencyMs":42,"details":null}"#;
        let req: RecordHealthCheckRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.checked_at, 1234567890);
        assert_eq!(req.status, "healthy");
        assert_eq!(req.latency_ms, Some(42));
        assert!(req.details.is_none());
    }

    #[test]
    fn test_record_health_check_request_minimal() {
        let json = r#"{"checkedAt":0,"status":"unknown"}"#;
        let req: RecordHealthCheckRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.checked_at, 0);
        assert_eq!(req.status, "unknown");
        assert!(req.latency_ms.is_none());
    }

    #[test]
    fn test_rental_response_request_accept() {
        let json = r#"{"accept":true,"memo":"looks good"}"#;
        let req: RentalResponseRequest = serde_json::from_str(json).unwrap();
        assert!(req.accept);
        assert_eq!(req.memo.unwrap(), "looks good");
    }

    #[test]
    fn test_rental_response_request_reject() {
        let json = r#"{"accept":false,"memo":null}"#;
        let req: RentalResponseRequest = serde_json::from_str(json).unwrap();
        assert!(!req.accept);
        assert!(req.memo.is_none());
    }

    #[test]
    fn test_extend_contract_response_serialization() {
        let resp = ExtendContractResponse {
            extension_payment_e9s: 1_000_000_000,
            new_end_timestamp_ns: 1700000000000000000,
            message: "Extended".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("extensionPaymentE9s"));
        assert!(json.contains("newEndTimestampNs"));
    }

    #[test]
    fn test_generate_offerings_request_defaults() {
        let json = r#"{"pricing":{"small":{"monthlyPrice":5.0,"currency":"USD"}}}"#;
        let req: GenerateOfferingsRequest = serde_json::from_str(json).unwrap();
        assert!(req.tiers.is_empty());
        assert_eq!(req.visibility, "public");
        assert!(!req.dry_run);
        assert_eq!(req.pricing["small"].monthly_price, 5.0);
    }

    #[test]
    fn test_reconcile_request_deserialization() {
        let json = r#"{"runningInstances":[{"externalId":"vm-100","contractId":"c-123"}]}"#;
        let req: ReconcileRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.running_instances.len(), 1);
        assert_eq!(req.running_instances[0].external_id, "vm-100");
        assert_eq!(
            req.running_instances[0].contract_id.as_deref(),
            Some("c-123")
        );
    }
}
