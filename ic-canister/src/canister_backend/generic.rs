use super::pre_icrc3::ledger_construct_hash_tree;
use borsh::BorshDeserialize;
use candid::Principal;
use dcc_common::cache_transactions::RecentCache;
use dcc_common::{
    account_balance_get, account_registration_fee_e9s, blocks_until_next_halving, cursor_from_data,
    get_account_from_pubkey, get_num_offerings, get_num_providers, get_pubkey_from_principal,
    refresh_caches_from_ledger, reputation_get, reward_e9s_per_block_recalculate,
    rewards_current_block_checked_in, rewards_distribute, rewards_pending_e9s, set_test_config,
    ContractId, ContractReqSerialized, FundsTransfer, LedgerCursor, TokenAmountE9s,
    BLOCK_INTERVAL_SECS, CACHE_TXS_NUM_COMMITTED, DATA_PULL_BYTES_BEFORE_LEN,
    LABEL_CONTRACT_SIGN_REQUEST, LABEL_DC_TOKEN_TRANSFER, LABEL_NP_CHECK_IN, LABEL_NP_OFFERING,
    LABEL_NP_PROFILE, LABEL_NP_REGISTER, LABEL_REWARD_DISTRIBUTION, LABEL_USER_REGISTER,
    MAX_RESPONSE_BYTES_NON_REPLICATED,
};
use flate2::{write::ZlibEncoder, Compression};
use ic_cdk::println;
use icrc_ledger_types::icrc::generic_metadata_value::MetadataValue;
use ledger_map::platform_specific::{persistent_storage_read, persistent_storage_write};
use ledger_map::{error, info, warn, LedgerMap};
use serde::Serialize;
use std::cell::RefCell;
use std::io::prelude::*;
use std::ops::AddAssign;
use std::time::Duration;

thread_local! {
    // Ledger that indexes only specific labels, to save on memory
    pub(crate) static LEDGER_MAP: RefCell<LedgerMap> = RefCell::new(LedgerMap::new(Some(vec![
        LABEL_NP_REGISTER.to_string(),
        LABEL_NP_CHECK_IN.to_string(),
        LABEL_USER_REGISTER.to_string(),
        LABEL_REWARD_DISTRIBUTION.to_string(),
        LABEL_NP_PROFILE.to_string(),
        LABEL_NP_OFFERING.to_string(),
        LABEL_CONTRACT_SIGN_REQUEST.to_string(),
    ])).expect("Failed to create LedgerMap"));
    pub(crate) static AUTHORIZED_PUSHER: RefCell<Option<Principal>> = const { RefCell::new(None) };
    #[cfg(target_arch = "wasm32")]
    static TIMER_IDS: RefCell<Vec<ic_cdk_timers::TimerId>> = const { RefCell::new(Vec::new()) };
    static COMMIT_INTERVAL: Duration = const { Duration::from_secs(BLOCK_INTERVAL_SECS) };
    pub(crate) static LAST_TOKEN_VALUE_USD_E6: RefCell<u64> = const { RefCell::new(1_000_000) }; // 6 decimal places
}

pub fn update_last_token_value_usd_e6(new_value: u64) {
    LAST_TOKEN_VALUE_USD_E6
        .with(|last_token_value_usd_e6| *last_token_value_usd_e6.borrow_mut() = new_value);
}

pub fn get_last_token_value_usd_e6() -> u64 {
    LAST_TOKEN_VALUE_USD_E6.with(|last_token_value_usd_e6| *last_token_value_usd_e6.borrow())
}

pub fn refresh_last_token_value_usd_e6() {
    // FIXME: Get the Token value from ICPSwap and KongSwap
    let token_value = 1_000_000;
    update_last_token_value_usd_e6(token_value);
}

pub(crate) fn get_commit_interval() -> Duration {
    COMMIT_INTERVAL.with(|commit_interval| *commit_interval)
}

