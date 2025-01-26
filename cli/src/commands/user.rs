use crate::argparse::UserCommands;
use crate::identity::{list_identities, list_local_identities, ListIdentityType};
use candid::{Decode, Encode};
use dcc_common::DccIdentity;
use decent_cloud::ledger_canister_client::LedgerCanister;
use ledger_map::LedgerMap;
use std::path::PathBuf;

pub async fn handle_user_command(
    user_cmd: UserCommands,
    network_url: &str,
    ledger_canister_id: candid::Principal,
    identity: Option<String>,
    ledger_local: LedgerMap,
) -> Result<(), Box<dyn std::error::Error>> {
    match user_cmd {
        UserCommands::List(list_args) => {
            if list_args.only_local {
                list_local_identities(list_args.balances)?
            } else {
                list_identities(&ledger_local, ListIdentityType::Users, list_args.balances)?
            }
        }
        UserCommands::Register => {
            let identity =
                identity.expect("Identity must be specified for this command, use --identity");
            let dcc_id = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;

            let canister =
                LedgerCanister::new_with_dcc_id(network_url, ledger_canister_id, &dcc_id).await?;
            let pubkey_bytes = dcc_id.to_bytes_verifying();
            let pubkey_signature = dcc_id.sign(&pubkey_bytes)?;
            let args = Encode!(&pubkey_bytes, &pubkey_signature.to_bytes())?;
            let result = canister.call_update("user_register", &args).await?.to_vec();
            let response = Decode!(&result, Result<String, String>).map_err(|e| e.to_string())?;

            match response {
                Ok(response) => {
                    println!("Registration successful: {}", response);
                }
                Err(e) => {
                    println!("Registration failed: {}", e);
                }
            }
        }
    }
    Ok(())
}
