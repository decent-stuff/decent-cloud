use super::pre_icrc3::ledger_construct_hash_tree;
use candid::{CandidType, Principal};
use dcc_common::{
    account_balance_get, account_registration_fee_e9s, blocks_until_next_halving, cursor_from_data,
    get_account_from_pubkey, get_num_providers, recent_transactions_cleanup,
    refresh_caches_from_ledger, reputation_get, reward_e9s_per_block,
    reward_e9s_per_block_recalculate, rewards_current_block_checked_in, rewards_distribute,
    rewards_pending_e9s, set_test_config, LedgerCursor, NextBlockSyncRequest,
    NextBlockSyncResponse, RecentCache, TokenAmountE9s, BLOCK_INTERVAL_SECS,
    DATA_PULL_BYTES_BEFORE_LEN, LABEL_PROV_CHECK_IN, LABEL_PROV_REGISTER,
    LABEL_REWARD_DISTRIBUTION, LABEL_USER_REGISTER,
};
use ic_cdk::println;
use icrc_ledger_types::icrc::generic_metadata_value::MetadataValue;
use ledger_map::platform_specific::{persistent_storage_read, persistent_storage_write};
use ledger_map::{error, info, LedgerMap};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::time::Duration;

/// Individual ledger entry
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct LedgerEntry {
    pub label: String,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

/// Resume cursor for efficient pagination through ledger entries
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct ResumeCursor {
    pub block_position: u64, // LBA (Logical Block Address) to resume from
    pub entry_index: u32,    // Entry index within that block to resume from
}

/// Result containing ledger entries and pagination info
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct LedgerEntriesResult {
    pub entries: Vec<LedgerEntry>,
    pub has_more: bool,
    pub next_cursor: Option<ResumeCursor>,
}

thread_local! {
    // Ledger that indexes only specific labels, to save on memory
    // CRITICAL: LedgerMap creation is essential for canister operation.
    // If it fails, the canister cannot function and must trap.
    pub(crate) static LEDGER_MAP: RefCell<LedgerMap> = {
        match LedgerMap::new(Some(vec![
            LABEL_PROV_REGISTER.to_string(),
            LABEL_PROV_CHECK_IN.to_string(),
            LABEL_USER_REGISTER.to_string(),
            LABEL_REWARD_DISTRIBUTION.to_string(),
        ])) {
            Ok(ledger) => RefCell::new(ledger),
            Err(e) => ic_cdk::trap(format!("CRITICAL: Failed to create LedgerMap: {}", e)),
        }
    };
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
            Err(e) => error!("Ledger commit: Failed to distribute rewards: {:#}", e),
            // Intentionally don't panic. If needed, transactions can be replayed and corrected.
        }

        // Commit the block
        ledger.commit_block().unwrap_or_else(|e| {
            error!("Failed to commit ledger: {:#}", e);
        });

        // Set certified data, for compliance with ICRC-3
        // Borrowed from https://github.com/ldclabs/ic-sft/blob/4825d760811731476ffbbb1705295a6ad4aae58f/src/ic_sft_canister/src/store.rs#L193-L210
        let root_hash = ledger_construct_hash_tree(ledger).digest();
        ic_cdk::api::certified_data_set(root_hash);

        // Cleanup old transactions that are used for deduplication
        recent_transactions_cleanup();
        RecentCache::ensure_cache_length();
    });
}

pub fn encode_to_cbor_bytes(obj: &impl Serialize) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    ciborium::into_writer(obj, &mut buf)
        .unwrap_or_else(|e| ic_cdk::trap(format!("CRITICAL: Failed to encode to CBOR: {}", e)));
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
        if let Err(e) = refresh_caches_from_ledger(&ledger.borrow()) {
            ic_cdk::trap(format!(
                "CRITICAL: _init failed to load caches from ledger: {}",
                e
            ));
        }
    });
    println!(
        "init: test_config = {}",
        enable_test_config.unwrap_or_default()
    );
    set_test_config(enable_test_config.unwrap_or_default());
}

pub fn _pre_upgrade() {
    LEDGER_MAP.with(|ledger| {
        // Force commit any pending next block data for upgrade safety
        ledger
            .borrow_mut()
            .force_commit_block()
            .unwrap_or_else(|e| {
                error!("Failed to force commit ledger: {:#}", e);
            });
        // Set certified data, for compliance with ICRC-3
        ic_cdk::api::certified_data_set(ledger.borrow().get_latest_block_hash());
    });
}

