use anyhow::Result;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use js_sys::Error;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast; // for `dyn_ref`
use web_sys::Storage;

/// The way storage in browsers works is the following:
/// - In browsers, local storage is limited to around 5MB.
///   See: https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria#web_storage
/// - Although IndexedDB can store more, it's asynchronous and not suitable for the current LedgerMap implementation.
/// - We store the last ledger block (or data) in persistent storage (browser's local storage),
///   which is sufficient for verifying ledger integrity.
/// - The entire ledger is held in ephemeral (in‑memory) storage, which is populated by JavaScript on page load.
/// - Changes to the ledger in ephemeral storage must be explicitly committed to persistent storage if needed.
///
/// Note that the ephemeral storage can be much larger than the last block. In such cases, only the last block should be
/// persisted. The logic to determine the start of the last block is not known at this level, so the function
/// `persist_last_block(block_start: u64)` is provided for higher-level callers to specify which part of the ledger
/// constitutes the "last block."

/// We store the "last block" (or relevant ledger data) in local storage using this key.
const PERSISTENT_STORAGE_DATA_KEY: &str = "ledger_map_last_block";

/// We store the offset of the last block in local storage under this key.
const PERSISTENT_STORAGE_OFFSET_KEY: &str = "ledger_map_last_block_offset";

thread_local! {
    /// Ephemeral (in‑memory) ledger data. May be larger than what we persist.
    static EPHEMERAL_STORAGE: RefCell<Vec<u8>> = RefCell::new(Vec::new());

    /// The beginning offset of valid data in `EPHEMERAL_STORAGE`.
    static EPHEMERAL_STORAGE_VALID_BEGIN: RefCell<u64> = RefCell::new(0);

    /// The end offset of valid data in `EPHEMERAL_STORAGE` (one past the last valid byte).
    static EPHEMERAL_STORAGE_VALID_END: RefCell<u64> = RefCell::new(0);

    /// Browser local storage handle, if available.
    /// If multi-threading is introduced in the future, you may need to synchronize access here.
    static PERSISTENT_LOCAL_STORAGE: RefCell<Option<Storage>> = RefCell::new(None);
}

//-------------------------------------
// Re-export macros for easy logging
//-------------------------------------
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        web_sys::console::debug_1(&format!($($arg)*).into());
    }};
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        web_sys::console::info_1(&format!($($arg)*).into());
    }};
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        web_sys::console::warn_1(&format!($($arg)*).into());
    }};
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        web_sys::console::error_1(&format!($($arg)*).into());
    }};
}

/// Public convenience for debug logging
pub fn export_debug(msg: &str) {
    debug!("{}", msg);
}

/// Public convenience for info logging
pub fn export_info(msg: &str) {
    info!("{}", msg);
}

/// Public convenience for warning logging
pub fn export_warn(msg: &str) {
    warn!("{}", msg);
}

/// Public convenience for error logging
pub fn export_error(msg: &str) {
    error!("{}", msg);
}

fn is_storage_initialized() -> bool {
    PERSISTENT_LOCAL_STORAGE.with(|ls| ls.borrow().is_some())
}

/// Ensures that local storage is initialized. Called automatically at startup.
///
/// If you run in a browser environment with default WASM, this function
/// is single-threaded, so no concurrency concerns arise here.
#[wasm_bindgen(start)]
pub fn ensure_storage_is_initialized() {
    if is_storage_initialized() {
        return;
    }

    let window = web_sys::window().expect("no global window exists");
    let storage = window
        .local_storage()
        .expect("no local storage exists")
        .expect("failed to get local storage");

    PERSISTENT_LOCAL_STORAGE.with(|ls| {
        *ls.borrow_mut() = Some(storage);
        info!("Persistent storage initialized successfully");
    })
}

/// Clears both browser local storage and the in-memory (ephemeral) storage.
#[wasm_bindgen]
pub fn clear_storage() {
    let window = web_sys::window().expect("no global window exists");
    let storage = window
        .local_storage()
        .expect("no local storage exists")
        .expect("failed to get local storage");
    storage.clear().expect("failed to clear local storage");

    PERSISTENT_LOCAL_STORAGE.with(|ls| {
        *ls.borrow_mut() = None;
    });

    clear_ephemeral_storage();
}

/// Clears the in-memory (ephemeral), simulating a new browser session.
#[wasm_bindgen]
pub fn clear_ephemeral_storage() {
    EPHEMERAL_STORAGE.with(|es| {
        *es.borrow_mut() = Vec::new();
    });
    EPHEMERAL_STORAGE_VALID_BEGIN.with(|b| *b.borrow_mut() = 0);
    EPHEMERAL_STORAGE_VALID_END.with(|e| *e.borrow_mut() = 0);
}

