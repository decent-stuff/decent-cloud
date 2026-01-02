use candid::encode_one;
use decent_cloud::ledger_canister_client::LedgerCanister;
use icrc_ledger_types::icrc::generic_metadata_value::MetadataValue;
use std::collections::HashMap;

pub async fn get_ledger_metadata(
    ledger_canister: &LedgerCanister,
) -> Result<HashMap<String, MetadataValue>, Box<dyn std::error::Error>> {
    let no_args = encode_one(())
        .map_err(|e| anyhow::anyhow!("Failed to encode metadata query arguments: {}", e))?;
    let response = ledger_canister
        .call_query("metadata", &no_args)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query ledger metadata: {}", e))?;
    let metadata_vec = candid::decode_one::<Vec<(String, MetadataValue)>>(&response)
        .map_err(|e| anyhow::anyhow!("Failed to decode metadata response: {}", e))?;

    Ok(metadata_vec.into_iter().collect())
}