fn ledger_periodic_task() {
    refresh_last_token_value_usd_e6();
    LEDGER_MAP.with(|ledger| {
        let ledger = &mut ledger.borrow_mut();
        match rewards_distribute(ledger) {
            Ok(_) => {}
            Err(e) => error!("Ledger commit: Failed to distribute rewards: {}", e),
            // Intentionally don't panic. If needed, transactions can be replayed and corrected.
        }

        let mut tx_num = CACHE_TXS_NUM_COMMITTED.with(|n| *n.borrow());
        for entry in ledger.next_block_iter(Some(LABEL_DC_TOKEN_TRANSFER)) {
            let transfer: FundsTransfer = BorshDeserialize::try_from_slice(entry.value())
                .unwrap_or_else(|e| {
                    ic_cdk::api::trap(&format!(
                        "Failed to deserialize transfer {:?} ==> {:?}",
                        entry, e
                    ));
                });
            RecentCache::add_entry(tx_num, transfer.into());
            tx_num += 1;
        }

        // Uncommitted transactions now get committed -- adjust the (cache) count of total committed transactions
        let count_total_txs_uncommitted =
            ledger.get_next_block_entries_count(Some(LABEL_DC_TOKEN_TRANSFER)) as u64;

        CACHE_TXS_NUM_COMMITTED.with(|n| n.borrow_mut().add_assign(count_total_txs_uncommitted));

        ledger.commit_block().unwrap_or_else(|e| {
            error!("Failed to commit ledger: {}", e);
        });

        // Set certified data, for compliance with ICRC-3
        // Borrowed from https://github.com/ldclabs/ic-sft/blob/4825d760811731476ffbbb1705295a6ad4aae58f/src/ic_sft_canister/src/store.rs#L193-L210
        let root_hash = ledger_construct_hash_tree(ledger).digest();
        ic_cdk::api::set_certified_data(&root_hash);
    });
}

pub fn encode_to_cbor_bytes(obj: &impl Serialize) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    ciborium::into_writer(obj, &mut buf).expect("failed to encode to CBOR");
    buf
}

// Compilation with timers fails on targets other than wasm32, so we have two different functions.
#[cfg(target_arch = "wasm32")]
fn start_periodic_ledger_task() {
    let secs = get_commit_interval();
    info!("Timer canister: Starting a periodic commit block timer with {secs:?} interval...");

    // Schedule a new periodic task
    let timer_id = ic_cdk_timers::set_timer_interval(secs, ledger_periodic_task);
    // Add the timer ID to the global vector.
    TIMER_IDS.with(|timer_ids| timer_ids.borrow_mut().push(timer_id));
}

// Compilation with timers fails on targets other than wasm32, so we use a mock function on other targets.
#[cfg(not(target_arch = "wasm32"))]
fn start_periodic_ledger_task() {
    let _secs = get_commit_interval();
    ledger_periodic_task();
}

pub fn _init(enable_test_config: Option<bool>) {
    start_periodic_ledger_task();
    LEDGER_MAP.with(|ledger| {
        refresh_caches_from_ledger(&ledger.borrow()).expect("Loading balances from ledger failed");
    });
    println!(
        "init: test_config = {}",
        enable_test_config.unwrap_or_default()
    );
    set_test_config(enable_test_config.unwrap_or_default());
}

pub fn _pre_upgrade() {
    LEDGER_MAP.with(|ledger| {
        ledger.borrow_mut().commit_block().unwrap_or_else(|e| {
            error!("Failed to commit ledger: {}", e);
        });
        // Set certified data, for compliance with ICRC-3
        ic_cdk::api::set_certified_data(&ledger.borrow().get_latest_block_hash());
    });
}

pub fn _post_upgrade(enable_test_config: Option<bool>) {
    start_periodic_ledger_task();
    LEDGER_MAP.with(|ledger| {
        refresh_caches_from_ledger(&ledger.borrow()).expect("Loading balances from ledger failed");
        reward_e9s_per_block_recalculate();
    });
    set_test_config(enable_test_config.unwrap_or_default());
}

pub(crate) fn _get_registration_fee() -> TokenAmountE9s {
    account_registration_fee_e9s()
}

pub(crate) fn _np_register(
    pubkey_bytes: Vec<u8>,
    signature_bytes: Vec<u8>,
) -> Result<String, String> {
    // To prevent DOS attacks, a fee is charged for executing this operation
    LEDGER_MAP.with(|ledger| {
        dcc_common::do_account_register(
            &mut ledger.borrow_mut(),
            LABEL_NP_REGISTER,
            pubkey_bytes,
            signature_bytes,
        )
    })
}