/// Reads data from ephemeral storage only.
/// If the requested range is within the valid region of ephemeral storage,
/// the data is copied into `buf`. Otherwise, an error is returned.
pub fn persistent_storage_read(offset: u64, buf: &mut [u8]) -> Result<(), String> {
    let valid_begin = EPHEMERAL_STORAGE_VALID_BEGIN.with(|b| *b.borrow());
    let valid_end = EPHEMERAL_STORAGE_VALID_END.with(|e| *e.borrow());

    if offset < valid_begin || offset + buf.len() as u64 > valid_end {
        return Err(format!(
            "Requested data offset [{}..{}] is not available in ephemeral storage [{}..{}]",
            offset,
            offset + buf.len() as u64,
            valid_begin,
            valid_end
        )
        .to_string());
    }

    EPHEMERAL_STORAGE.with(|es| {
        let storage = es.borrow();
        // debug!(
        //     "persistent_storage_read: offset: {}, len: {} from storage with len {}",
        //     offset,
        //     buf.len(),
        //     storage.len()
        // );
        buf.copy_from_slice(&storage[offset as usize..(offset as usize + buf.len())]);
    });
    Ok(())
}

/// Updates ephemeral storage with new data starting at `offset`.
/// Resizes ephemeral storage if needed and updates the valid region.
/// This function does NOT persist the data to browser local storage.
/// To persist the latest block, call `persist_last_block`.
pub fn persistent_storage_write(offset: u64, buf: &[u8]) {
    EPHEMERAL_STORAGE.with(|es| {
        let mut storage = es.borrow_mut();
        let current_len = storage.len() as u64;
        let new_end = offset + buf.len() as u64;

        if new_end > current_len {
            storage.resize(new_end as usize, 0);
        }

        // debug!(
        //     "persistent_storage_write: offset: {}, len: {}",
        //     offset,
        //     buf.len()
        // );
        storage[offset as usize..(offset as usize + buf.len())].copy_from_slice(buf);

        let valid_begin = EPHEMERAL_STORAGE_VALID_BEGIN.with(|b| *b.borrow());
        if offset < valid_begin {
            EPHEMERAL_STORAGE_VALID_BEGIN.with(|b| *b.borrow_mut() = offset);
        }
        EPHEMERAL_STORAGE_VALID_END.with(|e| {
            let mut valid = e.borrow_mut();
            if new_end > *valid {
                *valid = new_end;
            }
        });
    });
}

pub const PERSISTENT_STORAGE_PAGE_SIZE: u64 = 64 * 1024;

pub fn persistent_storage_grow(additional_pages: u64) -> Result<u64, String> {
    debug!(
        "persistent_storage_grow: {} additional_pages.",
        additional_pages
    );
    let prev_size = persistent_storage_size_bytes();
    EPHEMERAL_STORAGE.with(|es| {
        let mut storage = es.borrow_mut();
        if additional_pages > 0 {
            let additional_bytes = additional_pages * PERSISTENT_STORAGE_PAGE_SIZE;
            let new_size = storage.len() + additional_bytes as usize;
            storage.resize(new_size, 0); // Fill new space with zeros
        }
        Ok(prev_size)
    })
}

/// Returns the current length of the ephemeral storage buffer (in bytes).
pub fn persistent_storage_size_bytes() -> u64 {
    EPHEMERAL_STORAGE.with(|es| es.borrow().len() as u64)
}

/// Returns the last valid offset in the ephemeral storage buffer.
pub fn persistent_storage_last_valid_offset() -> u64 {
    EPHEMERAL_STORAGE_VALID_END.with(|e| *e.borrow())
}

/// Initializes ephemeral storage from data in local storage (if it exists).
/// If nothing is found in persistent storage, ephemeral storage is set to empty,
/// and valid offsets are set to 0.
pub fn init_ephemeral_storage_from_persistent() -> Result<(), String> {
    info!("Initializing ephemeral storage from persistent storage.");
    let (persistent_data, persistent_offset) = PERSISTENT_LOCAL_STORAGE.with(|ls| {
        if let Some(storage) = &*ls.borrow() {
            (
                storage.get_item(PERSISTENT_STORAGE_DATA_KEY).ok().flatten(),
                storage
                    .get_item(PERSISTENT_STORAGE_OFFSET_KEY)
                    .ok()
                    .flatten(),
            )
        } else {
            (None, None)
        }
    });

    match (persistent_data, persistent_offset) {
        (Some(data), Some(offset)) => {
            let decoded = decode_bytes(&data);
            if decoded.is_empty() {
                error!("Persistent ledger data was corrupted or invalid base64; resetting ephemeral storage.");
                report_and_recover_corrupted_ledger();
            } else {
                let offset: u64 = match offset.parse() {
                    Ok(o) => o,
                    Err(e) => {
                        error!(
                            "Persistent ledger offset was corrupted; resetting ephemeral storage."
                        );
                        report_and_recover_corrupted_ledger();
                        return Err(e.to_string());
                    }
                };
                let valid_end = offset as usize + decoded.len();
                EPHEMERAL_STORAGE.with(|es| {
                    let mut es = es.borrow_mut();
                    es.resize(valid_end, 0);
                    es[offset as usize..].copy_from_slice(&decoded);
                });
                EPHEMERAL_STORAGE_VALID_BEGIN.with(|b| *b.borrow_mut() = offset);
                EPHEMERAL_STORAGE_VALID_END.with(|e| *e.borrow_mut() = valid_end as u64);
            }
        }
        _ => {
            warn!("No persistent storage data found; initializing ephemeral storage.");
            // Initialize everything empty
            EPHEMERAL_STORAGE.with(|es| {
                *es.borrow_mut() = Vec::new();
            });
            EPHEMERAL_STORAGE_VALID_BEGIN.with(|b| *b.borrow_mut() = 0);
            EPHEMERAL_STORAGE_VALID_END.with(|e| *e.borrow_mut() = 0);
        }
    }
    Ok(())
}

