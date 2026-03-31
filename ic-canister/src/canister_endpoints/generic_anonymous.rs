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
/// Returns ledger entries from committed blocks with timestamp >= from_timestamp_ns.
/// If from_timestamp_ns is None, returns from the beginning.
/// Limit caps the number of entries returned (default 1000).
#[ic_cdk::query]
fn cf_sync_data(from_timestamp_ns: Option<u64>, limit: Option<u32>) -> Vec<SyncDataEntry> {
    use crate::canister_backend::generic::LEDGER_MAP;
    use ledger_map::ledger_entry::Operation;

    let from_ts = from_timestamp_ns.unwrap_or(0);
    let max_entries = limit.unwrap_or(1000) as usize;

    LEDGER_MAP.with(|ledger| {
        let ledger_ref = ledger.borrow();
        let mut results = Vec::new();

        if ledger_ref.get_blocks_count() == 0 {
            return results;
        }

        // Quick check: if tip timestamp is older than from_ts, nothing to return
        if from_ts > 0 && ledger_ref.get_latest_block_timestamp_ns() < from_ts {
            return results;
        }

        for block_result in ledger_ref.iter_raw(0) {
            match block_result {
                Ok((_block_header, ledger_block)) => {
                    let block_ts = ledger_block.timestamp();

                    // Skip blocks older than the watermark
                    if block_ts < from_ts {
                        continue;
                    }

                    let block_offset = ledger_block.get_offset();

                    for entry in ledger_block.entries() {
                        if results.len() >= max_entries {
                            return results;
                        }

                        let op_type = match entry.operation() {
                            Operation::Upsert => "UPSERT",
                            Operation::Delete => "DELETE",
                        };

                        results.push(SyncDataEntry {
                            label: entry.label().to_string(),
                            key: entry.key().to_vec(),
                            value: entry.value().to_vec(),
                            block_offset,
                            timestamp_ns: block_ts,
                            operation_type: op_type.to_string(),
                        });
                    }
                }
                Err(e) => {
                    ic_cdk::println!("cf_sync_data: block read error: {:#}", e);
                    break;
                }
            }
        }

        results
    })
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