pub(crate) fn _user_register(
    pubkey_bytes: Vec<u8>,
    signature_bytes: Vec<u8>,
) -> Result<String, String> {
    // To prevent DOS attacks, a fee is charged for executing this operation
    LEDGER_MAP.with(|ledger| {
        dcc_common::do_account_register(
            &mut ledger.borrow_mut(),
            LABEL_USER_REGISTER,
            pubkey_bytes,
            signature_bytes,
        )
    })
}

pub(crate) fn _node_provider_check_in(
    pubkey_bytes: Vec<u8>,
    memo: String,
    nonce_signature: Vec<u8>,
) -> Result<String, String> {
    // To prevent DOS attacks, a fee is charged for executing this operation
    LEDGER_MAP.with(|ledger| {
        dcc_common::do_node_provider_check_in(
            &mut ledger.borrow_mut(),
            pubkey_bytes,
            memo,
            nonce_signature,
        )
    })
}

pub(crate) fn _node_provider_update_profile(
    pubkey_bytes: Vec<u8>,
    profile_serialized: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    // To prevent DOS attacks, a fee is charged for executing this operation
    LEDGER_MAP.with(|ledger| {
        dcc_common::do_node_provider_update_profile(
            &mut ledger.borrow_mut(),
            pubkey_bytes,
            profile_serialized,
            crypto_signature,
        )
    })
}

pub(crate) fn _node_provider_update_offering(
    pubkey_bytes: Vec<u8>,
    update_offering_payload: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    // To prevent DOS attacks, a fee is charged for executing this operation
    LEDGER_MAP.with(|ledger| {
        dcc_common::do_node_provider_update_offering(
            &mut ledger.borrow_mut(),
            pubkey_bytes,
            update_offering_payload,
            crypto_signature,
        )
    })
}

pub(crate) fn _node_provider_get_profile_by_pubkey_bytes(pubkey_bytes: Vec<u8>) -> Option<String> {
    let np_profile = LEDGER_MAP
        .with(|ledger| dcc_common::do_node_provider_get_profile(&ledger.borrow(), pubkey_bytes));
    np_profile
        .map(|np_profile| serde_json::to_string_pretty(&np_profile).expect("Failed to encode"))
}

pub(crate) fn _node_provider_get_profile_by_principal(principal: Principal) -> Option<String> {
    let pubkey_bytes = get_pubkey_from_principal(principal);
    _node_provider_get_profile_by_pubkey_bytes(pubkey_bytes)
}

pub(crate) fn _get_check_in_nonce() -> Vec<u8> {
    LEDGER_MAP.with(|ledger| ledger.borrow().get_latest_block_hash())
}

pub(crate) fn _offering_search(query: String) -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut response_bytes = 0;
    let mut response = Vec::new();
    let max_offering_response_bytes = MAX_RESPONSE_BYTES_NON_REPLICATED * 9 / 10; // 90% of max response bytes
    LEDGER_MAP.with(|ledger| {
        for (dcc_id, offering) in dcc_common::do_get_matching_offerings(&ledger.borrow(), &query) {
            // convert results to json and compress that json with zlib
            let offering_json_string = match offering.as_json_string() {
                Ok(json) => json,
                Err(e) => {
                    warn!("Failed to serialize offering: {}", e);
                    continue;
                }
            };
            let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());

            enc.write_all(offering_json_string.as_bytes())
                .expect("Failed to compress");
            let compressed = enc.finish().expect("Failed to compress");
            let pubkey_bytes = dcc_id.to_bytes_verifying();
            response_bytes += pubkey_bytes.len() + compressed.len();
            if response_bytes > max_offering_response_bytes {
                break;
            }
            response.push((pubkey_bytes, compressed));
        }
    });
    response
}

pub(crate) fn _contract_sign_request(
    pubkey_bytes: Vec<u8>,
    request_serialized: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    LEDGER_MAP.with(|ledger| {
        dcc_common::do_contract_sign_request(
            &mut ledger.borrow_mut(),
            pubkey_bytes,
            request_serialized,
            crypto_signature,
        )
    })
}

pub(crate) fn _contracts_list_pending(
    pubkey_bytes: Option<Vec<u8>>,
) -> Vec<(ContractId, ContractReqSerialized)> {
    LEDGER_MAP.with(|ledger| {
        dcc_common::do_contracts_list_pending(&mut ledger.borrow_mut(), pubkey_bytes)
    })
}

