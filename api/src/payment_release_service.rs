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

    /// Run the payment release service indefinitely
    pub async fn run(self) {
        let mut interval = tokio::time::interval(self.interval);

        // Run initial release immediately on startup
        if let Err(e) = self.process_releases_once().await {
            tracing::error!("Initial payment release processing failed: {}", e);
        }

        loop {
            interval.tick().await;
            if let Err(e) = self.process_releases_once().await {
                tracing::error!("Payment release processing failed: {}", e);
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

        let current_timestamp_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

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

            // Calculate earned amount for this period
            let period_duration_ns = period_end_ns - period_start_ns;
            let release_amount_e9s = (contract.payment_amount_e9s as f64
                * period_duration_ns as f64
                / total_duration_ns as f64) as i64;

            if release_amount_e9s <= 0 {
                tracing::debug!(
                    "Contract {} has no earnings to release",
                    contract.contract_id
                );
                continue;
            }

            // Create payment release record
            match self
                .database
                .create_payment_release(
                    &contract_id_bytes,
                    "daily",
                    period_start_ns,
                    period_end_ns,
                    release_amount_e9s,
                    &hex::decode(&contract.provider_pubkey)
                        .map_err(|e| anyhow::anyhow!("Invalid provider_pubkey hex: {}", e))?,
                )
                .await
            {
                Ok(release) => {
                    tracing::info!(
                        "Created payment release {} for contract {} (amount: {} e9s, period: {} - {})",
                        release.id,
                        contract.contract_id,
                        release_amount_e9s,
                        period_start_ns,
                        period_end_ns
                    );

                    // Update contract tracking fields
                    let new_total_released =
                        contract.total_released_e9s.unwrap_or(0) + release_amount_e9s;
                    if let Err(e) = self
                        .database
                        .update_contract_release_tracking(
                            &contract_id_bytes,
                            current_timestamp_ns,
                            new_total_released,
                        )
                        .await
                    {
                        tracing::error!(
                            "Failed to update release tracking for contract {}: {}",
                            contract.contract_id,
                            e
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to create payment release for contract {}: {}",
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
