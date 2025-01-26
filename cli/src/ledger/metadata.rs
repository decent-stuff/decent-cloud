use candid::encode_one;
use decent_cloud::ledger_canister_client::LedgerCanister;
use icrc_ledger_types::icrc::generic_metadata_value::MetadataValue;
use std::collections::HashMap;

pub async fn get_ledger_metadata(
    ledger_canister: &LedgerCanister,
) -> HashMap<String, MetadataValue> {
    let no_args = encode_one(()).expect("Failed to encode empty tuple");
    let response = ledger_canister
        .call_query("metadata", &no_args)
        .await
        .expect("Failed to call ledger canister");
    candid::decode_one::<Vec<(String, MetadataValue)>>(&response)
        .expect("Failed to decode metadata")
        .into_iter()
        .collect()
}
