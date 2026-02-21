use crate::database::Database;
use std::sync::Arc;
use std::time::Duration;

/// Background service that auto-publishes scheduled draft offerings.
///
/// Runs every 60 seconds. Any draft offering with `publish_at <= NOW()` is published
/// by setting `is_draft = false` and clearing `publish_at`.
pub struct PublishScheduledService {
    database: Arc<Database>,
    interval: Duration,
}

impl PublishScheduledService {
    pub fn new(database: Arc<Database>, interval_secs: u64) -> Self {
        Self {
            database,
            interval: Duration::from_secs(interval_secs),
        }
    }

    /// Run the publish-scheduled service indefinitely.
    pub async fn run(self) {
        let mut interval = tokio::time::interval(self.interval);

        loop {
            interval.tick().await;
            match self.database.publish_scheduled_offerings().await {
                Ok(0) => {}
                Ok(n) => tracing::info!("Published {} scheduled offering(s)", n),
                Err(e) => tracing::error!("Failed to publish scheduled offerings: {:#}", e),
            }
        }
    }
}
