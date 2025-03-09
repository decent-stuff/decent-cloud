use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use candid::Encode;
use dcc_common::{
    cursor_from_data, ledger_block_parse_entries, refresh_caches_from_ledger, LedgerCursor,
    LedgerEntryAsJson, DATA_PULL_BYTES_BEFORE_LEN,
};
use js_sys::{Array, Reflect, Uint8Array};
#[cfg(not(target_arch = "wasm32"))]
use ledger_map::platform_specific as ledger_storage;
#[cfg(target_arch = "wasm32")]
use ledger_map::platform_specific_wasm32_browser as ledger_storage;
use ledger_map::{error, info, warn, LedgerMap};
use serde::Serialize;
use serde_json::Value;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

thread_local! {
    static LEDGER_MAP: RefCell<LedgerMap> = RefCell::new(LedgerMap::new(None).expect("Failed to create LedgerMap"));
}

#[wasm_bindgen(module = "/agent_js_wrapper.js")]
extern "C" {
    fn configure(config: JsValue);

    #[wasm_bindgen(catch)]
    async fn queryCanister(
        method_name: String,
        args: JsValue,
        options: JsValue,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    async fn fetchDataWithCache(
        cursor: String,
        bytes_before: Option<Vec<u8>>,
        bypass_cache: bool,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    async fn updateCanister(
        method_name: String,
        arg: JsValue,
        identity: JsValue,
        options: JsValue,
    ) -> Result<JsValue, JsValue>;
}

#[wasm_bindgen]
pub async fn initialize() {
    console_error_panic_hook::set_once();

    // Initialize storage as the very first thing
    #[cfg(target_arch = "wasm32")]
    ledger_storage::ensure_storage_is_initialized();

    // Extract the ledger data from thread-local storage
    let mut ledger_data = LEDGER_MAP.with(|ledger| std::mem::take(&mut *ledger.borrow_mut()));

    // Fetch ledger data with proper error handling
    match ledger_data_fetch(&mut ledger_data).await {
        Ok(_) => {
            let ledger_blocks = ledger_data.get_blocks_count();

            // Put the updated ledger data back
            LEDGER_MAP.with(|ledger| {
                *ledger.borrow_mut() = ledger_data;
            });

            info!(
                "Ledger initialized successfully, loaded {} blocks",
                ledger_blocks
            )
        }
        Err(e) => {
            // Still put the ledger data back even if there was an error
            LEDGER_MAP.with(|ledger| {
                *ledger.borrow_mut() = ledger_data;
            });

            error!("Ledger initialization error: {}", e)
        }
    }
}

#[wasm_bindgen]
pub fn ledger_storage_clear() {
    #[cfg(target_arch = "wasm32")]
    ledger_storage::clear_storage();
}

#[wasm_bindgen]
pub fn ledger_storage_size_bytes() -> u64 {
    ledger_storage::persistent_storage_size_bytes()
}

#[wasm_bindgen]
pub fn ledger_storage_read_offset(offset: u64, len: u64) -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; len as usize];
    match ledger_storage::persistent_storage_read(offset, &mut buf) {
        Ok(_) => Ok(buf),
        Err(e) => Err(format!(
            "Failed to read storage at offset {}: {}",
            offset, e
        )),
    }
}

#[wasm_bindgen]
pub fn ledger_storage_write_offset(offset: u64, data: &[u8]) {
    ledger_storage::persistent_storage_write(offset, data);
}

