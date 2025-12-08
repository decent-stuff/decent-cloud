use crate::database::Database;
use std::sync::Arc;
use std::time::Duration;

/// Background service for periodic cleanup tasks
pub struct CleanupService {
    database: Arc<Database>,
    interval: Duration,
    retention_days: i64,
}

impl CleanupService {
    pub fn new(database: Arc<Database>, interval_hours: u64, retention_days: i64) -> Self {
        Self {
            database,
            interval: Duration::from_secs(interval_hours * 60 * 60),
            retention_days,
        }
    }

    /// Run the cleanup service indefinitely
    pub async fn run(self) {
        let mut interval = tokio::time::interval(self.interval);

        // Run initial cleanup immediately on startup
        if let Err(e) = self.cleanup_once().await {
            tracing::error!("Initial cleanup failed: {:#}", e);
        }

        loop {
            interval.tick().await;
            if let Err(e) = self.cleanup_once().await {
                tracing::error!("Cleanup failed: {:#}", e);
            }
        }
    }

    async fn cleanup_once(&self) -> anyhow::Result<()> {
        tracing::info!(
            "Running signature audit cleanup (retention: {} days)",
            self.retention_days
        );

        let deleted_count = self
            .database
            .cleanup_signature_audit(self.retention_days)
            .await?;

        if deleted_count > 0 {
            tracing::info!("Deleted {} old signature audit records", deleted_count);
        } else {
            tracing::debug!("No old signature audit records to delete");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
