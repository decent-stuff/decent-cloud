use crate::identity::dcc_to_ic_auth;
use candid::{Decode, Encode};
use dcc_common::DccIdentity;
use ic_agent::{export::Principal, identity::BasicIdentity, Agent};
use log::Level;

type ResultString = Result<String, String>;

#[derive(Debug)]
pub struct LedgerCanister {
    agent: Agent,
    // wallet_canister_id: Principal,
    canister_id: Principal,
    // network_name: String,
    // network_url: String,
}

impl LedgerCanister {
    pub async fn new(
        // wallet_canister_id: Principal,
        canister_id: Principal,
        identity: Option<BasicIdentity>,
        // network_name: String,
        network_url: &str,
    ) -> anyhow::Result<Self> {
        let agent = Agent::builder().with_url(network_url);
        let agent = match identity {
            Some(identity) => agent.with_identity(identity),
            None => agent,
        }
        .build()?;

        // If you know the root key ahead of time, you can use `agent.set_root_key(root_key);`.
        agent.fetch_root_key().await?;

        Ok(Self {
            agent,
            // wallet_canister_id,
            canister_id,
            // network_name,
            // network_url: network_url.to_string(),
        })
    }

    pub async fn new_with_identity(
        network_url: &str,
        canister_id: Principal,
        identity: BasicIdentity,
    ) -> anyhow::Result<Self> {
        Self::new(
            // wallet_canister_id,
            canister_id,
            Some(identity),
            // network_name,
            network_url,
        )
        .await
    }

    pub async fn new_with_dcc_id(
        network_url: &str,
        canister_id: Principal,
        dcc_id: &DccIdentity,
    ) -> anyhow::Result<Self> {
        let ic_auth = dcc_to_ic_auth(dcc_id)?;
        Self::new_with_identity(network_url, canister_id, ic_auth).await
    }

    pub async fn new_without_identity(
        network_url: &str,
        canister_id: Principal,
    ) -> anyhow::Result<Self> {
        Self::new(
            // wallet_canister_id,
            canister_id,
            None,
            // network_name,
            network_url,
        )
        .await
    }

    pub fn canister_id(&self) -> &Principal {
        &self.canister_id
    }

    pub async fn call_update(&self, method_name: &str, args: &[u8]) -> Result<Vec<u8>, String> {
        self.agent
            .update(&self.canister_id, method_name)
            // .with_effective_canister_id(self.canister_id)
            .with_arg(args)
            .call_and_wait()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn call_query(&self, method_name: &str, args: &[u8]) -> Result<Vec<u8>, String> {
        self.agent
            .query(&self.canister_id, method_name)
            .with_arg(args)
            .call()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn init_ledger_map(&self) -> Result<String, String> {
        let args = Encode!(&()).map_err(|e| e.to_string())?;
        let response = self.call_update("init_ledger_map", &args).await?;
        #[allow(clippy::double_parens)]
        Decode!(response.as_slice(), ResultString).map_err(|e| e.to_string())?
    }

    pub async fn provider_register(&self, key: &[u8], value: &[u8]) -> Result<String, String> {
        let args = Encode!(&key, &value).map_err(|e| e.to_string())?;
        let response = self.call_update("provider_register", &args).await?;
        #[allow(clippy::double_parens)]
        Decode!(response.as_slice(), ResultString).map_err(|e| e.to_string())?
    }

    pub async fn provider_check_in(
        &self,
        key: &[u8],
        memo: &String,
        nonce_crypto_sig: &[u8],
    ) -> Result<String, String> {
        let args = Encode!(&key, &memo, &nonce_crypto_sig).map_err(|e| e.to_string())?;
        let response = self.call_update("provider_check_in", &args).await?;
        #[allow(clippy::double_parens)]
        Decode!(response.as_slice(), ResultString).map_err(|e| e.to_string())?
    }

    pub async fn get_check_in_nonce(&self) -> Result<Vec<u8>, String> {
        let args = Encode!(&()).map_err(|e| format!("Failed to encode args: {}", e))?;
        let response = self.call_query("get_check_in_nonce", &args).await?;
        #[allow(clippy::double_parens)]
        Decode!(response.as_slice(), Vec<u8>)
            .map_err(|e| format!("Failed to decode response: {}", e))
    }

    pub async fn data_fetch(
        &self,
        cursor: Option<String>,
        bytes_before: Option<Vec<u8>>,
    ) -> Result<(String, Vec<u8>), String> {
        let args = Encode!(&cursor, &bytes_before).map_err(|e| e.to_string())?;
        let response = self.call_query("data_fetch", &args).await?;
        #[allow(clippy::double_parens)]
        Decode!(response.as_slice(), Result<(String, Vec<u8>), String>)
            .map_err(|e| e.to_string())?
    }

    async fn get_logs_by_method(&self, method: &str) -> Result<String, String> {
        let args = Encode!(&()).map_err(|e| e.to_string())?;
        let response = self.call_query(method, &args).await?;
        #[allow(clippy::double_parens)]
        Decode!(response.as_slice(), ResultString).map_err(|e| e.to_string())?
    }

    pub async fn get_logs(&self, level: Level) -> Result<String, String> {
        let method = match level {
            Level::Error => "get_logs_error",
            Level::Warn => "get_logs_warn",
            Level::Info => "get_logs_info",
            Level::Debug => "get_logs_debug",
            Level::Trace => {
                return Err("Trace logs are not supported by the ledger canister".to_string())
            }
        };

        self.get_logs_by_method(method).await
    }

    pub async fn get_logs_debug(&self) -> Result<String, String> {
        self.get_logs(Level::Debug).await
    }

    pub async fn get_logs_info(&self) -> Result<String, String> {
        self.get_logs(Level::Info).await
    }

    pub async fn get_logs_warn(&self) -> Result<String, String> {
        self.get_logs(Level::Warn).await
    }

    pub async fn get_logs_error(&self) -> Result<String, String> {
        self.get_logs(Level::Error).await
    }
}