pub async fn ledger_data_fetch(
    ledger_local: &mut LedgerMap,
) -> Result<(), Box<dyn std::error::Error>> {
    // FIXME: needs to be adjusted to fetch data multiple times if needed, right now it only does it once
    let cursor_local = {
        let size_bytes = ledger_map::platform_specific::persistent_storage_last_valid_offset();
        info!("Persistent storage size: {}", size_bytes);
        cursor_from_data(
            ledger_map::partition_table::get_data_partition().start_lba,
            size_bytes,
            ledger_local.get_next_block_start_pos(),
            ledger_local.get_next_block_start_pos(),
        )
    };

    let bytes_before = if cursor_local.position > DATA_PULL_BYTES_BEFORE_LEN as u64 {
        let mut buf = vec![0u8; DATA_PULL_BYTES_BEFORE_LEN as usize];
        ledger_storage::persistent_storage_read(
            cursor_local.position - DATA_PULL_BYTES_BEFORE_LEN as u64,
            &mut buf,
        )?;
        Some(buf)
    } else {
        None
    };

    info!(
        "Fetching data from the Ledger canister, with local cursor: {} and bytes before: {:?}",
        cursor_local,
        hex::encode(bytes_before.as_ref().unwrap_or(&vec![])),
    );
    // Use proper error handling for fetchDataWithCache
    let result_js = match fetchDataWithCache(
        cursor_local.to_request_string(),
        bytes_before,
        false, // Don't bypass cache by default
    )
    .await
    {
        Ok(result) => {
            info!("Success fetchDataWithCache: {:?}", result);
            result
        }
        Err(e) => {
            warn!("Failed to fetch data: {:?}", e);
            return Err(format!("Failed to fetch data from canister: {:?}", e).into());
        }
    };

    info!("Result from fetchDataWithCache: {:?}", result_js);

    // Extract the "Ok" property from the returned object with proper error handling
    let ok_js = {
        match Reflect::get(&result_js, &JsValue::from_str("Ok")) {
            Ok(js_value) => {
                if js_value.is_undefined() || js_value.is_null() {
                    info!("'Ok' property is undefined or null");
                    return Err("Response 'Ok' property is undefined or null".into());
                }
                js_value
            }
            Err(e) => {
                info!("Failed to extract 'Ok' property from result: {:?}", e);
                return Err("Invalid response format from canister".into());
            }
        }
    };

    // Convert the Ok property to an array.
    let ok_array = Array::from(&ok_js);

    // The first element is the cursor string.
    // Safely extract the cursor string with better error handling
    let cursor_remote = {
        let cursor_js = ok_array.get(0);
        if cursor_js.is_undefined() || cursor_js.is_null() {
            info!("Cursor is undefined or null");
            return Err("Cursor is undefined or null in canister response".into());
        }

        match cursor_js.as_string() {
            Some(cursor) => {
                if cursor.is_empty() {
                    info!("Empty cursor string received");
                    return Err("Empty cursor string received from canister".into());
                }
                cursor
            }
            None => {
                info!("Invalid cursor format received from canister, not a string");
                return Err("Invalid cursor format received from canister - not a string".into());
            }
        }
    };

    // The second element is the data as a Uint8Array.
    let data_js = ok_array.get(1);

    // Validate that we actually got data
    if data_js.is_undefined() || data_js.is_null() {
        info!("No data received from canister");
        return Err("No data received from canister".into());
    }

    // More robust handling of data conversion from JS to Rust
    let data = {
        // Check if it's actually a Uint8Array or array-like
        if !js_sys::ArrayBuffer::instanceof(&data_js)
            && !js_sys::Array::instanceof(&data_js)
            && !Uint8Array::instanceof(&data_js)
        {
            info!(
                "Data is not ArrayBuffer, Array, or Uint8Array: {:?}",
                data_js
            );
            return Err("Invalid data format from canister - not binary data".into());
        }

        // Use a try-catch pattern since to_vec() can panic with malformed data
        let data_u8array = Uint8Array::new(&data_js);

        if data_u8array.length() == 0 {
            info!("Received empty Uint8Array data");
            // Return empty vec instead of error - this is valid
            vec![]
        } else {
            // Convert to Rust Vec<u8>
            data_u8array.to_vec()
        }
    };

    info!("Ledger canister returned {} bytes", data.len());

    // Create a LedgerCursor from the string with proper error handling
    // The new_from_string method calls unwrap() internally which can panic
    let cursor_remote = match cursor_remote.parse::<LedgerCursor>() {
        Ok(cursor) => {
            // Additional validations for cursor
            if cursor.position > u64::MAX / 2 {
                info!("Suspiciously large cursor position: {}", cursor.position);
                return Err(
                    format!("Suspiciously large cursor position: {}", cursor.position).into(),
                );
            }
            cursor
        }
        Err(e) => {
            info!("Failed to parse cursor string: {}", e);
            return Err(format!("Failed to parse cursor string: {}", e).into());
        }
    };

    // Add validation to ensure the cursor is valid
    if cursor_remote.position == 0 && !data.is_empty() {
        info!("Invalid cursor position: 0 with non-empty data");
        return Err("Invalid cursor position received from canister".into());
    }

    let offset_remote = cursor_remote.position;
    info!(
        "Ledger canister returned position {:0x}, full cursor: {}",
        offset_remote, cursor_remote
    );
    if offset_remote < cursor_local.position {
        return Err(format!(
            "Ledger canister has less data than available locally {} < {} bytes",
            offset_remote, cursor_local.position
        )
        .into());
    }
    if data.len() <= 64 {
        info!("Data: {} bytes ==> {:?}", data.len(), data);
    } else {
        // Improved data logging for better diagnostics
        info!(
            "Data: {} bytes ==> start:{:?}... end:{:?}",
            data.len(),
            &data[..32.min(data.len())],
            &data[(data.len() - 32.min(data.len()))..]
        );
    }

    // Make the storage writes safer
    if !data.is_empty() {
        // Write data with additional error logging
        info!(
            "Writing {} bytes to storage at offset {}",
            data.len(),
            offset_remote
        );
        ledger_storage::persistent_storage_write(offset_remote, &data);

        // Create a zero buffer with proper size handling
        use std::mem::size_of;
        let header_size = size_of::<ledger_map::ledger_entry::LedgerBlockHeader>();
        info!(
            "Writing {} zero bytes for header at offset {}",
            header_size,
            offset_remote + data.len() as u64
        );
        let zero_buffer = vec![0u8; header_size];

        ledger_storage::persistent_storage_write(offset_remote + data.len() as u64, &zero_buffer);
    } else {
        info!("Skipping storage write for empty data");
    }

    if !data.is_empty() {
        // TODO: All ledger blocks are effectively iterated twice here, it should be possible to do this in a single go
        ledger_local.refresh_ledger()?;

        // Add proper error handling for refresh_caches_from_ledger
        match refresh_caches_from_ledger(ledger_local) {
            Ok(_) => info!("Successfully refreshed caches from ledger"),
            Err(e) => {
                info!("Warning: Failed to refresh caches from ledger: {}", e);
                // We don't return an error here as the main operation succeeded
                // Just log the warning and continue
            }
        }
    }

    Ok(())
}

