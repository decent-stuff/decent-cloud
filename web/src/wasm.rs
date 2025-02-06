use crate::{info, LedgerEntry, LedgerMap};
use dcc_common::{cursor_from_data, CursorDirection, LedgerCursor, DATA_PULL_BYTES_BEFORE_LEN};
use decent_cloud::ledger_canister_client::LedgerCanister;
use ic_agent::export::Principal;
use js_sys::{Array, Uint8Array};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmLedgerMap {
    inner: LedgerMap,
    canister: LedgerCanister,
}

#[wasm_bindgen]
pub struct WasmLedgerMapBlock {
    entries: Vec<LedgerEntry>,
    timestamp: u64,
    parent_hash: Vec<u8>,
}

#[wasm_bindgen]
pub struct WasmLedgerMapEntry {
    label: String,
    key: Vec<u8>,
    value: Vec<u8>,
    operation: String,
}

#[wasm_bindgen]
impl WasmLedgerMapBlock {
    #[wasm_bindgen(getter)]
    pub fn entries(&self) -> Array {
        let arr = Array::new();
        for entry in &self.entries {
            let wasm_entry = WasmLedgerMapEntry {
                label: entry.label().to_string(),
                key: entry.key().to_vec(),
                value: entry.value().to_vec(),
                operation: format!("{:?}", entry.operation()),
            };
            arr.push(&JsValue::from(wasm_entry));
        }
        arr
    }

    #[wasm_bindgen(getter)]
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    #[wasm_bindgen(getter)]
    pub fn parent_hash(&self) -> Uint8Array {
        Uint8Array::from(&self.parent_hash[..])
    }
}

#[wasm_bindgen]
impl WasmLedgerMapEntry {
    #[wasm_bindgen(getter)]
    pub fn label(&self) -> String {
        self.label.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn key(&self) -> Uint8Array {
        Uint8Array::from(&self.key[..])
    }

    #[wasm_bindgen(getter)]
    pub fn value(&self) -> Uint8Array {
        Uint8Array::from(&self.value[..])
    }

    #[wasm_bindgen(getter)]
    pub fn operation(&self) -> String {
        self.operation.clone()
    }
}

#[wasm_bindgen]
impl WasmLedgerMap {
    #[wasm_bindgen(constructor)]
    pub async fn new(labels_to_index: Option<Vec<String>>) -> Result<WasmLedgerMap, JsValue> {
        let inner =
            LedgerMap::new(labels_to_index).map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(WasmLedgerMap {
            inner,
            canister: LedgerCanister::new(
                Principal::from_text("ggi4a-wyaaa-aaaai-actqq-cai").unwrap(),
                None,
                "https://icp-api.io",
            )
            .await
            .unwrap(),
        })
    }

    pub fn upsert(&mut self, label: &str, key: &[u8], value: &[u8]) -> Result<(), JsValue> {
        self.inner
            .upsert(label, key.to_vec(), value.to_vec())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn get(&self, label: &str, key: &[u8]) -> Result<Vec<u8>, JsValue> {
        self.inner
            .get(label, key)
            .map(|v| v.clone())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn delete(&mut self, label: &str, key: &[u8]) -> Result<(), JsValue> {
        self.inner
            .delete(label, key)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn refresh(&mut self) -> Result<(), JsValue> {
        self.inner
            .refresh_ledger()
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn commit_block(&mut self) -> Result<(), JsValue> {
        self.inner
            .commit_block()
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn get_blocks_count(&self) -> usize {
        self.inner.get_blocks_count()
    }

    pub fn get_latest_block_hash(&self) -> Uint8Array {
        let hash = self.inner.get_latest_block_hash();
        Uint8Array::from(&hash[..])
    }

    pub fn get_latest_block_timestamp(&self) -> u64 {
        self.inner.get_latest_block_timestamp_ns()
    }

    pub fn get_block_entries(&self, label: Option<String>) -> Array {
        let entries: Vec<_> = self.inner.iter(label.as_deref()).collect();
        let arr = Array::new();
        for entry in entries {
            info!("entry: {:#?}", entry);
            let wasm_entry = WasmLedgerMapEntry {
                label: entry.label().to_string(),
                key: entry.key().to_vec(),
                value: entry.value().to_vec(),
                operation: format!("{:?}", entry.operation()),
            };
            arr.push(&JsValue::from(wasm_entry));
        }
        arr
    }

    pub fn get_next_block_entries(&self, label: Option<String>) -> Array {
        let entries: Vec<_> = self.inner.next_block_iter(label.as_deref()).collect();
        let arr = Array::new();
        for entry in entries {
            let wasm_entry = WasmLedgerMapEntry {
                label: entry.label().to_string(),
                key: entry.key().to_vec(),
                value: entry.value().to_vec(),
                operation: format!("{:?}", entry.operation()),
            };
            arr.push(&JsValue::from(wasm_entry));
        }
        arr
    }

    pub fn get_next_block_entries_count(&self, label: Option<String>) -> usize {
        self.inner.get_next_block_entries_count(label.as_deref())
    }

    pub fn get_persistent_storage_position(&self) -> u64 {
        let ledger_cursor_local = cursor_from_data(
            ledger_map::partition_table::get_data_partition().start_lba,
            ledger_map::platform_specific::persistent_storage_size_bytes(),
            self.inner.get_next_block_start_pos(),
            self.inner.get_next_block_start_pos(),
        );
        // ledger_cursor_local

        let bytes_before = if ledger_cursor_local.position > DATA_PULL_BYTES_BEFORE_LEN as u64 {
            let mut buf = vec![0u8; DATA_PULL_BYTES_BEFORE_LEN as usize];
            ledger_map::platform_specific::persistent_storage_read(
                ledger_cursor_local.position - DATA_PULL_BYTES_BEFORE_LEN as u64,
                &mut buf,
            )
            .expect("Failed to read from persistent storage");
            Some(buf)
        } else {
            None
        };

        info!(
            "Fetching data from the Ledger canister {}, with local cursor: {} and bytes before: {:?}",
            ledger_canister.canister_id(),
            ledger_cursor_local,
            hex::encode(bytes_before.as_ref().unwrap_or(&vec![])),
        );
        let (cursor_remote, data) = ledger_canister
            .data_fetch(Some(ledger_cursor_local.to_request_string()), bytes_before)
            .await?;
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
    }
}
