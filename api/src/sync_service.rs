use crate::{database::Database, ledger_client::LedgerClient, ledger_path::ledger_dir_path};
use anyhow::Result;
use dcc_common::{fetch_and_write_ledger_data, parse_ledger_entries};
use ledger_map::LedgerMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct SyncService {
    ledger_client: Arc<LedgerClient>,
    database: Arc<Database>,
    interval: Duration,
    ledger_parser: Arc<Mutex<LedgerMap>>,
}

impl SyncService {
    pub fn new(
        ledger_client: Arc<LedgerClient>,
        database: Arc<Database>,
        interval_secs: u64,
    ) -> Self {
        let ledger_dir =
            ledger_dir_path().expect("Failed to resolve ledger directory for sync service");
        let ledger_file = ledger_dir.join("main.bin");

        let ledger_parser = LedgerMap::new_with_path(None, Some(ledger_file))
            .expect("Failed to create LedgerMap parser");

        Self {
            ledger_client,
            database,
            interval: Duration::from_secs(interval_secs),
            ledger_parser: Arc::new(Mutex::new(ledger_parser)),
        }
    }

    pub async fn run(self) {
        let mut interval = tokio::time::interval(self.interval);

        // Run initial sync immediately on startup
        if let Err(e) = self.sync_once().await {
            tracing::error!("Initial sync failed: {}", e);
        }

        loop {
            interval.tick().await;
            if let Err(e) = self.sync_once().await {
                tracing::error!("Sync failed: {}", e);
            }
        }
    }

    async fn sync_once(&self) -> Result<()> {
        let last_position = self.database.get_last_sync_position().await?;
        let (raw_data, data_start, data_end) = self.fetch_data(last_position).await?;

        if raw_data.is_empty() {
            return Ok(());
        }

        let entries = self.parse_ledger_data(data_start)?;
        self.store_entries(entries).await?;
        self.update_sync_position(data_end).await?;

        Ok(())
    }

    #[allow(clippy::await_holding_lock)]
    async fn fetch_data(&self, last_position: u64) -> Result<(Vec<u8>, u64, u64)> {
        tracing::debug!("Starting sync from position {}", last_position);

        let (raw_data, data_start, data_end) = {
            let mut ledger_parser = self.ledger_parser.lock().map_err(|_| {
                anyhow::anyhow!("Failed to acquire ledger parser lock - possible poisoning")
            })?;

            fetch_and_write_ledger_data(
                &mut ledger_parser,
                |cursor_str, bytes_before| async move {
                    self.ledger_client
                        .data_fetch(cursor_str, bytes_before)
                        .await
                },
                last_position,
            )
            .await?
        };

        Ok((raw_data, data_start, data_end))
    }

    async fn store_entries(&self, entries: Vec<crate::database::LedgerEntryData>) -> Result<()> {
        tracing::info!("Parsed {} ledger entries", entries.len());
        if !entries.is_empty() {
            self.database.insert_entries(entries).await?;
        }
        Ok(())
    }

    async fn update_sync_position(&self, new_position: u64) -> Result<()> {
        self.database.update_sync_position(new_position).await?;
        tracing::info!("Sync completed, new position: {}", new_position);
        Ok(())
    }

    fn parse_ledger_data(&self, start_pos: u64) -> Result<Vec<crate::database::LedgerEntryData>> {
        let parser = self.ledger_parser.lock().map_err(|_| {
            anyhow::anyhow!("Failed to acquire ledger parser lock - possible poisoning")
        })?;

        let common_entries = parse_ledger_entries(&parser, start_pos)?;

        // Convert common entries to API database entries
        let entries = common_entries
            .into_iter()
            .map(|e| crate::database::LedgerEntryData {
                label: e.label,
                key: e.key,
                value: e.value,
                block_timestamp_ns: e.block_timestamp_ns,
                block_hash: e.block_hash,
                block_offset: e.block_offset,
            })
            .collect();

        Ok(entries)
    }
}

#[cfg(test)]
mod tests;
