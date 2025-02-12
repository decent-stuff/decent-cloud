use candid::{Decode, Encode};
use dcc_common::DATA_PULL_BYTES_BEFORE_LEN;
use dcc_common::{cursor_from_data, CursorDirection, LedgerCursor};
use decent_cloud::ledger_canister_client::LedgerCanister;
use fs_err::OpenOptions;
use ledger_map::{platform_specific::persistent_storage_read, LedgerMap};
use log::info;
use std::{
    io::{Seek, Write},
    path::PathBuf,
};

use super::get_ledger_metadata;

const PUSH_BLOCK_SIZE: u64 = 1024 * 1024;

pub async fn ledger_data_fetch(
    ledger_canister: &LedgerCanister,
    ledger_local: &mut LedgerMap,
) -> Result<(), Box<dyn std::error::Error>> {
    let ledger_file_path = ledger_local
        .get_file_path()
        .expect("failed to open the local ledger path");

    let cursor_local = {
        cursor_from_data(
            ledger_map::partition_table::get_data_partition()
                .await
                .start_lba,
            ledger_map::platform_specific::persistent_storage_size_bytes().await,
            ledger_local.get_next_block_start_pos(),
            ledger_local.get_next_block_start_pos(),
        )
    };

    let bytes_before = if cursor_local.position > DATA_PULL_BYTES_BEFORE_LEN as u64 {
        let mut buf = vec![0u8; DATA_PULL_BYTES_BEFORE_LEN as usize];
        persistent_storage_read(
            cursor_local.position - DATA_PULL_BYTES_BEFORE_LEN as u64,
            &mut buf,
        )
        .await?;
        Some(buf)
    } else {
        None
    };

    info!(
        "Fetching data from the Ledger canister {} to {}, with local cursor: {} and bytes before: {:?}",
        ledger_canister.canister_id(),
        ledger_file_path.display(),
        cursor_local,
        hex::encode(bytes_before.as_ref().unwrap_or(&vec![])),
    );
    let (cursor_remote, data) = ledger_canister
        .data_fetch(Some(cursor_local.to_request_string()), bytes_before)
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
    let mut ledger_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&ledger_file_path)
        .expect("failed to open the local ledger path");
    let file_size_bytes = ledger_file.metadata().unwrap().len();
    let file_size_bytes_target = offset_remote + data.len() as u64 + 1024 * 1024;
    if file_size_bytes < file_size_bytes_target {
        ledger_file.set_len(file_size_bytes_target).unwrap();
        ledger_file
            .seek(std::io::SeekFrom::Start(offset_remote))
            .unwrap();
    }
    if offset_remote + cursor_remote.response_bytes > cursor_local.position {
        ledger_file.write_all(&data).unwrap();
        info!(
            "Wrote {} bytes at offset 0x{:0x} of file {}",
            data.len(),
            offset_remote,
            ledger_file_path.display()
        );
    }
    // Set the modified time to the current time, to mark that the data is up-to-date
    filetime::set_file_mtime(ledger_file_path, std::time::SystemTime::now().into())?;

    if !data.is_empty() {
        ledger_local.refresh_ledger().await?;
    }

    Ok(())
}

pub async fn ledger_data_push(
    ledger_canister: &LedgerCanister,
    local_ledger_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let ledger_local = LedgerMap::new_with_path(Some(vec![]), Some(local_ledger_path))
        .await
        .expect("Failed to create LedgerMap");
    let cursor_local = cursor_from_data(
        ledger_map::partition_table::get_data_partition()
            .await
            .start_lba,
        ledger_map::platform_specific::persistent_storage_size_bytes().await,
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
        persistent_storage_read(position, &mut buf)
            .await
            .map_err(|e| e.to_string())?;
        info!(
            "Pushing block of {} bytes at position {}",
            buf_size, position,
        );
        let args = Encode!(&cursor_push.to_urlenc_string(), &buf).map_err(|e| e.to_string())?;
        let result = ledger_canister.call_update("data_push", &args).await?;
        let result = Decode!(&result, Result<String, String>).map_err(|e| e.to_string())??;
        info!("Response from pushing at position {}: {}", position, result);
    }

    Ok(())
}
