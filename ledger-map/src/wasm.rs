use crate::{LedgerEntry, LedgerMap};
use js_sys::{Array, Uint8Array};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmLedgerMap {
    inner: LedgerMap,
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
    pub fn new(labels_to_index: Option<Vec<String>>) -> Result<WasmLedgerMap, JsValue> {
        let inner =
            LedgerMap::new(labels_to_index).map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(WasmLedgerMap { inner })
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

    pub fn get_latest_block_start_pos(&self) -> u64 {
        self.inner.get_latest_block_start_pos()
    }

    pub fn get_next_block_start_pos(&self) -> u64 {
        self.inner.get_next_block_start_pos()
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
}

#[cfg(test)]
#[path = "wasm_tests.rs"]
mod wasm_tests;