pub(crate) fn _contract_sign_reply(
    pubkey_bytes: Vec<u8>,
    reply_serialized: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    LEDGER_MAP.with(|ledger| {
        dcc_common::do_contract_sign_reply(
            &mut ledger.borrow_mut(),
            pubkey_bytes,
            reply_serialized,
            crypto_signature,
        )
    })
}

pub(crate) fn _get_identity_reputation(identity: Vec<u8>) -> u64 {
    reputation_get(identity)
}

pub(crate) fn _node_provider_list_checked_in() -> Result<Vec<String>, String> {
    LEDGER_MAP.with(|ledger| {
        let binding = ledger.borrow();
        let np_vec = binding
            .next_block_iter(Some(LABEL_NP_CHECK_IN))
            .map(|entry| String::from_utf8_lossy(entry.key()).to_string())
            .collect::<Vec<String>>();
        Ok(np_vec)
    })
}

pub(crate) fn _data_fetch(
    cursor: Option<String>,
    bytes_before: Option<Vec<u8>>,
) -> Result<(String, Vec<u8>), String> {
    LEDGER_MAP.with(|ledger| {
        info!(
            "Serving data request with cursor: {} and bytes_before: {}",
            cursor.as_ref().unwrap_or(&String::new()),
            hex::encode(bytes_before.as_ref().unwrap_or(&vec![]))
        );
        let req_cursor = LedgerCursor::new_from_string(cursor.unwrap_or_default());
        let req_position_start = req_cursor.position;
        let local_cursor = cursor_from_data(
            ledger_map::partition_table::get_data_partition().start_lba,
            ledger_map::platform_specific::persistent_storage_size_bytes(),
            ledger.borrow().get_next_block_start_pos(),
            req_position_start,
        );
        info!("Calculated cursor: {:?}", local_cursor);
        if req_position_start > local_cursor.position {
            return Err("Provided position start is after the end of the ledger".to_string());
        }
        if local_cursor.response_bytes == 0 {
            return Ok((local_cursor.to_urlenc_string(), vec![]));
        }
        if let Some(bytes_before) = bytes_before {
            if local_cursor.position > DATA_PULL_BYTES_BEFORE_LEN as u64 {
                let mut buf_bytes_before = vec![0u8; DATA_PULL_BYTES_BEFORE_LEN as usize];
                persistent_storage_read(
                    local_cursor.position - DATA_PULL_BYTES_BEFORE_LEN as u64,
                    &mut buf_bytes_before,
                )
                .map_err(|e| e.to_string())?;
                if bytes_before != buf_bytes_before {
                    return Err(format!(
                        "{} bytes before position {} does not match",
                        DATA_PULL_BYTES_BEFORE_LEN, local_cursor.position
                    ));
                }
            }
        }

        let mut buf = vec![0u8; local_cursor.response_bytes as usize];
        persistent_storage_read(local_cursor.position, &mut buf).map_err(|e| e.to_string())?;
        info!(
            "Fetching {} bytes from position 0x{:0x}",
            local_cursor.response_bytes, local_cursor.position
        );
        Ok((local_cursor.to_urlenc_string(), buf))
    })
}

pub(crate) fn _data_push_auth() -> Result<String, String> {
    // If LEDGER_MAP is currently empty and there is no authorized pusher,
    // set the authorized pusher to the caller.
    LEDGER_MAP.with(|ledger| {
        let ledger = ledger.borrow();
        let authorized_pusher =
            AUTHORIZED_PUSHER.with(|authorized_pusher| *authorized_pusher.borrow());
        if ledger.get_blocks_count() == 0 {
            let caller = ic_cdk::api::caller();

            match authorized_pusher {
                Some(authorized_pusher) => {
                    if caller == authorized_pusher {
                        Ok(format!("Success! Authorized pusher is {}", caller))
                    } else {
                        Err(format!("Failed to authorize caller {}", caller))
                    }
                }
                None => {
                    AUTHORIZED_PUSHER.with(|authorized_pusher| {
                        authorized_pusher.borrow_mut().replace(caller);
                    });
                    Ok(format!("Success! Authorized pusher is set to {}", caller))
                }
            }
        } else {
            Err("Ledger is not empty".to_string())
        }
    })
}

