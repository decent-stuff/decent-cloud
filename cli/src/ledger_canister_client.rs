use candid::{Decode, Encode};
use ic_agent::{export::Principal, identity::BasicIdentity, Agent};

type ResultString = Result<String, String>;

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
        network_url: String,
    ) -> anyhow::Result<Self> {
        let agent = Agent::builder().with_url(&network_url);
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
            // network_url,
        })
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

    pub fn list_functions_updates(&self) -> Vec<String> {
        vec![
            "init_ledger_map".to_string(),
            "node_provider_register".to_string(),
            "node_provider_check_in".to_string(),
            "node_provider_update_profile".to_string(),
            "data_push".to_string(),
            "data_push_auth".to_string(),
        ]
    }

    pub fn list_functions_queries(&self) -> Vec<String> {
        vec![
            "get_np_check_in_nonce".to_string(),
            "data_fetch".to_string(),
            "metadata".to_string(),
            "get_logs_debug".to_string(),
            "get_logs_info".to_string(),
            "get_logs_warn".to_string(),
            "get_logs_error".to_string(),
        ]
    }

    pub async fn init_ledger_map(&self) -> Result<String, String> {
        let args = Encode!(&()).map_err(|e| e.to_string())?;
        let response = self.call_update("init_ledger_map", &args).await?;
        Decode!(response.as_slice(), ResultString).map_err(|e| e.to_string())?
    }

    pub async fn node_provider_register(&self, key: &[u8], value: &[u8]) -> Result<String, String> {
        let args = Encode!(&key, &value).map_err(|e| e.to_string())?;
        let response = self.call_update("node_provider_register", &args).await?;
        Decode!(response.as_slice(), ResultString).map_err(|e| e.to_string())?
    }

    pub async fn node_provider_check_in(&self, key: &[u8], value: &[u8]) -> Result<String, String> {
        let args = Encode!(&key, &value).map_err(|e| e.to_string())?;
        let response = self.call_update("node_provider_check_in", &args).await?;
        Decode!(response.as_slice(), ResultString).map_err(|e| e.to_string())?
    }

    pub async fn node_provider_update_profile(
        &self,
        key: &[u8],
        value: &[u8],
    ) -> Result<String, String> {
        let args = Encode!(&key, &value).map_err(|e| e.to_string())?;
        let response = self
            .call_update("node_provider_update_profile", &args)
            .await?;
        Decode!(response.as_slice(), ResultString).map_err(|e| e.to_string())?
    }

    pub async fn node_provider_update_offering(
        &self,
        key: &[u8],
        value: &[u8],
    ) -> Result<String, String> {
        let args = Encode!(&key, &value).map_err(|e| e.to_string())?;
        let response = self
            .call_update("node_provider_update_offering", &args)
            .await?;
        Decode!(response.as_slice(), ResultString).map_err(|e| e.to_string())?
    }

    pub async fn get_np_check_in_nonce(&self) -> Vec<u8> {
        let args = Encode!(&()).expect("Failed to encode args");
        let response = self
            .call_query("get_np_check_in_nonce", &args)
            .await
            .expect("Failed to call get_np_check_in_nonce");
        Decode!(response.as_slice(), Vec<u8>).expect("Failed to decode response")
    }

    pub async fn data_fetch(
        &self,
        cursor: Option<String>,
        bytes_before: Option<Vec<u8>>,
    ) -> Result<(String, Vec<u8>), String> {
        let args = Encode!(&cursor, &bytes_before).map_err(|e| e.to_string())?;
        let response = self.call_query("data_fetch", &args).await?;
        Decode!(response.as_slice(), Result<(String, Vec<u8>), String>)
            .map_err(|e| e.to_string())?
    }

    pub async fn get_logs_debug(&self) -> Result<String, String> {
        let args = Encode!(&()).map_err(|e| e.to_string())?;
        let response = self.call_query("get_logs_debug", &args).await?;
        Decode!(response.as_slice(), ResultString).map_err(|e| e.to_string())?
    }

    pub async fn get_logs_info(&self) -> Result<String, String> {
        let args = Encode!(&()).map_err(|e| e.to_string())?;
        let response = self.call_query("get_logs_info", &args).await?;
        Decode!(response.as_slice(), ResultString).map_err(|e| e.to_string())?
    }

    pub async fn get_logs_warn(&self) -> Result<String, String> {
        let args = Encode!(&()).map_err(|e| e.to_string())?;
        let response = self.call_query("get_logs_warn", &args).await?;
        Decode!(response.as_slice(), ResultString).map_err(|e| e.to_string())?
    }

    pub async fn get_logs_error(&self) -> Result<String, String> {
        let args = Encode!(&()).map_err(|e| e.to_string())?;
        let response = self.call_query("get_logs_error", &args).await?;
        Decode!(response.as_slice(), ResultString).map_err(|e| e.to_string())?
    }
}

// let canister_id = create_a_canister().await.unwrap();
// eprintln!("{}", canister_id);
