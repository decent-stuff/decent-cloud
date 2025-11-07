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
        let agent = Agent::builder().with_url(network_url).build()?;

        agent.fetch_root_key().await?;

        Ok(Self { agent, canister_id })
    }

    /// Fetch ledger data starting from cursor position
    pub async fn data_fetch(&self, cursor: Option<String>) -> Result<(String, Vec<u8>)> {
        let args = Encode!(&cursor, &None::<Vec<u8>>)?;
        let response = self
            .agent
            .query(&self.canister_id, "data_fetch")
            .with_arg(args)
            .call()
            .await?;

        Decode!(response.as_slice(), Result<(String, Vec<u8>), String>)?
            .map_err(|e| anyhow::anyhow!("Canister error: {}", e))
    }
}

#[cfg(test)]
mod tests;
