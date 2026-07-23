use crate::database::Database;
use std::sync::Arc;
use std::time::Duration;

/// Background service for daily payment releases to providers
pub struct PaymentReleaseService {
    database: Arc<Database>,
    interval: Duration,
}

impl PaymentReleaseService {
    pub fn new(database: Arc<Database>, interval_hours: u64) -> Self {
        Self {
            database,
            interval: Duration::from_secs(interval_hours * 60 * 60),
        }
    }

    /// Run the payment release service until shutdown is signalled.
    pub async fn run(self, mut shutdown: tokio::sync::watch::Receiver<bool>) {
        let mut interval = tokio::time::interval(self.interval);

        // Run initial release immediately on startup
        if let Err(e) = self.process_releases_once().await {
            tracing::error!("Initial payment release processing failed: {:#}", e);
        }

        loop {
            tokio::select! {
                _ = interval.tick() => {}
                _ = shutdown.changed() => {
                    tracing::info!("Payment release service shutting down gracefully");
                    return;
                }
            }
            if let Err(e) = self.process_releases_once().await {
                tracing::error!("Payment release processing failed: {:#}", e);
            }
        }
    }

    async fn process_releases_once(&self) -> anyhow::Result<()> {
        tracing::info!("Processing payment releases for active ICPay contracts");

        let contracts = self.database.get_contracts_for_release().await?;

        if contracts.is_empty() {
            tracing::debug!("No contracts ready for payment release");
            return Ok(());
        }

        tracing::info!(
            "Found {} contracts ready for payment release",
            contracts.len()
        );

        let current_timestamp_ns = crate::now_ns()?;

        for contract in contracts {
            let contract_id_bytes = hex::decode(&contract.contract_id)
                .map_err(|e| anyhow::anyhow!("Invalid contract_id hex: {}", e))?;

            // Calculate release amount
            let last_release = contract.last_release_at_ns.unwrap_or(
                contract
                    .start_timestamp_ns
                    .unwrap_or(contract.created_at_ns),
            );
            let period_start_ns = last_release;
            let period_end_ns = current_timestamp_ns;

            // Calculate total contract duration
            let start = contract
                .start_timestamp_ns
                .unwrap_or(contract.created_at_ns);
            let end = contract.end_timestamp_ns.unwrap_or(current_timestamp_ns);
            let total_duration_ns = end - start;

            if total_duration_ns <= 0 {
                tracing::warn!(
                    "Contract {} has invalid duration, skipping",
                    contract.contract_id
                );
                continue;
            }

            // Calculate earned amount for this period using integer arithmetic
            // (avoid float precision loss in financial calculations)
            let period_duration_ns = period_end_ns - period_start_ns;
            let release_amount_e9s = ((contract.payment_amount_e9s as i128)
                * (period_duration_ns as i128)
                / (total_duration_ns as i128)) as i64;

            if release_amount_e9s <= 0 {
                tracing::debug!(
                    "Contract {} has no earnings to release",
                    contract.contract_id
                );
                continue;
            }

            // Create payment release record. The capped method increments
            // total_released_e9s and inserts the payment_releases row in ONE
            // transaction, refusing the release when it would push the total
            // past payment_amount_e9s (R2/R3: no over-pay, no TOCTOU).
            let provider_pubkey_bytes = hex::decode(&contract.provider_pubkey)
                .map_err(|e| anyhow::anyhow!("Invalid provider_pubkey hex: {}", e))?;
            match self
                .database
                .create_capped_payment_release(
                    &contract_id_bytes,
                    "daily",
                    period_start_ns,
                    period_end_ns,
                    release_amount_e9s,
                    &provider_pubkey_bytes,
                )
                .await
            {
                Ok(Some(release)) => {
                    tracing::info!(
                        "Created payment release {} for contract {} (amount: {} e9s, period: {} - {})",
                        release.id,
                        contract.contract_id,
                        release_amount_e9s,
                        period_start_ns,
                        period_end_ns
                    );
                }
                Ok(None) => {
                    // Refused: the release would exceed payment_amount_e9s.
                    // This is expected once a contract is fully released; log
                    // it loud rather than silently skipping (money path).
                    tracing::warn!(
                        "Payment release for contract {} refused: amount {} e9s would push total_released past payment_amount {} e9s",
                        contract.contract_id,
                        release_amount_e9s,
                        contract.payment_amount_e9s
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to create payment release for contract {}: {:#}",
                        contract.contract_id,
                        e
                    );
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
