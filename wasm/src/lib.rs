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
use ledger_map::{info, ledger_entry::LedgerBlockHeader, LedgerMap};
use serde::Serialize;
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
pub async fn initialize() -> String {
    console_error_panic_hook::set_once();
    // Initialize storage as the very first thing
    #[cfg(target_arch = "wasm32")]
    ledger_storage::ensure_storage_is_initialized();
    // ledger_storage::init_ephemeral_storage_from_persistent();
    // Fetch Ledger data
    // Extract the ledger data from thread-local storage.
    // This requires that LedgerMap implements Default (or that you can otherwise replace it).
    let mut ledger_data = LEDGER_MAP.with(|ledger| std::mem::take(&mut *ledger.borrow_mut()));
    // Now call your async function on the extracted ledger.
    ledger_data_fetch(&mut ledger_data).await.unwrap();

    let ledger_blocks = ledger_data.get_blocks_count();
    // Put the updated ledger data back.
    LEDGER_MAP.with(|ledger| {
        *ledger.borrow_mut() = ledger_data;
    });
    format!(
        "Ledger initialized successfully, loaded {} blocks",
        ledger_blocks
    )
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
pub fn ledger_storage_read_offset(offset: u64, len: u64) -> Vec<u8> {
    let mut buf = vec![0u8; len as usize];
    ledger_storage::persistent_storage_read(offset, &mut buf).expect("Failed to read storage");
    buf
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
    let result_js = fetchDataWithCache(
        cursor_local.to_request_string(),
        bytes_before,
        false, // Don't bypass cache by default
    )
    .await
    .expect("Failed to fetch data");

    // Extract the "Ok" property from the returned object.
    let ok_js = Reflect::get(&result_js, &JsValue::from_str("Ok"))
        .expect("Expected an 'Ok' property in the result");

    // Convert the Ok property to an array.
    let ok_array = Array::from(&ok_js);

    // The first element is the cursor string.
    let cursor_remote = ok_array
        .get(0)
        .as_string()
        .expect("Expected the first element to be a string");

    // The second element is the data as a Uint8Array.
    let data_js = ok_array.get(1);
    let data_u8array = Uint8Array::new(&data_js);
    let data = data_u8array.to_vec();

    info!("Ledger canister returned {} bytes", data.len());
    let cursor_remote = LedgerCursor::new_from_string(cursor_remote);
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
        info!(
            "Data: {} bytes ==> {:?}...",
            data.len(),
            &data[..64.min(data.len())]
        );
    }
    ledger_storage::persistent_storage_write(offset_remote, &data);
    ledger_storage::persistent_storage_write(
        offset_remote + data.len() as u64,
        &[0u8; size_of::<ledger_map::ledger_entry::LedgerBlockHeader>()],
    );

    if !data.is_empty() {
        // TODO: All ledger blocks are effectively iterated twice here, it should be possible to do this in a single go
        ledger_local.refresh_ledger()?;
        refresh_caches_from_ledger(ledger_local).unwrap();
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
        let (block_header, block) = ledger.get_block_at_offset(block_offset)?;

        #[derive(Serialize)]
        struct LedgerBlockHeaderAsJson {
            #[serde(flatten)]
            block_header: LedgerBlockHeader,
            parent_block_hash: String,
            offset: u64,
        }

        #[derive(Serialize)]
        struct LedgerBlockAsJson {
            block_header: String,
            block: Vec<LedgerEntryAsJson>,
        }

        serde_json::to_string(&LedgerBlockAsJson {
            block_header: serde_json::to_string(&LedgerBlockHeaderAsJson {
                block_header,
                parent_block_hash: BASE64.encode(block.parent_hash()),
                offset: block.get_offset(),
            })
            .unwrap(),
            block: ledger_block_parse_entries(&block),
        })
        .map_err(|e| e.to_string())
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