pub fn _post_upgrade(enable_test_config: Option<bool>) {
    start_periodic_ledger_task();
    LEDGER_MAP.with(|ledger| {
        if let Err(e) = refresh_caches_from_ledger(&ledger.borrow()) {
            ic_cdk::trap(format!(
                "CRITICAL: _post_upgrade failed to load caches from ledger: {}",
                e
            ));
        }
        reward_e9s_per_block_recalculate();
    });
    set_test_config(enable_test_config.unwrap_or_default());
}

pub(crate) fn _get_registration_fee() -> TokenAmountE9s {
    account_registration_fee_e9s()
}

pub(crate) fn _provider_register(
    pubkey_bytes: Vec<u8>,
    signature_bytes: Vec<u8>,
) -> Result<String, String> {
    // To prevent DOS attacks, a fee is charged for executing this operation
    LEDGER_MAP.with(|ledger| {
        dcc_common::do_account_register(
            &mut ledger.borrow_mut(),
            LABEL_PROV_REGISTER,
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

pub(crate) fn _provider_check_in(
    pubkey_bytes: Vec<u8>,
    memo: String,
    nonce_signature: Vec<u8>,
) -> Result<String, String> {
    // To prevent DOS attacks, a fee is charged for executing this operation
    LEDGER_MAP.with(|ledger| {
        dcc_common::do_provider_check_in(
            &mut ledger.borrow_mut(),
            pubkey_bytes,
            memo,
            nonce_signature,
        )
    })
}

pub(crate) fn _get_check_in_nonce() -> Vec<u8> {
    LEDGER_MAP.with(|ledger| ledger.borrow().get_latest_block_hash())
}

pub(crate) fn _get_identity_reputation(identity: Vec<u8>) -> u64 {
    reputation_get(identity)
}

pub(crate) fn _provider_list_checked_in() -> Result<Vec<String>, String> {
    LEDGER_MAP.with(|ledger| {
        let binding = ledger.borrow();
        let provider_vec = binding
            .next_block_iter(Some(LABEL_PROV_CHECK_IN))
            .map(|entry| String::from_utf8_lossy(entry.key()).to_string())
            .collect::<Vec<String>>();
        Ok(provider_vec)
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
        let req_cursor = LedgerCursor::new_from_string(cursor.unwrap_or_default())
            .map_err(|e| format!("Failed to parse cursor: {}", e))?;
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
            let caller = ic_cdk::api::msg_caller();

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
    let caller = ic_cdk::api::msg_caller();
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
            let cursor = LedgerCursor::new_from_string(cursor)
                .map_err(|e| format!("Failed to parse cursor: {}", e))?;
            persistent_storage_write(cursor.position, &data);
            persistent_storage_write(
                cursor.position + data.len() as u64,
                &vec![0u8; size_of::<ledger_map::ledger_entry::LedgerBlockHeader>()],
            );

            let refresh = if cursor.more {
                "; ledger NOT refreshed".to_string()
            } else {
                let refresh_result = LEDGER_MAP.with(|ledger| {
                    // TODO: Entire ledger is iterated twice, effectively. It should be possible to do this in a single go.
                    if let Err(e) = ledger.borrow_mut().refresh_ledger() {
                        error!("Failed to refresh ledger: {:#}", e)
                    }
                    refresh_caches_from_ledger(&ledger.borrow())
                        .map_err(|e| format!("Failed to refresh caches from ledger: {:#}", e))
                        .map(|_| {
                            reward_e9s_per_block_recalculate();
                        })
                });
                refresh_result?;
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
        reward_e9s_per_block_recalculate();
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
            MetadataValue::entry("ledger:reward_per_block_e9s", reward_e9s_per_block()),
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

pub(crate) fn _provider_list_registered() -> Result<Vec<String>, String> {
    LEDGER_MAP.with(|ledger| {
        let binding = ledger.borrow();
        let provider_vec = binding
            .iter(Some(LABEL_PROV_REGISTER))
            .map(|entry| {
                let provider = String::from_utf8_lossy(entry.key());
                let acct = get_account_from_pubkey(entry.value())?;
                let balance = account_balance_get(&acct);
                Ok(format!("{} ==> {} (acct balance: {})", provider, acct, balance))
            })
            .collect::<Result<Vec<String>, String>>()?;
        Ok(provider_vec)
    })
}

/// Get committed ledger entries from raw blocks with cursor-based pagination
/// Uses iter_raw() to iterate through committed blocks starting from cursor position
///
/// # Arguments
/// * `label` - Optional label filter (e.g., "ProvProfile", "ProvOffering")
/// * `cursor` - Optional resume cursor (block_position + entry_index within block)
/// * `limit` - Maximum number of entries to return
/// * `include_next_block` - If true, includes uncommitted entries from next_block
pub(crate) fn _ledger_entries(
    label: Option<String>,
    cursor: Option<ResumeCursor>,
    limit: u32,
    include_next_block: bool,
) -> LedgerEntriesResult {
    LEDGER_MAP.with(|ledger| {
        let ledger_ref = ledger.borrow();
        let limit = limit as usize;

        // Extract starting position from cursor, default to 0
        let start_block_pos = cursor.as_ref().map(|c| c.block_position).unwrap_or(0);
        let start_entry_idx = cursor.as_ref().map(|c| c.entry_index).unwrap_or(0) as usize;

        let mut collected_entries = Vec::new();
        let mut current_block_pos = start_block_pos;
        let mut next_cursor: Option<ResumeCursor> = None;

        // Phase 1: Iterate through committed blocks starting from cursor position
        if ledger_ref.get_blocks_count() > 0 {
            for block_result in ledger_ref.iter_raw(start_block_pos) {
                match block_result {
                    Ok((block_header, ledger_block)) => {
                        let mut block_entry_idx = 0u32;

                        for entry in ledger_block.entries() {
                            // Apply label filter if specified
                            if let Some(ref filter_label) = label {
                                if entry.label() != filter_label {
                                    continue;
                                }
                            }

                            // Only process upsert operations
                            if entry.operation() == ledger_map::ledger_entry::Operation::Upsert {
                                // Skip entries before start_entry_idx in the first block
                                if current_block_pos == start_block_pos
                                    && (block_entry_idx as usize) < start_entry_idx
                                {
                                    block_entry_idx += 1;
                                    continue;
                                }

                                // Check if we've collected enough entries
                                if collected_entries.len() >= limit {
                                    // Set next_cursor to resume from this position
                                    next_cursor = Some(ResumeCursor {
                                        block_position: current_block_pos,
                                        entry_index: block_entry_idx,
                                    });
                                    break;
                                }

                                collected_entries.push(LedgerEntry {
                                    label: entry.label().to_string(),
                                    key: entry.key().to_vec(),
                                    value: entry.value().to_vec(),
                                });

                                block_entry_idx += 1;
                            }
                        }

                        // If we've collected enough, stop iterating blocks
                        if collected_entries.len() >= limit {
                            break;
                        }

                        // Move to next block
                        current_block_pos += block_header.jump_bytes_next_block() as u64;
                    }
                    Err(e) => {
                        error!("Failed to read block during iter_raw: {:#}", e);
                        break;
                    }
                }
            }
        }

        // Phase 2: Optionally add next_block entries
        if include_next_block && collected_entries.len() < limit {
            let next_block_start_pos = ledger_ref.get_next_block_start_pos();
            let mut next_block_entry_idx = 0u32;

            for entry in ledger_ref.next_block_iter(label.as_deref()) {
                // If we're starting from next_block, skip entries before start_entry_idx
                if next_cursor.is_none()
                    && next_block_start_pos == start_block_pos
                    && (next_block_entry_idx as usize) < start_entry_idx
                {
                    next_block_entry_idx += 1;
                    continue;
                }

                if collected_entries.len() >= limit {
                    next_cursor = Some(ResumeCursor {
                        block_position: next_block_start_pos,
                        entry_index: next_block_entry_idx,
                    });
                    break;
                }

                collected_entries.push(LedgerEntry {
                    label: entry.label().to_string(),
                    key: entry.key().to_vec(),
                    value: entry.value().to_vec(),
                });

                next_block_entry_idx += 1;
            }
        }

        let has_more = next_cursor.is_some();

        LedgerEntriesResult {
            entries: collected_entries,
            has_more,
            next_cursor,
        }
    })
}

/// Get the current next block data for syncing (kept for compatibility)
/// Returns Borsh-pre-serialized data from in-memory buffer
pub(crate) fn _next_block_sync(
    _request: NextBlockSyncRequest,
) -> Result<NextBlockSyncResponse, String> {
    LEDGER_MAP.with(|ledger| {
        let ledger_ref = ledger.borrow();
        let serialized_data = ledger_ref.get_next_block_serialized_data();
        if serialized_data.is_empty() {
            Ok(NextBlockSyncResponse {
                has_block: false,
                block_position: Some(ledger_ref.get_next_block_start_pos()),
                ..Default::default()
            })
        } else {
            Ok(NextBlockSyncResponse {
                has_block: true,
                block_data: Some(serialized_data), // Reuse field for serialized entries
                entries_count: ledger_ref.get_next_block_entries_count(None),
                block_position: Some(ledger_ref.get_next_block_start_pos()),
                ..Default::default()
            })
        }
    })
}
