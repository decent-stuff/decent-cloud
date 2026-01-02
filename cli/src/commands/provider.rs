use crate::argparse::ProviderCommands;
use crate::identity::{list_identities, list_local_identities, ListIdentityType};
use dcc_common::DccIdentity;
use decent_cloud::ledger_canister_client::LedgerCanister;
use ledger_map::LedgerMap;
use log::info;
use std::{path::PathBuf, time::SystemTime};

use crate::ledger::ledger_data_fetch;

pub async fn handle_provider_command(
    provider_cmd: ProviderCommands,
    network_url: &str,
    ledger_canister_id: candid::Principal,
    identity: Option<String>,
    mut ledger_local: LedgerMap,
) -> Result<(), Box<dyn std::error::Error>> {
    match provider_cmd {
        ProviderCommands::List(list_args) => {
            if list_args.only_local {
                list_local_identities(list_args.balances)?
            } else {
                list_identities(
                    &ledger_local,
                    ListIdentityType::Providers,
                    list_args.balances,
                )?
            }
        }
        ProviderCommands::Register => {
            let identity = identity.ok_or_else(|| {
                "Identity must be specified for this command. Use --identity <name>".to_string()
            })?;
            let dcc_id = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;

            info!("Registering principal: {} as {}", identity, dcc_id);
            let pubkey_bytes = dcc_id.to_bytes_verifying();
            let pubkey_signature = dcc_id.sign(pubkey_bytes.as_ref())?;
            let canister =
                LedgerCanister::new_with_dcc_id(network_url, ledger_canister_id, &dcc_id).await?;
            let result = canister
                .provider_register(&pubkey_bytes, pubkey_signature.to_bytes().as_slice())
                .await?;
            println!("Register: {}", result);
        }
        ProviderCommands::CheckIn(check_in_args) => {
            if check_in_args.only_nonce {
                let nonce_bytes =
                    LedgerCanister::new_without_identity(network_url, ledger_canister_id)
                        .await?
                        .get_check_in_nonce()
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to get check-in nonce: {}", e))?;
                let nonce_string = hex::encode(&nonce_bytes);

                println!("0x{}", nonce_string);
            } else {
                let identity = identity.ok_or_else(|| {
                    "Identity must be specified for this command. Use --identity <name>".to_string()
                })?;

                let dcc_ident = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;

                // Check the local ledger timestamp
                let local_ledger_path = ledger_local.get_file_path().ok_or_else(|| {
                    anyhow::anyhow!(
                        "Failed to get local ledger path. The ledger may not be initialized. \
                             Try using --local-ledger-dir flag to specify the ledger directory, \
                             or run 'ledger remote data-fetch' to initialize the local ledger."
                    )
                })?;
                let local_ledger_file_mtime = local_ledger_path.metadata()?.modified()?;

                // If the local ledger is older than 1 minute, refresh it automatically before proceeding
                // If needed, the local ledger can also be refreshed manually from the command line
                if local_ledger_file_mtime < SystemTime::now() - std::time::Duration::from_secs(10)
                {
                    info!("Local ledger is older than 1 minute, refreshing...");
                    let canister =
                        LedgerCanister::new_without_identity(network_url, ledger_canister_id)
                            .await?;
                    ledger_data_fetch(&canister, &mut ledger_local).await?;

                    dcc_common::refresh_caches_from_ledger(&ledger_local).map_err(|e| {
                        anyhow::anyhow!("Failed to load balances from ledger: {}", e)
                    })?;
                }
                // The local ledger needs to be refreshed to get the latest nonce
                // This provides the incentive to clone and frequently re-fetch the ledger
                let nonce_bytes = ledger_local.get_latest_block_hash();
                let nonce_string = hex::encode(&nonce_bytes);

                info!(
                    "Checking-in provider identity {} ({}), using nonce: {} ({} bytes)",
                    identity,
                    dcc_ident,
                    nonce_string,
                    nonce_bytes.len()
                );
                let check_in_memo = check_in_args.memo.clone().unwrap_or_else(|| {
                    println!(
                        "No memo specified, did you know that you can specify one? Try out --memo"
                    );
                    String::new()
                });
                let nonce_crypto_signature = dcc_ident.sign(nonce_bytes.as_ref())?;
                let canister =
                    LedgerCanister::new_with_dcc_id(network_url, ledger_canister_id, &dcc_ident)
                        .await?;
                let result = canister
                    .provider_check_in(
                        &dcc_ident.to_bytes_verifying(),
                        &check_in_memo,
                        &nonce_crypto_signature.to_bytes(),
                    )
                    .await
                    .map_err(|e| format!("Check-in failed: {}", e))?;
                info!("Check-in success: {}", result);
            }
        }
        ProviderCommands::UpdateProfile(_update_profile_args) => {
            let identity = identity.ok_or_else(|| {
                "Identity must be specified for this command. Use --identity <name>".to_string()
            })?;

            let _dcc_id = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;

            todo!("Update the profile in the decent-cloud api server, and sign it with the local identity");
        }
        ProviderCommands::UpdateOffering(_update_offering_args) => {
            let identity = identity.ok_or_else(|| {
                "Identity must be specified for this command. Use --identity <name>".to_string()
            })?;
            let _dcc_id = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;

            todo!("Update the offering in the decent-cloud api server, and sign it with the local identity");
        }
    }
    Ok(())
}
