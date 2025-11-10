use anyhow::Result;
use candid::{Decode, Encode, Principal};
use ic_agent::Agent;
use icrc_ledger_types::icrc::generic_metadata_value::MetadataValue;

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

    /// Fetch metadata from the canister with retry logic
    pub async fn fetch_metadata(&self) -> Result<Vec<(String, MetadataValue)>> {
        const MAX_RETRIES: u32 = 3;
        const RETRY_DELAY_MS: u64 = 1000;

        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            // Refresh root key on retries (especially important for local development)
            if attempt > 0 {
                if let Err(e) = self.agent.fetch_root_key().await {
                    tracing::warn!("Failed to refresh root key on retry {}: {}", attempt, e);
                }
            }

            match self.try_fetch_metadata().await {
                Ok(metadata) => return Ok(metadata),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < MAX_RETRIES - 1 {
                        let delay_ms = RETRY_DELAY_MS * 2_u64.pow(attempt);
                        tracing::debug!(
                            "Metadata fetch failed (attempt {}), retrying in {}ms: {}",
                            attempt + 1,
                            delay_ms,
                            last_error.as_ref().unwrap()
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!("Failed to fetch metadata after {} attempts", MAX_RETRIES)
        }))
    }

    /// Internal method to attempt a single metadata fetch
    async fn try_fetch_metadata(&self) -> Result<Vec<(String, MetadataValue)>> {
        tracing::debug!(
            "Attempting to fetch metadata from canister {}",
            self.canister_id
        );

        let response = self
            .agent
            .query(&self.canister_id, "metadata")
            .call()
            .await
            .map_err(|e| {
                anyhow::anyhow!("Query call to canister {} failed: {}", self.canister_id, e)
            })?;

        tracing::debug!("Received response, decoding metadata...");
        #[allow(clippy::double_parens)]
        Decode!(response.as_slice(), Vec<(String, MetadataValue)>)
            .map_err(|e| anyhow::anyhow!("Failed to decode metadata response: {}", e))
    }
}

#[cfg(test)]
mod tests;
