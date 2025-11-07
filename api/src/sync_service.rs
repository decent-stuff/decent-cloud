use crate::{database::Database, ledger_client::LedgerClient};
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;

pub struct SyncService {
    ledger_client: Arc<LedgerClient>,
    database: Arc<Database>,
    interval: Duration,
}

impl SyncService {
    pub fn new(
        ledger_client: Arc<LedgerClient>,
        database: Arc<Database>,
        interval_secs: u64,
    ) -> Self {
        Self {
            ledger_client,
            database,
            interval: Duration::from_secs(interval_secs),
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
        // TODO: Implement incremental sync logic
        // 1. Get last sync position from DB
        // 2. Fetch data from canister using cursor
        // 3. Parse ledger data into blocks and entries
        // 4. Insert entries into DB
        // 5. Update sync position
        tracing::warn!("Sync not yet implemented - skipping");
        Ok(())
    }
}
