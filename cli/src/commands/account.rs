use crate::argparse::AccountArgs;
use crate::identity::{list_identities, ListIdentityType};
use crate::ledger::handle_funds_transfer;
use crate::LedgerMap;
use candid::Principal as IcPrincipal;
use dcc_common::{
    account_balance_get_as_string, DccIdentity, IcrcCompatibleAccount, TokenAmountE9s,
    DC_TOKEN_DECIMALS_DIV, DC_TOKEN_SYMBOL,
};
use std::path::PathBuf;

pub async fn handle_account_command(
    account_args: AccountArgs,
    network_url: &str,
    ledger_canister_id: IcPrincipal,
    identity: Option<String>,
    ledger_local: &LedgerMap,
) -> Result<(), Box<dyn std::error::Error>> {
    if account_args.list_all {
        return list_identities(ledger_local, ListIdentityType::All, true);
    }

    let identity = identity.expect("Identity must be specified for this command, use --identity");
    let dcc_id = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;

    println!("Account Principal ID: {}", dcc_id);
    println!(
        "Account balance: {} {}",
        account_balance_get_as_string(&dcc_id.as_icrc_compatible_account()),
        DC_TOKEN_SYMBOL
    );

    if let Some(to_principal_string) = &account_args.transfer_to {
        let to_icrc1_account = IcrcCompatibleAccount::from(to_principal_string);

        let transfer_amount_e9s = match &account_args.amount_dct {
            Some(value) => {
                (value.parse::<f64>()? * (DC_TOKEN_DECIMALS_DIV as f64)).round() as TokenAmountE9s
            }
            None => match &account_args.amount_e9s {
                Some(value) => value.parse::<TokenAmountE9s>()?,
                None => {
                    panic!("You must specify either --amount-dct or --amount-e9s")
                }
            },
        };

        println!(
            "{}",
            handle_funds_transfer(
                network_url,
                ledger_canister_id,
                &dcc_id,
                &to_icrc1_account,
                transfer_amount_e9s,
            )
            .await?
        );
    }

    Ok(())
}