/// Demonstrates how you might handle a corrupted ledger: we clear and reset ephemeral data
/// and log a warning. This is called automatically if we decode an empty vector from an
/// unexpected base64. In production, you might handle it differently based on your needs.
fn report_and_recover_corrupted_ledger() {
    warn!("Recovering from corrupted ledger data... clearing ephemeral ledger.");
    EPHEMERAL_STORAGE.with(|es| {
        es.borrow_mut().clear();
    });
    EPHEMERAL_STORAGE_VALID_BEGIN.with(|b| *b.borrow_mut() = 0);
    EPHEMERAL_STORAGE_VALID_END.with(|e| *e.borrow_mut() = 0);
}

/// Persists the last block of the ledger (from `block_start` to the end of ephemeral storage)
/// in the browser local storage. Overwrites any previous ledger data in local storage.
pub fn persist_last_block(block_start: u64) -> Result<(), String> {
    EPHEMERAL_STORAGE.with(|es| {
        let storage = es.borrow();
        info!(
            "Persisting block of data in BROWSER LOCAL STORAGE: [{}..{}]",
            block_start,
            storage.len()
        );
        if block_start as usize > storage.len() {
            return Err(format!(
                "block_start {} is beyond ephemeral storage length {}",
                block_start,
                storage.len()
            ));
        }
        let last_block = &storage[block_start as usize..];
        let encoded = encode_bytes(last_block);
        let last_block_offset_str = format!("{}", block_start);

        PERSISTENT_LOCAL_STORAGE.with(|ls| {
            if let Some(storage) = &*ls.borrow() {
                write_with_quota_check(storage, PERSISTENT_STORAGE_DATA_KEY, &encoded)?;
                write_with_quota_check(
                    storage,
                    PERSISTENT_STORAGE_OFFSET_KEY,
                    &last_block_offset_str,
                )
            } else {
                Err("Persistent local storage not initialized".into())
            }
        })
    })
}

//-------------------------------------
// Internal Utility Functions
//-------------------------------------

/// Writes to `localStorage` and checks for quota errors or other exceptions,
/// returning a descriptive error if something goes wrong.
fn write_with_quota_check(storage: &Storage, key: &str, value: &str) -> Result<(), String> {
    match storage.set_item(key, value) {
        Ok(_) => Ok(()),
        Err(e) => {
            // Try casting the error to a js_sys::Error and check its name property.
            if let Some(js_error) = e.dyn_ref::<Error>() {
                if js_error.name() == "QuotaExceededError" {
                    return Err(format!(
                        "Browser storage quota exceeded when writing key '{}'",
                        key
                    ));
                }
            }
            // Otherwise, it's a different error
            Err(format!(
                "Failed to write key '{}' to local storage: {:?}",
                key, e
            ))
        }
    }
}

/// Encodes a slice of bytes into a base64 string.
fn encode_bytes(bytes: &[u8]) -> String {
    BASE64.encode(bytes)
}

/// Decodes a base64 string into bytes.
/// Returns an empty vector on error and logs it, which triggers a reset
/// in ledger initialization logic if relevant.
fn decode_bytes(data: &str) -> Vec<u8> {
    BASE64.decode(data).unwrap_or_else(|e| {
        error!("Failed to decode base64 data: {:?}", e);
        Vec::new()
    })
}

/// No-op in browsers. Could be used in other environments (e.g., Node with fs).
pub fn set_backing_file(_path: Option<std::path::PathBuf>) -> Result<(), String> {
    Ok(())
}

/// Always `None` in browsers since we have no real file path here.
pub fn get_backing_file_path() -> Option<std::path::PathBuf> {
    None
}

/// Returns a timestamp in nanoseconds, derived from the browser's high-resolution timer.
pub fn get_timestamp_nanos() -> u64 {
    (js_sys::Date::now() * 1_000_000.0) as u64
}
