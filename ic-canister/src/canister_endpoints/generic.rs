use crate::canister_backend::generic::*;
use candid::Principal;
use dcc_common::{
    ContractId, ContractReqSerialized, NextBlockSyncRequest, NextBlockSyncResponse, TokenAmountE9s,
};
#[allow(unused_imports)]
use ic_cdk::println;
use icrc_ledger_types::icrc::generic_metadata_value::MetadataValue;

#[ic_cdk::init]
fn init(enable_test_config: Option<bool>) {
    _init(enable_test_config)
}

#[ic_cdk::pre_upgrade]
fn pre_upgrade() {
    _pre_upgrade()
}

#[ic_cdk::post_upgrade]
fn post_upgrade(enable_test_config: Option<bool>) {
    _post_upgrade(enable_test_config)
}

#[ic_cdk::query]
fn get_registration_fee() -> TokenAmountE9s {
    _get_registration_fee()
}

#[ic_cdk::update]
fn provider_register(pubkey_bytes: Vec<u8>, crypto_signature: Vec<u8>) -> Result<String, String> {
    _provider_register(pubkey_bytes, crypto_signature)
}

#[ic_cdk::update]
fn user_register(pubkey_bytes: Vec<u8>, crypto_signature: Vec<u8>) -> Result<String, String> {
    _user_register(pubkey_bytes, crypto_signature)
}

#[ic_cdk::update]
fn provider_check_in(
    pubkey_bytes: Vec<u8>,
    memo: String,
    nonce_crypto_signature: Vec<u8>,
) -> Result<String, String> {
    _provider_check_in(pubkey_bytes, memo, nonce_crypto_signature)
}

#[ic_cdk::update]
fn provider_update_profile(
    pubkey_bytes: Vec<u8>,
    profile_serialized: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    _provider_update_profile(pubkey_bytes, profile_serialized, crypto_signature)
}

#[ic_cdk::update]
fn provider_update_offering(
    pubkey_bytes: Vec<u8>,
    offering_serialized: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    _provider_update_offering(pubkey_bytes, offering_serialized, crypto_signature)
}

#[ic_cdk::query]
fn offering_search(search_query: String) -> Vec<(Vec<u8>, Vec<u8>)> {
    _offering_search(search_query)
}

#[ic_cdk::update]
fn contract_sign_request(
    pubkey_bytes: Vec<u8>,
    request_serialized: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    _contract_sign_request(pubkey_bytes, request_serialized, crypto_signature)
}

#[ic_cdk::query]
fn contracts_list_pending(
    pubkey_bytes: Option<Vec<u8>>,
) -> Vec<(ContractId, ContractReqSerialized)> {
    _contracts_list_pending(pubkey_bytes)
}

#[ic_cdk::update]
fn contract_sign_reply(
    pubkey_bytes: Vec<u8>,
    reply_serialized: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    _contract_sign_reply(pubkey_bytes, reply_serialized, crypto_signature)
}

#[ic_cdk::query]
fn provider_get_profile_by_pubkey_bytes(pubkey_bytes: Vec<u8>) -> Option<String> {
    _provider_get_profile_by_pubkey_bytes(pubkey_bytes)
}

#[ic_cdk::query]
fn provider_get_profile_by_principal(principal: Principal) -> Option<String> {
    _provider_get_profile_by_principal(principal)
}

#[ic_cdk::query]
fn get_check_in_nonce() -> Vec<u8> {
    _get_check_in_nonce()
}

#[ic_cdk::query]
fn get_identity_reputation(pubkey_bytes: Vec<u8>) -> u64 {
    _get_identity_reputation(pubkey_bytes)
}

#[ic_cdk::query]
fn provider_list_checked_in() -> Result<Vec<String>, String> {
    _provider_list_checked_in()
}

#[ic_cdk::query]
fn data_fetch(
    cursor: Option<String>,
    bytes_before: Option<Vec<u8>>,
) -> Result<(String, Vec<u8>), String> {
    _data_fetch(cursor, bytes_before)
}

#[ic_cdk::update]
fn data_push_auth() -> Result<String, String> {
    _data_push_auth()
}

#[ic_cdk::update]
fn data_push(cursor: String, data: Vec<u8>) -> Result<String, String> {
    _data_push(cursor, data)
}

#[ic_cdk::query]
fn metadata() -> Vec<(String, MetadataValue)> {
    _metadata()
}

#[ic_cdk::update]
fn set_timestamp_ns(ts: u64) {
    _set_timestamp_ns(ts)
}

#[ic_cdk::update]
fn run_periodic_task() {
    _run_periodic_task()
}

#[ic_cdk::query]
fn provider_list_registered() -> Result<Vec<String>, String> {
    _provider_list_registered()
}

/// Query endpoint to get entries from the next block with simple paging
/// Returns entries in chronological order (insertion order)
#[ic_cdk::query]
fn next_block_entries(
    label: Option<String>,
    offset: Option<u32>,
    limit: Option<u32>,
) -> NextBlockEntriesResult {
    _next_block_entries(label, offset.unwrap_or(0), limit.unwrap_or(100))
}

/// Query endpoint to get committed ledger entries with simple paging
/// Returns entries from committed blocks (not next_block) in chronological order
#[ic_cdk::query]
fn ledger_entries(
    label: Option<String>,
    offset: Option<u32>,
    limit: Option<u32>,
) -> NextBlockEntriesResult {
    _ledger_entries(label, offset.unwrap_or(0), limit.unwrap_or(100))
}

/// Query endpoint to get the next block from the ledger for synchronization (kept for compatibility)
#[ic_cdk::query]
fn next_block_sync(
    start_position: Option<u64>,
    include_data: Option<bool>,
    max_entries: Option<u32>,
) -> Result<NextBlockSyncResponse, String> {
    let request = NextBlockSyncRequest {
        start_position,
        include_data: include_data.unwrap_or(true),
        max_entries: max_entries.map(|e| e as usize),
    };
    _next_block_sync(request)
}

// test utilities
#[ic_cdk::query]
fn get_timestamp_ns() -> u64 {
    dcc_common::get_timestamp_ns()
}
