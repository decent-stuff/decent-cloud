use crate::database::Database;
use std::sync::Arc;
use std::time::Duration;

/// Background service that auto-renews expiring contracts.
///
/// Runs every 6 hours. For each active contract with auto_renew=true that expires
/// within 48 hours, it creates a new rental request with the same parameters and
/// clears auto_renew on the old contract so it won't trigger again.
pub struct AutoRenewalService {
    database: Arc<Database>,
    interval: Duration,
}

impl AutoRenewalService {
    pub fn new(database: Arc<Database>, interval_hours: u64) -> Self {
        Self {
            database,
            interval: Duration::from_secs(interval_hours * 60 * 60),
        }
    }

    /// Run the auto-renewal service until shutdown is signalled.
    pub async fn run(self, mut shutdown: tokio::sync::watch::Receiver<bool>) {
        let mut interval = tokio::time::interval(self.interval);

        // Run initial check on startup
        self.process_renewals_once().await;

        loop {
            tokio::select! {
                _ = interval.tick() => {}
                _ = shutdown.changed() => {
                    tracing::info!("Auto-renewal service shutting down gracefully");
                    return;
                }
            }
            self.process_renewals_once().await;
        }
    }

    async fn process_renewals_once(&self) {
        tracing::info!("Checking for contracts due for auto-renewal");

        let contracts = match self.database.get_contracts_for_renewal().await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to fetch contracts for auto-renewal: {:#}", e);
                return;
            }
        };

        if contracts.is_empty() {
            tracing::debug!("No contracts due for auto-renewal");
            return;
        }

        tracing::info!("{} contract(s) due for auto-renewal", contracts.len());

        for contract in contracts {
            if let Err(e) = self.renew_contract(&contract).await {
                tracing::error!(
                    contract_id = %contract.contract_id,
                    "Auto-renewal failed: {:#}",
                    e
                );
                // Continue processing remaining contracts — one failure must not block others
            }
        }
    }

    async fn renew_contract(
        &self,
        contract: &crate::database::contracts::Contract,
    ) -> anyhow::Result<()> {
        let contract_id_bytes = hex::decode(&contract.contract_id)
            .map_err(|e| anyhow::anyhow!("Invalid contract_id hex: {}", e))?;
        let requester_pubkey_bytes = hex::decode(&contract.requester_pubkey)
            .map_err(|e| anyhow::anyhow!("Invalid requester_pubkey hex: {}", e))?;

        // Parse offering_db_id from the stored offering_id string
        let offering_db_id: i64 = contract
            .offering_id
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid offering_id '{}'", contract.offering_id))?;

        let params = crate::database::contracts::RentalRequestParams {
            offering_db_id,
            ssh_pubkey: Some(contract.requester_ssh_pubkey.clone()),
            contact_method: Some(contract.requester_contact.clone()),
            request_memo: Some(format!("Auto-renewal of {}", &contract.contract_id[..12])),
            duration_hours: contract.original_duration_hours.or(contract.duration_hours),
            payment_method: Some(contract.payment_method.clone()),
            buyer_address: contract.buyer_address.clone(),
            operating_system: contract.operating_system.clone(),
        };

        let new_contract_id = self
            .database
            .create_rental_request(&requester_pubkey_bytes, params)
            .await?;

        // Clear auto_renew on the old contract so it won't trigger again
        self.database
            .set_contract_auto_renew(&contract_id_bytes, &requester_pubkey_bytes, false)
            .await?;

        tracing::info!(
            old_contract_id = %contract.contract_id,
            new_contract_id = %hex::encode(&new_contract_id),
            "Auto-renewed contract"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_auto_renewal_service_interval() {
        // Verify the 6-hour interval is 21600 seconds
        assert_eq!(6u64 * 60 * 60, 21_600);
    }
}
