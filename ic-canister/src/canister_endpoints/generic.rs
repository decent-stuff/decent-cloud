use crate::canister_backend::generic::*;
use candid::Principal;
use dcc_common::{ContractId, ContractReqSerialized, TokenAmountE9s};
#[allow(unused_imports)]
use ic_cdk::println;
use icrc_ledger_types::icrc::generic_metadata_value::MetadataValue;

#[ic_cdk::init]
async fn init(enable_test_config: Option<bool>) {
    _init(enable_test_config).await
}

#[ic_cdk::pre_upgrade]
async fn pre_upgrade() {
    _pre_upgrade().await
}

#[ic_cdk::post_upgrade]
async fn post_upgrade(enable_test_config: Option<bool>) {
    _post_upgrade(enable_test_config).await
}

#[ic_cdk::query]
fn get_registration_fee() -> TokenAmountE9s {
    _get_registration_fee()
}

#[ic_cdk::update]
fn node_provider_register(
    pubkey_bytes: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    _np_register(pubkey_bytes, crypto_signature)
}

#[ic_cdk::update]
fn user_register(pubkey_bytes: Vec<u8>, crypto_signature: Vec<u8>) -> Result<String, String> {
    _user_register(pubkey_bytes, crypto_signature)
}

#[ic_cdk::update]
fn node_provider_check_in(
    pubkey_bytes: Vec<u8>,
    memo: String,
    nonce_crypto_signature: Vec<u8>,
) -> Result<String, String> {
    _node_provider_check_in(pubkey_bytes, memo, nonce_crypto_signature)
}

#[ic_cdk::update]
fn node_provider_update_profile(
    pubkey_bytes: Vec<u8>,
    profile_serialized: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    _node_provider_update_profile(pubkey_bytes, profile_serialized, crypto_signature)
}

#[ic_cdk::update]
fn node_provider_update_offering(
    pubkey_bytes: Vec<u8>,
    offering_serialized: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    _node_provider_update_offering(pubkey_bytes, offering_serialized, crypto_signature)
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
fn node_provider_get_profile_by_pubkey_bytes(pubkey_bytes: Vec<u8>) -> Option<String> {
    _node_provider_get_profile_by_pubkey_bytes(pubkey_bytes)
}

#[ic_cdk::query]
fn node_provider_get_profile_by_principal(principal: Principal) -> Option<String> {
    _node_provider_get_profile_by_principal(principal)
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
fn node_provider_list_checked_in() -> Result<Vec<String>, String> {
    _node_provider_list_checked_in()
}

#[ic_cdk::query]
async fn data_fetch(
    cursor: Option<String>,
    bytes_before: Option<Vec<u8>>,
) -> Result<(String, Vec<u8>), String> {
    _data_fetch(cursor, bytes_before).await
}

#[ic_cdk::update]
fn data_push_auth() -> Result<String, String> {
    _data_push_auth()
}

#[ic_cdk::update]
async fn data_push(cursor: String, data: Vec<u8>) -> Result<String, String> {
    _data_push(cursor, data).await
}

#[ic_cdk::query]
async fn metadata() -> Vec<(String, MetadataValue)> {
    _metadata().await
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
fn node_provider_list_registered() -> Result<Vec<String>, String> {
    _node_provider_list_registered()
}

// test utilities
#[ic_cdk::query]
fn get_timestamp_ns() -> u64 {
    dcc_common::get_timestamp_ns()
}
