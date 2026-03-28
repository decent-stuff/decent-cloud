use crate::database::Database;
use crate::stripe_client::StripeClient;
use std::sync::Arc;
use std::time::Duration;

/// Background service for periodic cleanup tasks
pub struct CleanupService {
    database: Arc<Database>,
    stripe_client: Option<Arc<StripeClient>>,
    interval: Duration,
    retention_days: i64,
}

impl CleanupService {
    pub fn new(
        database: Arc<Database>,
        stripe_client: Option<Arc<StripeClient>>,
        interval_hours: u64,
        retention_days: i64,
    ) -> Self {
        Self {
            database,
            stripe_client,
            interval: Duration::from_secs(interval_hours * 60 * 60),
            retention_days,
        }
    }

    /// Run the cleanup service until shutdown is signalled.
    pub async fn run(self, mut shutdown: tokio::sync::watch::Receiver<bool>) {
        let mut interval = tokio::time::interval(self.interval);

        // Run initial cleanup immediately on startup
        if let Err(e) = self.cleanup_once().await {
            tracing::error!("Initial cleanup failed: {:#}", e);
        }

        loop {
            tokio::select! {
                _ = interval.tick() => {}
                _ = shutdown.changed() => {
                    tracing::info!("Cleanup service shutting down gracefully");
                    return;
                }
            }
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

        // Clean up expired credentials (auto-delete after 7 days)
        match self.database.cleanup_expired_credentials().await {
            Ok(count) if count > 0 => {
                tracing::info!("Cleaned up {} expired VM credentials", count);
            }
            Ok(_) => {
                tracing::debug!("No expired VM credentials to clean up");
            }
            Err(e) => {
                tracing::error!("Failed to clean up expired credentials: {:#}", e);
            }
        }

        // Purge terminal contracts older than retention period
        match self
            .database
            .purge_terminal_contracts(self.retention_days)
            .await
        {
            Ok(count) if count > 0 => {
                tracing::info!(
                    "Purged {} terminal contracts older than {} days",
                    count,
                    self.retention_days
                );
            }
            Ok(_) => {
                tracing::debug!("No terminal contracts to purge");
            }
            Err(e) => {
                tracing::error!("Failed to purge terminal contracts: {:#}", e);
            }
        }

        // Expire cloud contracts past their end_timestamp_ns
        match self.database.expire_and_cleanup_cloud_contracts().await {
            Ok(count) if count > 0 => {
                tracing::info!(
                    "Expired {} cloud contracts and marked resources for deletion",
                    count
                );
            }
            Ok(_) => {
                tracing::debug!("No expired cloud contracts to clean up");
            }
            Err(e) => {
                tracing::error!("Failed to expire cloud contracts: {:#}", e);
            }
        }

        // Report metered usage to Stripe for completed billing periods
        self.report_metered_usage().await;

        Ok(())
    }

    /// Process unreported metered usage - update from heartbeats and mark as reported
    async fn report_metered_usage(&self) {
        let unreported = match self.database.get_unreported_usage().await {
            Ok(usage) => usage,
            Err(e) => {
                tracing::error!("Failed to get unreported usage: {:#}", e);
                return;
            }
        };

        if unreported.is_empty() {
            tracing::debug!("No unreported usage to process");
            return;
        }

        for usage in unreported {
            let contract_id_bytes = match hex::decode(&usage.contract_id) {
                Ok(bytes) => bytes,
                Err(e) => {
                    tracing::error!("Invalid contract_id hex {}: {:#}", usage.contract_id, e);
                    continue;
                }
            };

            // Update usage from heartbeats to get final count
            if let Err(e) = self
                .database
                .update_usage_from_heartbeats(&contract_id_bytes, usage.id, "hour")
                .await
            {
                tracing::error!(
                    "Failed to update usage from heartbeats for {}: {:#}",
                    usage.contract_id,
                    e
                );
                continue;
            }

            // Report usage to Stripe and mark as reported
            let stripe_record_id = self
                .report_usage_to_stripe(&contract_id_bytes, &usage)
                .await;
            if let Err(e) = self
                .database
                .mark_usage_reported(usage.id, stripe_record_id.as_deref().unwrap_or(""))
                .await
            {
                tracing::error!("Failed to mark usage {} as reported: {:#}", usage.id, e);
            } else {
                tracing::debug!(
                    "Processed usage {} for contract {} (stripe_record: {})",
                    usage.id,
                    usage.contract_id,
                    stripe_record_id.as_deref().unwrap_or("none")
                );
            }
        }
    }

    /// Attempt to report usage to Stripe for a single billing period.
    ///
    /// Returns `Some(stripe_usage_record_id)` on success, `None` if Stripe is not
    /// configured or the contract has no subscription. Errors are logged and
    /// converted to `None` so the usage is still marked as reported locally to
    /// prevent reprocessing loops.
    async fn report_usage_to_stripe(
        &self,
        contract_id_bytes: &[u8],
        usage: &crate::database::contracts::ContractUsage,
    ) -> Option<String> {
        let stripe = self.stripe_client.as_ref()?;

        let contract = match self.database.get_contract(contract_id_bytes).await {
            Ok(Some(c)) => c,
            Ok(None) => {
                tracing::warn!(
                    "Contract {} not found, skipping Stripe reporting",
                    usage.contract_id
                );
                return None;
            }
            Err(e) => {
                tracing::error!("Failed to fetch contract {}: {:#}", usage.contract_id, e);
                return None;
            }
        };

        let subscription_id = match contract.stripe_subscription_id {
            Some(ref id) if !id.is_empty() => id.clone(),
            _ => return None,
        };

        let offering = match self
            .database
            .get_offering_by_id(&contract.offering_id)
            .await
        {
            Ok(Some(o)) => o,
            Ok(None) => {
                tracing::warn!(
                    "Offering {} not found for contract {}, skipping Stripe reporting",
                    contract.offering_id,
                    usage.contract_id
                );
                return None;
            }
            Err(e) => {
                tracing::error!(
                    "Failed to fetch offering {} for contract {}: {:#}",
                    contract.offering_id,
                    usage.contract_id,
                    e
                );
                return None;
            }
        };

        let metered_price_id = match offering.stripe_metered_price_id {
            Some(ref id) if !id.is_empty() => id.clone(),
            _ => {
                tracing::debug!(
                    "Offering {} has no metered price, skipping Stripe reporting",
                    contract.offering_id
                );
                return None;
            }
        };

        // Find the subscription item ID matching the metered price
        let sub_item_id = match stripe
            .get_subscription_item_id(&subscription_id, &metered_price_id)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                tracing::error!(
                    "Failed to find subscription item for sub={} price={}: {:#}",
                    subscription_id,
                    metered_price_id,
                    e
                );
                return None;
            }
        };

        let quantity = usage.units_used.round() as i64;
        let timestamp = Some(usage.billing_period_end);

        match stripe
            .create_usage_record(&sub_item_id, quantity, timestamp, "set")
            .await
        {
            Ok(record) => {
                tracing::info!(
                    "Reported usage to Stripe: record_id={} quantity={} for contract {}",
                    record.id,
                    quantity,
                    usage.contract_id
                );
                Some(record.id)
            }
            Err(e) => {
                tracing::error!(
                    "Failed to create Stripe usage record for contract {}: {:#}",
                    usage.contract_id,
                    e
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests;
