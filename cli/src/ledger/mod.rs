mod data_operations;
mod metadata;

pub use data_operations::{ledger_data_fetch, ledger_data_push};
pub use metadata::get_ledger_metadata;

use candid::{Decode, Encode, Nat};
use dcc_common::{amount_as_string, DccIdentity, IcrcCompatibleAccount, TokenAmountE9s};
use decent_cloud::ledger_canister_client::LedgerCanister;
use decent_cloud_canister::DC_TOKEN_TRANSFER_FEE_E9S;
use icrc_ledger_types::{
    icrc1::transfer::TransferArg, icrc1::transfer::TransferError as Icrc1TransferError,
};

use crate::identity::dcc_to_ic_auth;

pub async fn handle_funds_transfer(
    network_url: &str,
    ledger_canister_id: candid::Principal,
    from_dcc_id: &DccIdentity,
    to_icrc1_account: &IcrcCompatibleAccount,
    transfer_amount_e9s: TokenAmountE9s,
) -> Result<String, Box<dyn std::error::Error>> {
    let from_icrc1_account = from_dcc_id.as_icrc_compatible_account();
    let from_ic_auth = dcc_to_ic_auth(from_dcc_id);

    println!(
        "Transferring {} tokens from {} \t to account {}",
        amount_as_string(transfer_amount_e9s),
        from_icrc1_account,
        to_icrc1_account,
    );

    let canister = LedgerCanister::new(ledger_canister_id, from_ic_auth, network_url).await?;
    let transfer_args = TransferArg {
        amount: transfer_amount_e9s.into(),
        fee: Some(DC_TOKEN_TRANSFER_FEE_E9S.into()),
        from_subaccount: None,
        to: to_icrc1_account.into(),
        created_at_time: None,
        memo: None,
    };
    let args = Encode!(&transfer_args).map_err(|e| e.to_string())?;
    let result = canister.call_update("icrc1_transfer", &args).await?;
    let response = Decode!(&result, Result<Nat, Icrc1TransferError>).map_err(|e| e.to_string())?;

    match response {
        Ok(block_num) => Ok(format!(
            "Transfer request successful, will be included in block: {}",
            block_num
        )),
        Err(e) => Err(Box::<dyn std::error::Error>::from(format!(
            "Transfer error: {}",
            e
        ))),
    }
}
