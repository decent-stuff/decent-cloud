use crate::canister_backend::generic::*;
use candid::Principal;

/// Anonymous version of provider_register - accepts caller info in parameters
#[ic_cdk::update]
fn provider_register_anonymous(
    pubkey_bytes: Vec<u8>,
    crypto_signature: Vec<u8>,
    caller_principal: Option<String>,
) -> Result<String, String> {
    // Set caller if provided (for compatibility)
    if let Some(principal_str) = caller_principal {
        if let Ok(principal) = principal_str.parse::<Principal>() {
            ic_cdk::println!("Setting caller principal: {}", principal);
        }
    }

    _provider_register(pubkey_bytes, crypto_signature)
}

/// Anonymous version of provider_check_in
#[ic_cdk::update]
fn provider_check_in_anonymous(
    pubkey_bytes: Vec<u8>,
    memo: String,
    nonce_crypto_signature: Vec<u8>,
    caller_principal: Option<String>,
) -> Result<String, String> {
    if let Some(principal_str) = caller_principal {
        if let Ok(principal) = principal_str.parse::<Principal>() {
            ic_cdk::println!("Setting caller principal: {}", principal);
        }
    }

    _provider_check_in(pubkey_bytes, memo, nonce_crypto_signature)
}

/// Anonymous version of contract_sign_request
#[ic_cdk::update]
fn contract_sign_request_anonymous(
    pubkey_bytes: Vec<u8>,
    request_serialized: Vec<u8>,
    crypto_signature: Vec<u8>,
    caller_principal: Option<String>,
) -> Result<String, String> {
    if let Some(principal_str) = caller_principal {
        if let Ok(principal) = principal_str.parse::<Principal>() {
            ic_cdk::println!("Setting caller principal: {}", principal);
        }
    }

    _contract_sign_request(pubkey_bytes, request_serialized, crypto_signature)
}

/// Anonymous version of contract_sign_reply
#[ic_cdk::update]
fn contract_sign_reply_anonymous(
    pubkey_bytes: Vec<u8>,
    reply_serialized: Vec<u8>,
    crypto_signature: Vec<u8>,
    caller_principal: Option<String>,
) -> Result<String, String> {
    if let Some(principal_str) = caller_principal {
        if let Ok(principal) = principal_str.parse::<Principal>() {
            ic_cdk::println!("Setting caller principal: {}", principal);
        }
    }

    _contract_sign_reply(pubkey_bytes, reply_serialized, crypto_signature)
}

/// Anonymous version of user_register
#[ic_cdk::update]
fn user_register_anonymous(
    pubkey_bytes: Vec<u8>,
    crypto_signature: Vec<u8>,
    caller_principal: Option<String>,
) -> Result<String, String> {
    if let Some(principal_str) = caller_principal {
        if let Ok(principal) = principal_str.parse::<Principal>() {
            ic_cdk::println!("Setting caller principal: {}", principal);
        }
    }

    _user_register(pubkey_bytes, crypto_signature)
}

/// Bulk operations for CF service - accept multiple operations in one call
#[ic_cdk::update]
fn bulk_update_from_cf(operations: Vec<BulkOperation>) -> Vec<BulkResult> {
    let mut results = Vec::new();

    for (index, op) in operations.into_iter().enumerate() {
        let result = match op.operation_type.as_str() {
            "provider_register" => _provider_register(op.pubkey_bytes, op.crypto_signature),
            "provider_check_in" => _provider_check_in(
                op.pubkey_bytes,
                op.memo.unwrap_or_default(),
                op.crypto_signature,
            ),
            "user_register" => _user_register(op.pubkey_bytes, op.crypto_signature),
            _ => Err(format!("Unknown operation type: {}", op.operation_type)),
        };

        results.push(BulkResult {
            index,
            operation_id: op.operation_id,
            result,
        });
    }

    results
}

/// Query to get data for CF synchronization
#[ic_cdk::query]
fn cf_sync_data(_from_timestamp_ns: Option<u64>, _limit: Option<u32>) -> Vec<SyncDataEntry> {
    // TODO: Implement this to return recent changes for CF sync
    // This would query the ledger for changes since from_timestamp_ns
    vec![]
}

// Types for bulk operations
#[derive(candid::CandidType, serde::Deserialize)]
pub struct BulkOperation {
    pub operation_id: String,
    pub operation_type: String,
    pub pubkey_bytes: Vec<u8>,
    pub crypto_signature: Vec<u8>,
    pub profile_serialized: Option<Vec<u8>>,
    pub offering_serialized: Option<Vec<u8>>,
    pub contract_serialized: Option<Vec<u8>>,
    pub reply_serialized: Option<Vec<u8>>,
    pub memo: Option<String>,
    pub timestamp_ns: u64,
}

#[derive(candid::CandidType, serde::Serialize)]
pub struct BulkResult {
    pub index: usize,
    pub operation_id: String,
    pub result: Result<String, String>,
}

#[derive(candid::CandidType, serde::Serialize)]
pub struct SyncDataEntry {
    pub label: String,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub block_offset: u64,
    pub timestamp_ns: u64,
    pub operation_type: String, // INSERT, UPDATE, DELETE
}
