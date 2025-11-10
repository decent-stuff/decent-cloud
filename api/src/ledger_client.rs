use anyhow::Result;
use candid::{Decode, Encode, Principal, CandidType};
use ic_agent::Agent;
use serde::{Deserialize, Serialize};

/// Ledger canister client for fetching data
pub struct LedgerClient {
    agent: Agent,
    canister_id: Principal,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
#[serde(untagged)]
pub enum MetadataValue {
    Nat(u64),
    Int(i64),
    Text(String),
    Blob(Vec<u8>),
}

impl LedgerClient {
    pub async fn new(network_url: &str, canister_id: Principal) -> Result<Self> {
        let agent = Agent::builder().with_url(network_url).build()?;

        agent.fetch_root_key().await?;

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

    /// Fetch metadata from the canister
    pub async fn fetch_metadata(&self) -> Result<Vec<(String, MetadataValue)>> {
        let response = self
            .agent
            .query(&self.canister_id, "metadata")
            .call()
            .await?;

        Decode!(response.as_slice(), Vec<(String, MetadataValue)>)
            .map_err(|e| anyhow::anyhow!("Failed to decode metadata: {}", e))
    }
}

#[cfg(test)]
mod tests;
