use crate::platform_specific_wasm32_browser::{
    clear_ephemeral_storage, clear_storage, ensure_storage_is_initialized,
    init_ephemeral_storage_from_persistent, persist_last_block, persistent_storage_grow,
    persistent_storage_read, persistent_storage_size_bytes, persistent_storage_write,
    PERSISTENT_STORAGE_PAGE_SIZE,
};
use crate::wasm::WasmLedgerMap;
use js_sys::{Object, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::*;

// Configure tests to run in the browser.
wasm_bindgen_test_configure!(run_in_browser);

//
// Helper Functions
//

/// Creates a new ledger with two committed blocks and sample entries.
fn create_test_ledger() -> WasmLedgerMap {
    clear_storage();
    ensure_storage_is_initialized();
    let mut ledger = WasmLedgerMap::new(None).expect("Failed to create WasmLedgerMap");
    // Block 1: Two entries under different labels.
    ledger.upsert("label1", b"key1", b"value1").unwrap();
    ledger.upsert("label2", b"key2", b"value2").unwrap();
    ledger.commit_block().unwrap();
    // Block 2: One more entry for label1.
    ledger.upsert("label1", b"key3", b"value3").unwrap();
    ledger.commit_block().unwrap();
    ledger
}

//
// Persistent Storage Tests
//

#[wasm_bindgen_test]
fn test_clear_and_init_storage() {
    clear_storage();
    ensure_storage_is_initialized();
    // After clearing, the ephemeral storage should be empty.
    let size = persistent_storage_size_bytes();
    assert_eq!(
        size, 0,
        "Ephemeral storage should be empty after clear_storage()"
    );
}

#[wasm_bindgen_test]
fn test_persistent_storage_write_read() {
    clear_storage();
    ensure_storage_is_initialized();
    let data = b"Hello, Wasm!";
    persistent_storage_write(0, data);
    let mut buf = vec![0u8; data.len()];
    persistent_storage_read(0, &mut buf).unwrap();
    assert_eq!(&buf, data, "Data read should match data written");
}

#[wasm_bindgen_test]
fn test_persistent_storage_grow() {
    clear_storage();
    ensure_storage_is_initialized();
    // Write initial data.
    let data = b"Data";
    persistent_storage_write(0, data);
    let initial_size = persistent_storage_size_bytes();
    // Grow by 2 pages.
    persistent_storage_grow(2).unwrap();
    let new_size = persistent_storage_size_bytes();
    assert!(
        new_size >= initial_size + 2 * PERSISTENT_STORAGE_PAGE_SIZE,
        "Storage should grow by at least 2 pages"
    );
}

#[wasm_bindgen_test]
fn test_persist_last_block() {
    clear_storage();
    ensure_storage_is_initialized();
    let mut ledger = create_test_ledger();

    let block_start_pos = ledger.get_latest_block_start_pos();
    // Commit the last block of the ledger.
    persist_last_block(block_start_pos).unwrap();
    // Simulate a backup of the ephemeral data somewhere else (e.g. IndexedDB)
    let mut buf = vec![0u8; ledger.get_next_block_start_pos() as usize];
    persistent_storage_read(0, &mut buf).unwrap();

    // Simulate a new browser session.
    clear_ephemeral_storage();
    init_ephemeral_storage_from_persistent().unwrap();
    let err_msg = ledger.refresh().unwrap_err().as_string().unwrap();
    info!("Error message: {}", err_msg);
    assert!(err_msg
        .starts_with("Failed to read Ledger block: Requested data offset [8388608..8388624]"));
    // Simulate a reload.
    clear_ephemeral_storage();
    init_ephemeral_storage_from_persistent().unwrap();
    persistent_storage_write(0, &buf);
    ledger.refresh().unwrap();

    assert_eq!(ledger.get("label1", b"key1").unwrap(), b"value1".to_vec());
}

//
// Ledger (WasmLedgerMap) Tests
//

#[wasm_bindgen_test]
fn test_ledger_basic_operations() {
    clear_storage();
    ensure_storage_is_initialized();
    let mut ledger = WasmLedgerMap::new(None).expect("Failed to create WasmLedgerMap");
    let key = b"test_key".to_vec();
    let value = b"test_value".to_vec();
    ledger.upsert("test_label", &key, &value).unwrap();
    ledger.commit_block().unwrap();
    let retrieved = ledger.get("test_label", &key).unwrap();
    assert_eq!(retrieved, value, "Value should be retrieved correctly");
    // Test deletion.
    ledger.delete("test_label", &key).unwrap();
    ledger.commit_block().unwrap();
    assert!(
        ledger.get("test_label", &key).is_err(),
        "Key should be deleted"
    );
}

#[wasm_bindgen_test]
fn test_ledger_multiple_labels() {
    clear_storage();
    ensure_storage_is_initialized();
    let mut ledger = WasmLedgerMap::new(None).expect("Failed to create WasmLedgerMap");
    ledger.upsert("label1", b"key1", b"value1").unwrap();
    ledger.upsert("label2", b"key2", b"value2").unwrap();
    ledger.commit_block().unwrap();
    assert_eq!(
        ledger.get("label1", b"key1").unwrap(),
        b"value1".to_vec(),
        "Label1 data should match"
    );
    assert_eq!(
        ledger.get("label2", b"key2").unwrap(),
        b"value2".to_vec(),
        "Label2 data should match"
    );
}

#[wasm_bindgen_test]
fn test_ledger_block_operations() {
    clear_storage();
    ensure_storage_is_initialized();
    let mut ledger = WasmLedgerMap::new(None).expect("Failed to create WasmLedgerMap");
    ledger.upsert("test", b"block_test_key", b"value1").unwrap();
    ledger.commit_block().unwrap();
    assert_eq!(ledger.get_blocks_count(), 1, "Block count should be 1");
    assert_eq!(
        ledger.get("test", b"block_test_key").unwrap(),
        b"value1".to_vec()
    );
    ledger.upsert("test", b"block_test_key", b"value2").unwrap();
    ledger.commit_block().unwrap();
    assert_eq!(ledger.get_blocks_count(), 2, "Block count should be 2");
    assert_eq!(
        ledger.get("test", b"block_test_key").unwrap(),
        b"value2".to_vec()
    );
    let hash = ledger.get_latest_block_hash();
    assert!(hash.length() > 0, "Latest block hash should be non-empty");
}

#[wasm_bindgen_test]
fn test_ledger_refresh_persistence() {
    clear_storage();
    ensure_storage_is_initialized();
    let mut ledger = WasmLedgerMap::new(None).expect("Failed to create WasmLedgerMap");
    ledger
        .upsert("test", b"persist_key", b"persist_value")
        .unwrap();
    ledger.commit_block().unwrap();
    // Simulate a page reload.
    ledger.refresh().unwrap();
    assert_eq!(
        ledger.get("test", b"persist_key").unwrap(),
        b"persist_value".to_vec(),
        "Persisted value should be reloaded"
    );
}

#[wasm_bindgen_test]
fn test_ledger_entries() {
    let ledger = create_test_ledger();
    let entries = ledger.get_block_entries(None);
    assert_eq!(entries.length(), 3, "Should have 3 total entries");
    let label1_entries = ledger.get_block_entries(Some("label1".to_string()));
    assert_eq!(label1_entries.length(), 2, "Label1 should have 2 entries");
    let label2_entries = ledger.get_block_entries(Some("label2".to_string()));
    assert_eq!(label2_entries.length(), 1, "Label2 should have 1 entry");
    // Check that a sample entry has the expected properties.
    let entry = label2_entries.get(0).dyn_into::<JsValue>().unwrap();
    let entry_obj = Object::from(entry);
    assert!(
        Reflect::has(&entry_obj, &JsValue::from_str("label")).unwrap(),
        "Entry should have a 'label' property"
    );
    assert!(
        Reflect::has(&entry_obj, &JsValue::from_str("key")).unwrap(),
        "Entry should have a 'key' property"
    );
    assert!(
        Reflect::has(&entry_obj, &JsValue::from_str("value")).unwrap(),
        "Entry should have a 'value' property"
    );
    assert!(
        Reflect::has(&entry_obj, &JsValue::from_str("operation")).unwrap(),
        "Entry should have an 'operation' property"
    );
}

#[wasm_bindgen_test]
fn test_ledger_next_block_entries() {
    let mut ledger = create_test_ledger();
    ledger.upsert("label3", b"key4", b"value4").unwrap();
    ledger.upsert("label3", b"key5", b"value5").unwrap();
    assert_eq!(
        ledger.get_next_block_entries_count(None),
        2,
        "Next block should have 2 entries"
    );
    assert_eq!(
        ledger.get_next_block_entries_count(Some("label3".to_string())),
        2,
        "Label3 next block entries count should be 2"
    );
    let next_entries = ledger.get_next_block_entries(None);
    assert_eq!(
        next_entries.length(),
        2,
        "Next block entries length mismatch"
    );
    ledger.commit_block().unwrap();
    assert_eq!(
        ledger.get_next_block_entries_count(None),
        0,
        "After commit, next block entries should be cleared"
    );
}

#[wasm_bindgen_test]
fn test_ledger_metadata_accessors() {
    let ledger = create_test_ledger();
    // Verify that block metadata exists.
    assert_eq!(
        ledger.get_blocks_count(),
        2,
        "Should have 2 committed blocks"
    );
    let hash = ledger.get_latest_block_hash();
    assert!(hash.length() > 0, "Latest block hash should be available");
    let timestamp = ledger.get_latest_block_timestamp();
    assert!(timestamp > 0, "Latest block timestamp should be non-zero");
}
