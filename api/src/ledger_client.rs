use anyhow::Result;
use candid::{Decode, Encode, Principal};
use ic_agent::Agent;

/// Ledger canister client for fetching data
pub struct LedgerClient {
    agent: Agent,
    canister_id: Principal,
}

impl LedgerClient {
    pub async fn new(network_url: &str, canister_id: Principal) -> Result<Self> {
        tracing::debug!(
            "Initializing ledger client with network_url: {}, canister_id: {}",
            network_url,
            canister_id
        );

        let agent = Agent::builder()
            .with_url(network_url)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build agent: {}", e))?;

        tracing::debug!("Agent built, fetching root key...");
        agent
            .fetch_root_key()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch root key from {}: {}", network_url, e))?;

        tracing::info!(
            "Ledger client initialized successfully for canister {}",
            canister_id
        );
        Ok(Self { agent, canister_id })
    }

    /// Fetch ledger data starting from cursor position
    pub async fn data_fetch(
        &self,
        cursor: Option<String>,
        bytes_before: Option<Vec<u8>>,
    ) -> Result<(String, Vec<u8>)> {
        let args = Encode!(&cursor, &bytes_before)?;
        let response = self
            .agent
            .query(&self.canister_id, "data_fetch")
            .with_arg(args)
            .call()
            .await?;

        #[allow(clippy::double_parens)]
        {
            Decode!(response.as_slice(), Result<(String, Vec<u8>), String>)?
                .map_err(|e| anyhow::anyhow!("Canister error: {}", e))
        }
    }
}

#[cfg(test)]
mod tests;
