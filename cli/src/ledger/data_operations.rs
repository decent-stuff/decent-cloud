use candid::{Decode, Encode};
use dcc_common::{cursor_from_data, fetch_and_write_ledger_data, CursorDirection, LedgerCursor};
use decent_cloud::ledger_canister_client::LedgerCanister;
use ledger_map::platform_specific::persistent_storage_read;
use ledger_map::LedgerMap;
use log::info;
use std::path::PathBuf;

use super::get_ledger_metadata;

const PUSH_BLOCK_SIZE: u64 = 1024 * 1024;

pub async fn ledger_data_fetch(
    ledger_canister: &LedgerCanister,
    ledger_local: &mut LedgerMap,
) -> Result<(), Box<dyn std::error::Error>> {
    let last_position = ledger_local.get_next_block_start_pos();

    let (data, _new_position) = fetch_and_write_ledger_data(
        ledger_local,
        |cursor_str, bytes_before| async move {
            ledger_canister
                .data_fetch(cursor_str, bytes_before)
                .await
                .map_err(|e| anyhow::anyhow!(e))
        },
        last_position,
    )
    .await?;

    if !data.is_empty() {
        // Set the modified time to the current time, to mark that the data is up-to-date
        if let Some(ledger_file_path) = ledger_local.get_file_path() {
            filetime::set_file_mtime(ledger_file_path, std::time::SystemTime::now().into())?;
        }
    }

    Ok(())
}

pub async fn ledger_data_push(
    ledger_canister: &LedgerCanister,
    local_ledger_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let ledger_local = LedgerMap::new_with_path(Some(vec![]), Some(local_ledger_path))
        .expect("Failed to create LedgerMap");
    let cursor_local = cursor_from_data(
        ledger_map::partition_table::get_data_partition().start_lba,
        ledger_map::platform_specific::persistent_storage_size_bytes(),
        ledger_local.get_next_block_start_pos(),
        ledger_local.get_next_block_start_pos(),
    );

    let remote_metadata = get_ledger_metadata(ledger_canister).await;
    let cursor_remote: LedgerCursor = remote_metadata.into();

    if cursor_local.data_end_position <= cursor_remote.data_end_position {
        info!("Nothing to push");
        return Ok(());
    }

    info!(
        "Data end position local {} remote {} ==> {} bytes to push",
        cursor_local.data_end_position,
        cursor_remote.data_end_position,
        cursor_local.data_end_position - cursor_remote.data_end_position
    );

    let last_i = (cursor_local
        .data_end_position
        .saturating_sub(cursor_local.data_begin_position))
        / PUSH_BLOCK_SIZE
        + 1;
    for i in 0..last_i {
        let position = (i * PUSH_BLOCK_SIZE).max(cursor_local.data_begin_position);

        let cursor_push = LedgerCursor::new(
            cursor_local.data_begin_position,
            position,
            cursor_local.data_end_position,
            CursorDirection::Forward,
            i + 1 < last_i,
        );

        let buf_size =
            PUSH_BLOCK_SIZE.min(cursor_local.data_end_position.saturating_sub(position)) as usize;
        let mut buf = vec![0u8; buf_size];
        persistent_storage_read(position, &mut buf).map_err(|e| e.to_string())?;
        info!(
            "Pushing block of {} bytes at position {}",
            buf_size, position,
        );
        let args = Encode!(&cursor_push.to_urlenc_string(), &buf).map_err(|e| e.to_string())?;
        let result = ledger_canister.call_update("data_push", &args).await?;
        #[allow(clippy::double_parens)]
        let result = Decode!(&result, Result<String, String>).map_err(|e| e.to_string())??;
        info!("Response from pushing at position {}: {}", position, result);
    }

    Ok(())
}
