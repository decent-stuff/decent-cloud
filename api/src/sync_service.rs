use crate::{database::Database, ledger_client::LedgerClient};
use anyhow::Result;
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
        let ledger_dir = std::env::var("LEDGER_DIR")
            .map(|path| std::path::PathBuf::from(path))
            .unwrap_or_else(|_| {
                // Fallback to temp directory for development
                let temp_dir = tempfile::tempdir()
                    .expect("Failed to create temp dir");
                temp_dir.keep()
            });

        // Ensure the directory exists
        std::fs::create_dir_all(&ledger_dir).expect("Failed to create ledger directory");

        let ledger_parser = LedgerMap::new_with_path(None, Some(ledger_dir))
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
        interval.tick().await; // Skip first immediate tick

        loop {
            interval.tick().await;
            if let Err(e) = self.sync_once().await {
                tracing::error!("Sync failed: {}", e);
            }
        }
    }

    async fn sync_once(&self) -> Result<()> {
        let last_position = self.database.get_last_sync_position().await?;
        let raw_data = self.fetch_data(last_position).await?;

        if raw_data.is_empty() {
            return Ok(());
        }

        let entries = self.parse_ledger_data(&raw_data)?;
        self.store_entries(entries).await?;
        self.update_sync_position(last_position, raw_data.len())
            .await?;

        Ok(())
    }

    async fn fetch_data(&self, last_position: u64) -> Result<Vec<u8>> {
        tracing::debug!("Starting sync from position {}", last_position);

        let cursor_str = if last_position > 0 {
            Some(format!("position={}", last_position))
        } else {
            None
        };

        let (new_cursor_str, raw_data) = self.ledger_client.data_fetch(cursor_str).await?;
        tracing::info!(
            "Fetched {} bytes, new cursor: {}",
            raw_data.len(),
            new_cursor_str
        );

        Ok(raw_data)
    }

    async fn store_entries(&self, entries: Vec<crate::database::LedgerEntryData>) -> Result<()> {
        tracing::info!("Parsed {} ledger entries", entries.len());
        if !entries.is_empty() {
            self.database.insert_entries(entries).await?;
        }
        Ok(())
    }

    async fn update_sync_position(&self, last_position: u64, data_len: usize) -> Result<()> {
        let new_position = last_position + data_len as u64;
        self.database.update_sync_position(new_position).await?;
        tracing::info!("Sync completed, new position: {}", new_position);
        Ok(())
    }

    fn parse_ledger_data(&self, data: &[u8]) -> Result<Vec<crate::database::LedgerEntryData>> {
        let mut entries = Vec::new();
        let parser = self.ledger_parser.lock().unwrap();

        for block_result in parser.iter_raw_from_slice(data) {
            let (_block_header, block, _block_hash) = block_result?;

            for entry in block.entries() {
                entries.push(crate::database::LedgerEntryData {
                    label: entry.label().to_string(),
                    key: entry.key().to_vec(),
                    value: entry.value().to_vec(),
                });
            }
        }

        Ok(entries)
    }
}

#[cfg(test)]
mod tests;