#[wasm_bindgen]
pub fn ledger_cursor_local_as_str() -> String {
    let cursor_local = LEDGER_MAP.with(|ledger| {
        let ledger = ledger.borrow();
        cursor_from_data(
            ledger_map::partition_table::get_data_partition().start_lba,
            ledger_map::platform_specific::persistent_storage_size_bytes(),
            ledger.get_next_block_start_pos(),
            ledger.get_next_block_start_pos(),
        )
    });

    cursor_local.to_string()
}

#[wasm_bindgen]
pub fn ledger_get_block_as_json(block_offset: u64) -> Result<String, String> {
    LEDGER_MAP.with(|ledger| {
        let ledger = ledger.borrow();

        // Get block with proper error handling
        let (block_header, block) = match ledger.get_block_at_offset(block_offset) {
            Ok(result) => result,
            Err(e) => {
                return Err(format!(
                    "Failed to get block at offset {}: {}",
                    block_offset, e
                ))
            }
        };

        #[derive(Serialize)]
        struct LedgerBlockHeaderAsJson {
            block_version: u32,
            jump_bytes_prev: i32,
            jump_bytes_next: u32,
            parent_block_hash: String,
            offset: u64,
            timestamp_ns: u64,
        }

        #[derive(Serialize)]
        struct LedgerBlockAsJson {
            block_header: Value,
            block: Vec<LedgerEntryAsJson>,
        }

        // Serialize block header with proper error handling
        let header_json = match serde_json::to_value(&LedgerBlockHeaderAsJson {
            block_version: block_header.block_version(),
            jump_bytes_prev: block_header.jump_bytes_prev_block(),
            jump_bytes_next: block_header.jump_bytes_next_block(),
            parent_block_hash: BASE64.encode(block.parent_hash()),
            offset: block.get_offset(),
            timestamp_ns: block.timestamp(),
        }) {
            Ok(json) => json,
            Err(e) => return Err(format!("Failed to serialize block header: {}", e)),
        };

        // Parse entries and serialize the full block
        let entries = ledger_block_parse_entries(&block);

        serde_json::to_string(&LedgerBlockAsJson {
            block_header: header_json,
            block: entries,
        })
        .map_err(|e| format!("Failed to serialize block: {}", e))
    })
}

// Function to expose LedgerMap functionality
#[wasm_bindgen]
pub fn ledger_get_value(label: &str, key: &[u8]) -> Result<Vec<u8>, String> {
    Ok(LEDGER_MAP.with(|ledger| ledger.borrow().get(label, key))?)
}

#[wasm_bindgen]
pub fn ledger_set_value(label: &str, key: &[u8], value: &[u8]) -> Result<(), String> {
    Ok(LEDGER_MAP.with(|ledger| ledger.borrow_mut().upsert(label, key, value))?)
}

#[wasm_bindgen]
pub fn ledger_remove_value(label: &str, key: &[u8]) -> Result<(), String> {
    Ok(LEDGER_MAP.with(|ledger| ledger.borrow_mut().delete(label, key))?)
}

/// Generic query function that can be used for any query method
pub async fn call_query_canister(method_name: &str, args: &[u8]) -> Result<Vec<u8>, JsValue> {
    // Convert the slice of bytes into a Uint8Array JsValue.
    let args_jsvalue = Uint8Array::from(args).into();

    let result_js = queryCanister(method_name.to_string(), args_jsvalue, JsValue::null()).await?;
    let result = Uint8Array::new(&result_js).to_vec();
    Ok(result)
}

/// Generic update function that can be used for any update method
pub async fn call_update_canister(
    method_name: &str,
    arg: JsValue,
    identity: JsValue,
) -> Result<JsValue, JsValue> {
    updateCanister(method_name.to_string(), arg, identity, JsValue::null()).await
}

#[wasm_bindgen]
pub async fn get_transactions() -> Result<Vec<u8>, JsValue> {
    let empty_args = Encode!(&()).map_err(|e| e.to_string())?;
    call_query_canister("get_transactions", &empty_args).await
}