pub(crate) fn _data_push(cursor: String, data: Vec<u8>) -> Result<String, String> {
    let caller = ic_cdk::api::caller();
    let authorized_pusher = AUTHORIZED_PUSHER.with(|authorized_pusher| *authorized_pusher.borrow());

    match authorized_pusher {
        Some(authorized_pusher) => {
            if caller != authorized_pusher {
                return Err("Caller is not authorized".to_string());
            }
            info!(
                "Caller {} pushing {} bytes with cursor {}",
                caller,
                data.len(),
                cursor
            );
            let cursor = LedgerCursor::new_from_string(cursor);
            persistent_storage_write(cursor.position, &data);
            let refresh = if cursor.more {
                "; ledger NOT refreshed".to_string()
            } else {
                LEDGER_MAP.with(|ledger| {
                    if let Err(e) = ledger.borrow_mut().refresh_ledger() {
                        error!("Failed to refresh ledger: {}", e)
                    }
                    refresh_caches_from_ledger(&ledger.borrow())
                        .expect("Loading balances from ledger failed");
                    reward_e9s_per_block_recalculate();
                });
                "; ledger refreshed".to_string()
            };
            let response = format!(
                "Success! {} pushed {} bytes at position 0x{:0x} {}",
                caller,
                data.len(),
                cursor.position,
                refresh
            );
            Ok(response)
        }
        None => Err("No principal is authorized as a pusher".to_string()),
    }
}

pub(crate) fn _metadata() -> Vec<(String, MetadataValue)> {
    let authorized_pusher = AUTHORIZED_PUSHER.with(|authorized_pusher| *authorized_pusher.borrow());
    LEDGER_MAP.with(|ledger| {
        let ledger = ledger.borrow();
        vec![
            MetadataValue::entry(
                "ledger:data_start_lba",
                ledger_map::partition_table::get_data_partition().start_lba,
            ),
            MetadataValue::entry("ledger:num_blocks", ledger.get_blocks_count() as u64),
            MetadataValue::entry("ledger:latest_block_hash", ledger.get_latest_block_hash()),
            MetadataValue::entry(
                "ledger:latest_block_timestamp_ns",
                ledger.get_latest_block_timestamp_ns(),
            ),
            MetadataValue::entry(
                "ledger:next_block_start_pos",
                ledger.get_next_block_start_pos(),
            ),
            MetadataValue::entry(
                "ledger:authorized_pusher",
                authorized_pusher.map(|s| s.to_string()).unwrap_or_default(),
            ),
            MetadataValue::entry(
                "ledger:token_value_in_usd_e6",
                get_last_token_value_usd_e6(),
            ),
            MetadataValue::entry("ledger:total_providers", get_num_providers()),
            MetadataValue::entry("ledger:total_offerings", get_num_offerings()),
            MetadataValue::entry(
                "ledger:blocks_until_next_halving",
                blocks_until_next_halving(),
            ),
            MetadataValue::entry(
                "ledger:current_block_validators",
                rewards_current_block_checked_in(&ledger) as u64,
            ),
            MetadataValue::entry(
                "ledger:current_block_rewards_e9s",
                rewards_pending_e9s(&ledger),
            ),
        ]
    })
}

pub(crate) fn _set_timestamp_ns(ts: u64) {
    if !dcc_common::is_test_config() {
        ic_cdk::trap("invalid request");
    }
    info!("set_timestamp_ns: {}", ts);
    dcc_common::set_timestamp_ns(ts)
}

pub(crate) fn _run_periodic_task() {
    if !dcc_common::is_test_config() {
        ic_cdk::trap("invalid request");
    }
    ledger_periodic_task();
}

pub(crate) fn _node_provider_list_registered() -> Result<Vec<String>, String> {
    LEDGER_MAP.with(|ledger| {
        let binding = ledger.borrow();
        let np_vec = binding
            .iter(Some(LABEL_NP_REGISTER))
            .map(|entry| {
                let np = String::from_utf8_lossy(entry.key());
                let acct = get_account_from_pubkey(entry.value());
                let balance = account_balance_get(&acct);
                format!("{} ==> {} (acct balance: {})", np, acct, balance)
            })
            .collect::<Vec<String>>();
        Ok(np_vec)
    })
}
