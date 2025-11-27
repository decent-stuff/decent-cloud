use crate::{cursor_from_data, LedgerCursor, DATA_PULL_BYTES_BEFORE_LEN};
use anyhow::Result;
use ledger_map::{platform_specific::persistent_storage_read, LedgerMap};
#[cfg(not(all(target_arch = "wasm32", feature = "ic")))]
use std::io::{Seek, Write};
#[cfg(not(all(target_arch = "wasm32", feature = "ic")))]
use std::mem::size_of;

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

fn select_fetch_start(local_next_block_start: u64, requested_position: u64) -> u64 {
    requested_position.min(local_next_block_start)
}

/// Fetch ledger data and write it to the ledger file
/// This function handles the core logic of fetching data from the canister
/// and writing it to the local ledger file, ensuring proper cursor handling
/// and file management.
pub async fn fetch_and_write_ledger_data<F, Fut>(
    ledger_map: &mut LedgerMap,
    data_fetch_fn: F,
    last_position: u64,
) -> Result<(Vec<u8>, u64)>
where
    F: FnOnce(Option<String>, Option<Vec<u8>>) -> Fut,
    Fut: std::future::Future<Output = Result<(String, Vec<u8>)>>,
{
    // Get the ledger file path if available - reborrow to get immutable access
    #[cfg(not(all(target_arch = "wasm32", feature = "ic")))]
    let ledger_file_path = (ledger_map as &LedgerMap).get_file_path();
    #[cfg(all(target_arch = "wasm32", feature = "ic"))]
    let ledger_file_path: Option<std::path::PathBuf> = None;

    let local_next_block = ledger_map.get_next_block_start_pos();
    let fetch_start = select_fetch_start(local_next_block, last_position);
    if fetch_start != last_position {
        tracing_info!(
            "Requested position {} is ahead of local ledger {}; using local cursor instead",
            last_position,
            local_next_block
        );
    }

    // Create proper cursor using the same approach as CLI
    let cursor_local = cursor_from_data(
        ledger_map::partition_table::get_data_partition().start_lba,
        ledger_map::platform_specific::persistent_storage_size_bytes(),
        local_next_block,
        fetch_start,
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
    let cursor_remote = LedgerCursor::new_from_string(new_cursor_str);
    let new_position = cursor_remote.position + cursor_remote.response_bytes;

    // Write the fetched data to the ledger file if we have a file path (same as CLI)
    if !raw_data.is_empty() {
        #[cfg(not(all(target_arch = "wasm32", feature = "ic")))]
        if let Some(ref path) = ledger_file_path {
            write_data_to_ledger_file(&raw_data, cursor_remote.position, path)?;
        }

        #[cfg(all(target_arch = "wasm32", feature = "ic"))]
        if ledger_file_path.is_none() {
            // For wasm/browser environments, write directly to persistent storage
            ledger_map::platform_specific::persistent_storage_write(
                cursor_remote.position,
                &raw_data,
            );
        }

        // Refresh the ledger parser after writing data (same as CLI)
        ledger_map.refresh_ledger()?;
    }

    Ok((raw_data, new_position))
}

/// Write data to the ledger file at the specified offset
/// This function handles the low-level file operations for writing ledger data
#[cfg(not(all(target_arch = "wasm32", feature = "ic")))]
pub fn write_data_to_ledger_file(
    data: &[u8],
    offset: u64,
    ledger_file_path: &std::path::Path,
) -> Result<()> {
    use std::fs::OpenOptions;

    let mut ledger_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(ledger_file_path)?;

    let file_size_bytes = ledger_file.metadata()?.len();
    let file_size_bytes_target = offset + data.len() as u64 + 1024 * 1024;

    if file_size_bytes < file_size_bytes_target {
        ledger_file.set_len(file_size_bytes_target)?;
    }

    ledger_file.seek(std::io::SeekFrom::Start(offset))?;
    ledger_file.write_all(data)?;

    // Write a zero block header as terminator (same as CLI)
    ledger_file.write_all(&[0u8; size_of::<ledger_map::ledger_entry::LedgerBlockHeader>()])?;

    tracing_info!(
        "Wrote {} bytes at offset 0x{:0x} to file {}",
        data.len(),
        offset,
        ledger_file_path.display()
    );

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetch_start_prefers_requested_when_not_ahead() {
        assert_eq!(select_fetch_start(512, 256), 256);
    }

    #[test]
    fn fetch_start_clamps_to_local_when_request_ahead() {
        assert_eq!(select_fetch_start(128, 256), 128);
    }
}
