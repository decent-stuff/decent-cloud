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
            "Running cleanup (signature audit retention: {} days)",
            self.retention_days
        );

        // Signature audit cleanup
        let deleted_count = self
            .database
            .cleanup_signature_audit(self.retention_days)
            .await?;

        if deleted_count > 0 {
            tracing::info!("Deleted {} old signature audit records", deleted_count);
        } else {
            tracing::debug!("No old signature audit records to delete");
        }

        // Expired provisioning locks cleanup
        match self.database.clear_expired_provisioning_locks().await {
            Ok(count) if count > 0 => {
                tracing::info!("Cleared {} expired provisioning locks", count);
            }
            Ok(_) => {
                tracing::debug!("No expired provisioning locks to clear");
            }
            Err(e) => {
                tracing::error!("Failed to clear expired provisioning locks: {:#}", e);
            }
        }

        // Expired setup tokens cleanup
        match self.database.cleanup_expired_setup_tokens().await {
            Ok(count) if count > 0 => {
                tracing::info!("Cleaned up {} expired setup tokens", count);
            }
            Ok(_) => {
                tracing::debug!("No expired setup tokens to clean up");
            }
            Err(e) => {
                tracing::error!("Failed to clean up expired setup tokens: {:#}", e);
            }
        }

        // Mark stale agents as offline (no heartbeat in last 5 minutes)
        match self.database.mark_stale_agents_offline().await {
            Ok(count) if count > 0 => {
                tracing::info!("Marked {} stale agents as offline", count);
            }
            Ok(_) => {
                tracing::debug!("No stale agents to mark offline");
            }
            Err(e) => {
                tracing::error!("Failed to mark stale agents offline: {:#}", e);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
