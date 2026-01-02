use crate::{cursor_from_data, LedgerCursor, DATA_PULL_BYTES_BEFORE_LEN};
use anyhow::Result;
use ledger_map::{
    ledger_entry::LedgerBlockHeader,
    platform_specific::{persistent_storage_read, persistent_storage_write},
    LedgerMap,
};

#[cfg(all(target_arch = "wasm32", feature = "ic"))]
macro_rules! tracing_info {
    ($($arg:tt)*) => {
        ic_cdk::println!($($arg)*)
    };
}

#[cfg(all(target_arch = "wasm32", not(feature = "ic")))]
macro_rules! tracing_info {
    ($($arg:tt)*) => {{
        web_sys::console::info_1(&format!($($arg)*).into());
    }};
}

#[cfg(not(target_arch = "wasm32"))]
use tracing;
#[cfg(not(target_arch = "wasm32"))]
macro_rules! tracing_info {
    ($($arg:tt)*) => {
        tracing::info!($($arg)*)
    };
}

fn read_bytes_before(position: u64) -> Result<Vec<u8>> {
    let mut buf = vec![0u8; DATA_PULL_BYTES_BEFORE_LEN as usize];
    persistent_storage_read(position - DATA_PULL_BYTES_BEFORE_LEN as u64, &mut buf)
        .map_err(|e| anyhow::anyhow!("Failed to read persistent storage: {}", e))?;
    Ok(buf)
}

/// Fetch ledger data and write it to the ledger file
/// This function handles the core logic of fetching data from the canister
/// and writing it to the local ledger file, ensuring proper cursor handling
/// and file management.
///
/// Returns (raw_data, data_start_position, data_end_position)
pub async fn fetch_and_write_ledger_data<F, Fut>(
    ledger_map: &mut LedgerMap,
    data_fetch_fn: F,
    last_position: u64,
) -> Result<(Vec<u8>, u64, u64)>
where
    F: FnOnce(Option<String>, Option<Vec<u8>>) -> Fut,
    Fut: std::future::Future<Output = Result<(String, Vec<u8>)>>,
{
    // Use last_position directly - the remote will tell us where data actually starts
    // Don't clamp to local ledger state since we may be fetching new data
    let cursor_local = cursor_from_data(
        ledger_map::partition_table::get_data_partition().start_lba,
        ledger_map::platform_specific::persistent_storage_size_bytes(),
        last_position, // Use requested position, not ledger state
        last_position,
    );

    let bytes_before = if cursor_local.position > DATA_PULL_BYTES_BEFORE_LEN as u64 {
        Some(read_bytes_before(cursor_local.position)?)
    } else {
        None
    };

    // Use the cursor's request string format (same as CLI)
    let cursor_str = Some(cursor_local.to_request_string());
    tracing_info!("Fetching data from cursor: {}", cursor_str.clone().unwrap());

    let (new_cursor_str, raw_data) = data_fetch_fn(cursor_str, bytes_before).await?;
    tracing_info!(
        "Fetched {} bytes, new cursor: {}",
        raw_data.len(),
        new_cursor_str
    );

    // Parse the returned cursor to get the actual position (same as CLI)
    let cursor_remote = LedgerCursor::new_from_string(new_cursor_str)
        .map_err(|e| anyhow::anyhow!("Failed to parse remote cursor: {}", e))?;
    let data_start = cursor_remote.position;
    let data_end = cursor_remote.position + cursor_remote.response_bytes;

    // Write the fetched data using the same BackingFile handle as LedgerMap
    if !raw_data.is_empty() {
        // Write data to persistent storage (uses same file handle as LedgerMap)
        persistent_storage_write(data_start, &raw_data);

        // Write a zero block header as terminator
        let terminator = [0u8; LedgerBlockHeader::sizeof()];
        persistent_storage_write(data_start + raw_data.len() as u64, &terminator);

        tracing_info!(
            "Wrote {} bytes at offset 0x{:0x} to persistent storage (more={})",
            raw_data.len(),
            data_start,
            cursor_remote.more
        );

        // Only refresh when we have all data (no more chunks to fetch)
        // Refreshing with partial data fails because blocks may be truncated
        if !cursor_remote.more {
            ledger_map.refresh_ledger()?;
        }
    }

    Ok((raw_data, data_start, data_end))
}

/// Parse ledger entries from the file starting at the given position
pub fn parse_ledger_entries(
    ledger_map: &LedgerMap,
    start_pos: u64,
) -> Result<Vec<LedgerEntryData>> {
    let mut entries = Vec::new();

    for block_result in ledger_map.iter_raw(start_pos) {
        let (_block_header, block) = block_result?;
        let block_hash = LedgerMap::_compute_block_chain_hash(
            block.parent_hash(),
            block.entries(),
            block.timestamp(),
        )?;
        let block_timestamp = block.timestamp();
        let block_offset = block.get_offset();

        for entry in block.entries() {
            entries.push(LedgerEntryData {
                label: entry.label().to_string(),
                key: entry.key().to_vec(),
                value: entry.value().to_vec(),
                block_timestamp_ns: block_timestamp,
                block_hash: block_hash.clone(),
                block_offset,
            });
        }
    }

    Ok(entries)
}

/// Data structure for ledger entries that can be stored in the database
#[derive(Debug, Clone)]
pub struct LedgerEntryData {
    pub label: String,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub block_timestamp_ns: u64,
    pub block_hash: Vec<u8>,
    pub block_offset: u64,
}
