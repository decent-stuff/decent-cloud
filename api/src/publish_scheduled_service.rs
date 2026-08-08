use crate::database::Database;
use std::sync::Arc;
use std::time::Duration;

/// Background service that auto-publishes scheduled draft offerings.
///
/// Polls at the configured interval (operator-configurable via
/// `PUBLISH_SCHEDULED_INTERVAL_SECS`, default 60s). Any draft offering with
/// `publish_at <= NOW()` is published by setting `is_draft = false` and
/// clearing `publish_at`.
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

    /// Run the publish-scheduled service until shutdown is signalled.
    pub async fn run(self, mut shutdown: tokio::sync::watch::Receiver<bool>) {
        let mut interval = tokio::time::interval(self.interval);

        loop {
            tokio::select! {
                _ = interval.tick() => {}
                _ = shutdown.changed() => {
                    tracing::info!("Publish-scheduled service shutting down gracefully");
                    return;
                }
            }
            match self.database.publish_scheduled_offerings().await {
                Ok(0) => {}
                Ok(n) => tracing::info!("Published {} scheduled offering(s)", n),
                Err(e) => tracing::error!("Failed to publish scheduled offerings: {:#}", e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DEFAULT_DATABASE_URL;

    // Test-only accessor to verify the interval is honored (regression guard:
    // main.rs previously hardcoded 60s instead of reading
    // PUBLISH_SCHEDULED_INTERVAL_SECS; this pins that the service itself is
    // parameterized so the env value flows through).
    impl PublishScheduledService {
        fn test_interval(&self) -> Duration {
            self.interval
        }
    }

    #[tokio::test]
    async fn interval_is_configurable_not_constant() {
        // The interval passed to new() must be stored verbatim (not a hardcoded
        // constant). Connects to the warm-stack DB; skips gracefully if no DB is
        // available, matching the convention in main_tests.rs.
        let base_url =
            std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());        let db = match Database::new(&base_url).await {
            Ok(db) => Arc::new(db),
            Err(e) => {
                eprintln!(
                    "Skipping interval_is_configurable_not_constant: DB unavailable ({:#})",
                    e
                );
                return;
            }
        };
        let s60 = PublishScheduledService::new(db.clone(), 60);
        let s120 = PublishScheduledService::new(db, 120);
        assert_eq!(s60.test_interval(), Duration::from_secs(60));
        assert_eq!(s120.test_interval(), Duration::from_secs(120),
            "interval must be operator-configurable, not a hardcoded constant");
    }
}
