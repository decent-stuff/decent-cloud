use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use dcc_common::{
    ledger_block_parse_entries, DccIdentity, WasmLedgerEntry, DATA_PULL_BYTES_BEFORE_LEN,
};
#[cfg(target_arch = "wasm32")]
use ledger_map::platform_specific_wasm32_browser as ledger_storage;
use ledger_map::{info, warn, LedgerMap};
use serde::Serialize;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

thread_local! {
    static LEDGER_MAP: RefCell<LedgerMap> = RefCell::new(LedgerMap::new(None).expect("Failed to create LedgerMap"));
}

#[wasm_bindgen]
pub fn init() {
    console_error_panic_hook::set_once();

    // Initialize storage as the very first thing
    #[cfg(target_arch = "wasm32")]
    ledger_storage::ensure_storage_is_initialized();
}

#[wasm_bindgen]
pub fn ledger_storage_clear() {
    #[cfg(target_arch = "wasm32")]
    ledger_storage::clear_storage();
}

// Serializable structs for JSON conversion
#[derive(Serialize)]
struct WasmLedgerBlockHeader {
    block_version: u32,
    jump_bytes_prev: i32,
    jump_bytes_next: u32,
    parent_block_hash: String,
    block_hash: String,
    offset: u64,
    fetch_compare_bytes: String,
    fetch_offset: u64,
    timestamp_ns: u64,
}

#[derive(Serialize)]
struct WasmLedgerBlockData {
    block_header: WasmLedgerBlockHeader,
    block: Vec<WasmLedgerEntry>,
}

// Parse input data and return serialized JSON
#[wasm_bindgen]
pub fn parse_ledger_blocks(
    input_data: Vec<u8>,
    input_data_start_offset: u64,
) -> Result<String, String> {
    LEDGER_MAP.with(|ledger| {
        let ledger = ledger.borrow();

        let mut result = Vec::new();
        for iter in ledger.iter_raw_from_slice(&input_data) {
            let (block_header, block, block_hash) = match iter {
                Ok(val) => val,
                Err(err) => {
                    warn!("Failed to parse ledger block: {}", err);
                    break;
                }
            };

            // Serialize block header with proper error handling
            let block_end_offset = block.get_offset() + block_header.jump_bytes_next_block() as u64;
            if block_end_offset > input_data.len() as u64 {
                info!(
                    "Block end offset {} beyond the end of the input data length {}",
                    block_end_offset,
                    input_data.len()
                );
                break;
            }
            let fetch_compare_bytes = if block_end_offset >= DATA_PULL_BYTES_BEFORE_LEN as u64 {
                BASE64.encode(
                    &input_data[(block_end_offset - DATA_PULL_BYTES_BEFORE_LEN as u64) as usize
                        ..block_end_offset as usize],
                )
            } else {
                "".to_string()
            };
            let offset = input_data_start_offset + block.get_offset();
            let block_header = WasmLedgerBlockHeader {
                block_version: block_header.block_version(),
                jump_bytes_prev: block_header.jump_bytes_prev_block(),
                jump_bytes_next: block_header.jump_bytes_next_block(),
                parent_block_hash: hex::encode(block.parent_hash()),
                block_hash: hex::encode(block_hash),
                offset,
                fetch_compare_bytes,
                fetch_offset: offset + block_header.jump_bytes_next_block() as u64,
                timestamp_ns: block.timestamp(),
            };

            // Parse entries and serialize the full block
            let entries = ledger_block_parse_entries(&block);

            result.push(WasmLedgerBlockData {
                block_header,
                block: entries,
            });
        }
        serde_json::to_string(&result).map_err(|e| e.to_string())
    })
}

#[wasm_bindgen]
pub fn ed25519_sign(private_key: Vec<u8>, data: Vec<u8>) -> Result<Vec<u8>, String> {
    let dcc_id = DccIdentity::new_signing_from_der(&private_key)?;
    dcc_id
        .sign(&data)
        .map(|sig| sig.to_vec())
        .map_err(|e| e.to_string())
}
