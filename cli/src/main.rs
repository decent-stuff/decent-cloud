mod argparse;
mod keygen;

use argparse::{
    Commands, ContractCommands, LedgerRemoteCommands, NpCommands, OfferingCommands, UserCommands,
};
// use borsh::{BorshDeserialize, BorshSerialize};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use bip39::Seed;
use candid::{Decode, Encode, Nat, Principal as IcPrincipal};
use chrono::DateTime;
use dcc_common::{
    account_balance_get_as_string, amount_as_string, cursor_from_data,
    offerings::do_get_matching_offerings, refresh_caches_from_ledger, reputation_get,
    CursorDirection, DccIdentity, FundsTransfer, IcrcCompatibleAccount, LedgerCursor,
    TokenAmountE9s, DATA_PULL_BYTES_BEFORE_LEN, DC_TOKEN_DECIMALS_DIV, LABEL_DC_TOKEN_TRANSFER,
};
use dcc_common::{
    ContractSignReply, ContractSignRequest, PaymentEntries, PaymentEntry, PaymentEntryWithAmount,
    LABEL_NP_REGISTER, LABEL_USER_REGISTER,
};
use decent_cloud::ledger_canister_client::LedgerCanister;
use decent_cloud_canister::DC_TOKEN_TRANSFER_FEE_E9S;
use fs_err::OpenOptions;
use ic_agent::identity::BasicIdentity;
use icrc_ledger_types::{
    icrc::generic_metadata_value::MetadataValue, icrc1::transfer::TransferArg,
    icrc1::transfer::TransferError as Icrc1TransferError,
};
use ledger_map::{platform_specific::persistent_storage_read, LedgerMap};
use log::{info, Level};
use np_offering::Offering;
use std::time::SystemTime;
use std::{
    collections::HashMap,
    io::{self, BufReader, Seek, Write},
    path::PathBuf,
};
use tabular::{Row, Table};

const PUSH_BLOCK_SIZE: u64 = 1024 * 1024;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = argparse::parse_args();
    init_logger(cli.verbose);

    let ledger_path = cli.local_ledger_dir.map(PathBuf::from).unwrap_or_else(|| {
        dirs::home_dir()
            .expect("Could not get home directory")
            .join(".dcc")
            .join("ledger")
            .join("main.bin")
    });

    let ledger_local =
        LedgerMap::new_with_path(None, Some(ledger_path)).expect("Failed to load the local ledger");
    refresh_caches_from_ledger(&ledger_local).expect("Failed to get balances");

    let network = cli.network.unwrap_or_else(|| "ic".to_string());

    let ledger_canister_id = match network.as_str() {
        "local" => IcPrincipal::from_text("bkyz2-fmaaa-aaaaa-qaaaq-cai")?,
        "mainnet-eu" => IcPrincipal::from_text("tlvs5-oqaaa-aaaas-aaabq-cai")?,
        "mainnet-01" | "ic" => IcPrincipal::from_text("ggi4a-wyaaa-aaaai-actqq-cai")?,
        "mainnet-02" => IcPrincipal::from_text("gplx4-aqaaa-aaaai-actra-cai")?,
        _ => panic!("unknown network: {}", network),
    };
    let network_url = match network.as_str() {
        "local" => "http://127.0.0.1:8000",
        "mainnet-eu" | "mainnet-01" | "mainnet-02" | "ic" => "https://ic0.app",
        _ => panic!("unknown network: {}", network),
    };
    let ledger_canister = |identity| async {
        LedgerCanister::new(ledger_canister_id, identity, network_url.to_string()).await
    };
    let identity_name = cli.identity.clone();

    match cli.command {
        Commands::Keygen(ref keygen_args) => {
            let identity =
                identity_name.expect("Identity must be specified for this command, use --identity");

            let mnemonic = if keygen_args.generate {
                let mnemonic =
                    bip39::Mnemonic::new(bip39::MnemonicType::Words12, bip39::Language::English);
                info!("Generated mnemonic:\n{}", mnemonic);
                mnemonic
            } else if keygen_args.mnemonic.is_some() {
                let mnemonic_string = keygen_args
                    .mnemonic
                    .clone()
                    .unwrap_or_default()
                    .split_whitespace()
                    .map(String::from)
                    .collect::<Vec<_>>();
                if mnemonic_string.len() < 12 {
                    let reader = BufReader::new(io::stdin());
                    keygen::mnemonic_from_stdin(reader, io::stdout())?
                } else {
                    keygen::mnemonic_from_strings(mnemonic_string)?
                }
            } else {
                panic!("Neither mnemonic nor generate specified");
            };

            let seed = Seed::new(&mnemonic, "");
            let dcc_identity = DccIdentity::new_from_seed(seed.as_bytes())?;
            info!("Generated identity: {}", dcc_identity);
            dcc_identity.save_to_dir(&identity)
        }
        Commands::Account(ref account_args) => {
            let identities_dir = DccIdentity::identities_dir();
            let identity =
                identity_name.expect("Identity must be specified for this command, use --identity");
            let from_dcc_id = DccIdentity::load_from_dir(&identities_dir.join(identity))?;
            let to_principal_string = &account_args
                .transfer_to
                .clone()
                .expect("You must specify --transfer-to");
            let to_icrc1_account = IcrcCompatibleAccount::from(to_principal_string);
            let transfer_amount_e9s = match &account_args.amount_dct {
                Some(value) => (value.parse::<f64>()? * (DC_TOKEN_DECIMALS_DIV as f64)).round()
                    as TokenAmountE9s,
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
                    &from_dcc_id,
                    &to_icrc1_account,
                    transfer_amount_e9s,
                )
                .await?
            );

            Ok(())
        }
        Commands::Np(ref np_cmd) => {
            match np_cmd {
                NpCommands::List(list_args) => {
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
                NpCommands::Register => {
                    let identity = identity_name
                        .expect("Identity must be specified for this command, use --identity");
                    let dcc_ident = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;
                    let ic_auth = dcc_to_ic_auth(&dcc_ident);

                    info!("Registering principal: {} as {}", identity, dcc_ident);
                    let pubkey_bytes = dcc_ident.to_bytes_verifying();
                    let pubkey_signature = dcc_ident.sign(pubkey_bytes.as_ref())?;
                    let result = ledger_canister(ic_auth)
                        .await?
                        .node_provider_register(
                            &pubkey_bytes,
                            pubkey_signature.to_bytes().as_slice(),
                        )
                        .await?;
                    println!("Register: {}", result);
                }
                NpCommands::CheckIn(check_in_args) => {
                    if check_in_args.only_nonce {
                        let nonce_bytes = ledger_canister(None).await?.get_check_in_nonce().await;
                        let nonce_string = hex::encode(&nonce_bytes);

                        println!("0x{}", nonce_string);
                    } else {
                        let identity = identity_name
                            .expect("Identity must be specified for this command, use --identity");

                        let dcc_ident = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;
                        let ic_auth = dcc_to_ic_auth(&dcc_ident);

                        // Check the local ledger timestamp
                        let local_ledger_path = ledger_local
                            .get_file_path()
                            .expect("Failed to get local ledger path");
                        let local_ledger_file_mtime = local_ledger_path.metadata()?.modified()?;

                        // If the local ledger is older than 1 minute, refresh it automatically before proceeding
                        // If needed, the local ledger can also be refreshed manually from the command line
                        if local_ledger_file_mtime
                            < SystemTime::now() - std::time::Duration::from_secs(10)
                        {
                            info!("Local ledger is older than 1 minute, refreshing...");
                            let canister = ledger_canister(None).await?;
                            ledger_data_fetch(&canister, local_ledger_path).await?;

                            refresh_caches_from_ledger(&ledger_local)
                                .expect("Loading balances from ledger failed");
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
                        println!("No memo specified, did you know that you can specify one? Try out --memo");
                        String::new()
                    });
                        let nonce_crypto_signature = dcc_ident.sign(nonce_bytes.as_ref())?;
                        let result = ledger_canister(ic_auth)
                            .await?
                            .node_provider_check_in(
                                &dcc_ident.to_bytes_verifying(),
                                &check_in_memo,
                                &nonce_crypto_signature.to_bytes(),
                            )
                            .await
                            .map_err(|e| format!("Check-in failed: {}", e))?;
                        info!("Check-in success: {}", result);
                    }
                }
                NpCommands::UpdateProfile(update_profile_args) => {
                    let identity = identity_name
                        .expect("Identity must be specified for this command, use --identity");

                    let dcc_id = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;
                    let ic_auth = dcc_to_ic_auth(&dcc_id);

                    let np_profile =
                        np_profile::Profile::new_from_file(&update_profile_args.profile_file)?;
                    let np_profile_bytes = borsh::to_vec(&np_profile)?;
                    let crypto_signature = dcc_id.sign(&np_profile_bytes)?;

                    let result = ledger_canister(ic_auth)
                        .await?
                        .node_provider_update_profile(
                            &dcc_id.to_bytes_verifying(),
                            &np_profile_bytes,
                            &crypto_signature.to_bytes(),
                        )
                        .await
                        .map_err(|e| format!("Update profile failed: {}", e))?;
                    info!("Profile update response: {}", result);
                }
                NpCommands::UpdateOffering(update_offering_args) => {
                    let identity = identity_name
                        .expect("Identity must be specified for this command, use --identity");
                    let dcc_id = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;
                    let ic_auth = dcc_to_ic_auth(&dcc_id);

                    // Offering::new_from_file returns an error if the schema validation fails
                    let np_offering =
                        np_offering::Offering::new_from_file(&update_offering_args.offering_file)?;
                    let np_offering_bytes = np_offering.serialize()?;
                    let crypto_signature = dcc_id.sign(&np_offering_bytes)?;

                    let result = ledger_canister(ic_auth)
                        .await?
                        .node_provider_update_offering(
                            &dcc_id.to_bytes_verifying(),
                            &np_offering_bytes,
                            &crypto_signature.to_bytes(),
                        )
                        .await
                        .map_err(|e| format!("Update offering failed: {}", e))?;
                    info!("Offering update response: {}", result);
                }
            }
            Ok(())
        }
        Commands::User(ref user_cmd) => {
            match user_cmd {
                UserCommands::List(list_args) => {
                    if list_args.only_local {
                        list_local_identities(list_args.balances)?
                    } else {
                        list_identities(&ledger_local, ListIdentityType::Users, list_args.balances)?
                    }
                }
                UserCommands::Register => {
                    let identity = identity_name
                        .expect("Identity must be specified for this command, use --identity");
                    let dcc_id = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;
                    let ic_auth = dcc_to_ic_auth(&dcc_id);

                    let canister = ledger_canister(ic_auth).await?;
                    let pubkey_bytes = dcc_id.to_bytes_verifying();
                    let pubkey_signature = dcc_id.sign(&pubkey_bytes)?;
                    let args = Encode!(&pubkey_bytes, &pubkey_signature.to_bytes())?;
                    let result = canister.call_update("user_register", &args).await?;
                    let response =
                        Decode!(&result, Result<String, String>).map_err(|e| e.to_string())?;

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
        Commands::LedgerLocal(ref local_args) => {
            if local_args.list_entries {
                println!("Entries:");
                for entry in ledger_local.iter(None) {
                    match entry.label() {
                        LABEL_DC_TOKEN_TRANSFER => {
                            let transfer_id = BASE64.encode(entry.key());
                            let transfer: FundsTransfer =
                                borsh::from_slice(entry.value()).map_err(|e| e.to_string())?;
                            println!("[DCTokenTransfer] TransferId {}: {}", transfer_id, transfer);
                        }
                        _ => println!("{}", entry),
                    };
                }
            } else if local_args.list_entries_raw {
                println!("Raw Entries:");
                for entry in ledger_local.iter_raw() {
                    let (blk_header, ledger_block) = entry?;
                    println!("{}", blk_header);
                    println!("{}", ledger_block)
                }
            }
            Ok(())
        }
        Commands::LedgerRemote(ref subcmd) => {
            // TODO: Switch to subcommands
            let local_ledger_path = ledger_local
                .get_file_path()
                .expect("Failed to get local ledger path");

            match subcmd {
                LedgerRemoteCommands::DataFetch => {
                    let canister = ledger_canister(None).await?;
                    return ledger_data_fetch(&canister, local_ledger_path).await;
                }
                LedgerRemoteCommands::DataPushAuthorize | LedgerRemoteCommands::DataPush => {
                    let identity = identity_name
                        .expect("Identity must be specified for this command, use --identity");

                    let dcc_ident = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;

                    let push_auth = *subcmd == LedgerRemoteCommands::DataPushAuthorize;

                    if push_auth {
                        let ic_auth = dcc_to_ic_auth(&dcc_ident);
                        let canister = ledger_canister(ic_auth).await?;
                        let args = Encode!(&()).map_err(|e| e.to_string())?;
                        let result = canister.call_update("data_push_auth", &args).await?;
                        let response = Decode!(&result, Result<String, String>)
                            .map_err(|e| e.to_string())??;

                        println!("Push auth: {}", response);
                    }

                    // After authorizing, we can push the data
                    let ic_auth = dcc_to_ic_auth(&dcc_ident);
                    let canister = ledger_canister(ic_auth).await?;

                    return ledger_data_push(&canister, local_ledger_path).await;
                }
                LedgerRemoteCommands::Metadata => {
                    let canister = ledger_canister(None).await?;

                    let mut table = Table::new("{:<}  {:<}");
                    table.add_row(Row::new().with_cell("Key").with_cell("Value"));

                    for md_entry in get_ledger_metadata(&canister).await {
                        let md_entry_val = match md_entry.1 {
                            MetadataValue::Nat(v) => v.to_string(),
                            MetadataValue::Int(v) => v.to_string(),
                            MetadataValue::Text(v) => v.to_string(),
                            MetadataValue::Blob(v) => hex::encode(v),
                        };
                        table.add_row(Row::new().with_cell(md_entry.0).with_cell(md_entry_val));
                    }
                    print!("{}", table);
                }
                LedgerRemoteCommands::GetRegistrationFee => {
                    let canister = ledger_canister(None).await?;
                    let noargs = Encode!(&()).expect("Failed to encode args");
                    let response = canister.call_query("get_registration_fee", &noargs).await?;
                    let fee_e9s = Decode!(response.as_slice(), u64).map_err(|e| e.to_string())?;
                    println!("Registration fee: {}", amount_as_string(fee_e9s));
                }
                LedgerRemoteCommands::GetCheckInNonce => {
                    let nonce_bytes = ledger_canister(None).await?.get_check_in_nonce().await;
                    println!("{}", hex::encode(nonce_bytes));
                }
                LedgerRemoteCommands::GetLogsDebug => {
                    println!("Ledger canister DEBUG logs:");
                    for entry in serde_json::from_str::<Vec<serde_json::Value>>(
                        &ledger_canister(None).await?.get_logs_debug().await?,
                    )?
                    .into_iter()
                    {
                        log_with_level(entry, Level::Debug);
                    }
                }
                LedgerRemoteCommands::GetLogsInfo => {
                    println!("Ledger canister INFO logs:");
                    for entry in serde_json::from_str::<Vec<serde_json::Value>>(
                        &ledger_canister(None).await?.get_logs_info().await?,
                    )?
                    .into_iter()
                    {
                        log_with_level(entry, Level::Info);
                    }
                }
                LedgerRemoteCommands::GetLogsWarn => {
                    println!("Ledger canister WARN logs:");
                    for entry in serde_json::from_str::<Vec<serde_json::Value>>(
                        &ledger_canister(None).await?.get_logs_warn().await?,
                    )?
                    .into_iter()
                    {
                        log_with_level(entry, Level::Warn);
                    }
                }
                LedgerRemoteCommands::GetLogsError => {
                    println!("Ledger canister ERROR logs:");
                    for entry in serde_json::from_str::<Vec<serde_json::Value>>(
                        &ledger_canister(None).await?.get_logs_error().await?,
                    )?
                    .into_iter()
                    {
                        log_with_level(entry, Level::Error);
                    }
                }
            }

            fn log_with_level(log_entry: serde_json::Value, log_level: Level) {
                let timestamp_ns = log_entry["timestamp"].as_u64().unwrap_or_default();
                let timestamp_s = (timestamp_ns / 1_000_000_000) as i64;
                // Create DateTime from the timestamp
                let dt = DateTime::from_timestamp(timestamp_s, 0).unwrap_or_default();
                println!(
                    "{} [{}] - {}",
                    dt.format("%Y-%m-%dT%H:%M:%S"),
                    log_level,
                    log_entry["message"]
                        .as_str()
                        .expect("Invalid message field")
                );
            }

            Ok(())
        }
        Commands::Offering(ref cmd) => {
            let query = match cmd {
                OfferingCommands::List => "",
                OfferingCommands::Query(query_args) => &query_args.query,
            };
            let offerings = do_get_matching_offerings(&ledger_local, query);
            println!("Found {} matching offerings:", offerings.len());
            for (dcc_id, offering) in offerings {
                println!(
                    "{} ==>\n{}",
                    dcc_id.display_as_ic_and_pem_one_line(),
                    &offering.as_json_string_pretty().unwrap_or_default()
                );
            }

            Ok(())
        }
        Commands::Contract(ref contract_args) => match contract_args {
            ContractCommands::ListOpen(_list_open_args) => {
                println!("Listing all open contracts...");
                // A user may provide the identity (public key), but doesn't have to
                let pubkey_bytes = cli.identity.map(|name| {
                    let dcc_id = DccIdentity::load_from_dir(&PathBuf::from(&name)).unwrap();
                    dcc_id.to_bytes_verifying()
                });
                let canister = ledger_canister(None).await?;
                let contracts_open = canister.contracts_list_pending(&pubkey_bytes).await;
                if contracts_open.is_empty() {
                    println!("No open contracts");
                } else {
                    for open_contract in contracts_open {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&open_contract).unwrap_or_default()
                        );
                    }
                }
                Ok(())
            }
            ContractCommands::SignRequest(sign_req_args) => {
                println!("Request to sign a contract...");
                loop {
                    println!();
                    let i = sign_req_args.interactive;
                    let identity =
                        prompt_input("Please enter the identity name", &cli.identity, i, false);
                    let instance_id = prompt_input(
                        "Please enter the offering id",
                        &sign_req_args.offering_id,
                        i,
                        false,
                    );
                    let requester_ssh_pubkey=prompt_input(
                        "Please enter your ssh public key, which will be granted access to the contract",
                        &sign_req_args.requester_ssh_pubkey, i, false
                    );
                    let requester_contact = prompt_input(
                        "Enter your contact information (this will be public)",
                        &sign_req_args.requester_contact,
                        i,
                        true,
                    );
                    let provider_pubkey_pem = sign_req_args
                        .provider_pubkey_pem
                        .clone()
                        .unwrap_or_else(|| {
                            prompt_editor(
                                "# Enter the provider's public key below, as a PEM string",
                                i,
                            )
                            .lines()
                            .map(|line| {
                                line.split_once('#')
                                    .map(|line| line.0)
                                    .unwrap_or(line)
                                    .trim()
                                    .to_string()
                            })
                            .filter(|line| !line.is_empty())
                            .collect::<Vec<String>>()
                            .join("\n")
                        });
                    let provider_dcc_id =
                        match DccIdentity::new_verifying_from_pem(&provider_pubkey_pem) {
                            Ok(ident) => ident,
                            Err(e) => {
                                eprintln!("ERROR: Failed to parse provider pubkey: {}", e);
                                continue;
                            }
                        };
                    let provider_pubkey_bytes = provider_dcc_id.to_bytes_verifying();
                    // Find the offering with the given id, from the provider
                    let offerings = do_get_matching_offerings(
                        &ledger_local,
                        &format!("instance_types.id = \"{instance_id}\""),
                    )
                    .into_iter()
                    .filter(|o| o.0.to_bytes_verifying() == provider_pubkey_bytes)
                    .collect::<Vec<(DccIdentity, Offering)>>();

                    let offering = match offerings.len() {
                        0 => {
                            eprintln!(
                                "ERROR: No offering found for the provider {provider_dcc_id} and id: {instance_id}"
                            );
                            continue;
                        }
                        1 => &offerings[0].1,
                        _ => {
                            eprintln!("ERROR: Provider {provider_dcc_id} has multiple offerings with id: {instance_id}");
                            continue;
                        }
                    };

                    let payment_entries = prompt_for_payment_entries(
                        &sign_req_args.payment_entries_json,
                        offering,
                        &instance_id,
                    );

                    let payment_amount_e9s = payment_entries.iter().map(|e| e.amount_e9s).sum();

                    let memo = prompt_input(
                        "Please enter a memo for the contract (this will be public)",
                        &sign_req_args.memo,
                        i,
                        true,
                    );

                    let dcc_id = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;

                    let requester_pubkey_bytes = dcc_id.to_bytes_verifying();
                    let req = ContractSignRequest::new(
                        &requester_pubkey_bytes,
                        requester_ssh_pubkey,
                        requester_contact,
                        &provider_pubkey_bytes,
                        instance_id.clone(),
                        None,
                        None,
                        None,
                        payment_amount_e9s,
                        payment_entries,
                        None,
                        memo.clone(),
                    );
                    println!("The following contract sign request will be sent:");
                    println!("{}", serde_json::to_string_pretty(&req)?);
                    if dialoguer::Confirm::new()
                        .with_prompt("Is this correct? If yes, press enter to send.")
                        .default(false)
                        .show_default(true)
                        .interact()
                        .unwrap()
                    {
                        let payload_bytes = borsh::to_vec(&req).unwrap();
                        let payload_sig_bytes = dcc_id.sign(&payload_bytes)?.to_bytes();
                        let ic_auth = dcc_to_ic_auth(&dcc_id);
                        let canister = ledger_canister(ic_auth).await?;

                        match canister
                            .contract_sign_request(
                                &requester_pubkey_bytes,
                                &payload_bytes,
                                &payload_sig_bytes,
                            )
                            .await
                        {
                            Ok(response) => {
                                println!("Contract sign request successful: {}", response);
                                break;
                            }
                            Err(e) => {
                                println!("Contract sign request failed: {}", e);
                                if dialoguer::Confirm::new()
                                    .with_prompt("Do you want to retry?")
                                    .default(true)
                                    .show_default(true)
                                    .interact()
                                    .unwrap()
                                {
                                    continue;
                                } else {
                                    break;
                                }
                            }
                        }
                    } else {
                        println!("Contract sign request canceled.");
                        break;
                    }
                }
                Ok(())
            }
            ContractCommands::SignReply(sign_reply_args) => {
                println!("Reply to a contract-sign request...");
                loop {
                    let i = sign_reply_args.interactive;
                    let identity =
                        prompt_input("Please enter the identity name", &cli.identity, i, false);
                    let contract_id = prompt_input(
                        "Please enter the contract id, as a base64 encoded string",
                        &sign_reply_args.contract_id,
                        i,
                        false,
                    );
                    let accept = prompt_bool(
                        "Do you accept the contract?",
                        sign_reply_args.sign_accept,
                        i,
                    );
                    let response_text = prompt_input(
                        "Please enter a response text for the contract (this will be public)",
                        &sign_reply_args.response_text,
                        i,
                        true,
                    );
                    let response_details = prompt_input(
                        "Please enter a response details for the contract (this will be public)",
                        &sign_reply_args.response_details,
                        i,
                        true,
                    );

                    let dcc_id = DccIdentity::load_from_dir(&PathBuf::from(&identity)).unwrap();
                    let provider_pubkey_bytes = dcc_id.to_bytes_verifying();
                    let ic_auth = dcc_to_ic_auth(&dcc_id);
                    let canister = ledger_canister(ic_auth).await?;

                    let contracts_open = canister
                        .contracts_list_pending(&Some(provider_pubkey_bytes.clone()))
                        .await;
                    let open_contract = contracts_open
                        .iter()
                        .find(|c| c.contract_id_base64 == contract_id)
                        .expect("Provided contract id not found");

                    let contract_id_bytes = BASE64.decode(contract_id.as_bytes()).unwrap();

                    let reply = ContractSignReply::new(
                        open_contract.contract_req.requester_pubkey_bytes().to_vec(),
                        open_contract.contract_req.request_memo(),
                        contract_id_bytes,
                        accept,
                        &response_text,
                        &response_details,
                    );

                    let payload_bytes = borsh::to_vec(&reply).unwrap();
                    let signature = dcc_id.sign(&payload_bytes)?.to_vec();

                    match canister
                        .contract_sign_reply(&provider_pubkey_bytes, &payload_bytes, &signature)
                        .await
                    {
                        Ok(response) => {
                            println!("Contract sign reply sent successfully: {}", response);
                            break;
                        }
                        Err(e) => {
                            println!("Error sending contract sign reply: {:?}", e);
                        }
                    }
                }
                Ok(())
            }
        },
    }?;
    Ok(())
}

fn prompt_input<S: ToString>(
    prompt_message: &str,
    cli_arg_value: &Option<S>,
    interactive: bool,
    allow_empty: bool,
) -> String {
    match cli_arg_value {
        Some(value) => value.to_string(),
        None => {
            if interactive {
                dialoguer::Input::<String>::new()
                    .with_prompt(prompt_message)
                    .allow_empty(allow_empty)
                    .show_default(false)
                    .interact()
                    .unwrap_or_default()
            } else {
                panic!("CLI argument required: {}", prompt_message)
            }
        }
    }
}

fn prompt_bool(prompt_message: &str, cli_arg_value: Option<bool>, interactive: bool) -> bool {
    match cli_arg_value {
        Some(value) => value,
        None => {
            if interactive {
                dialoguer::Confirm::new()
                    .with_prompt(prompt_message)
                    .default(false)
                    .show_default(true)
                    .interact()
                    .unwrap_or_default()
            } else {
                panic!("CLI argument required: {}", prompt_message)
            }
        }
    }
}

fn prompt_editor(prompt_message: &str, interactive: bool) -> String {
    if interactive {
        match dialoguer::Editor::new().edit(prompt_message) {
            Ok(Some(content)) => content,
            Ok(None) => {
                println!("No input received.");
                String::new()
            }
            Err(err) => {
                eprintln!("Error opening editor: {}", err);
                String::new()
            }
        }
    } else {
        panic!("CLI argument required: {}", prompt_message);
    }
}

/// We only allow one payment entry at a time, but this can be easily changed later in the CLI
fn prompt_for_payment_entries(
    payment_entries_json: &Option<PaymentEntries>,
    offering: &Offering,
    instance_id: &str,
) -> Vec<PaymentEntryWithAmount> {
    let pricing: HashMap<String, HashMap<String, String>> = offering.instance_pricing(instance_id);

    let get_total_price = |model: &str, time_period_unit: &str, quantity: u64| -> TokenAmountE9s {
        pricing
            .get(model)
            .and_then(|units| units.get(time_period_unit))
            .map(|amount| {
                amount
                    .replace("_", "")
                    .parse::<TokenAmountE9s>()
                    .expect("Failed to parse the offering price as TokenAmountE9s")
                    * quantity
            })
            .unwrap()
    };
    let mut payment_entries: Vec<_> = payment_entries_json
        .clone()
        .map(|entries| {
            entries
                .0
                .into_iter()
                .map(|e| PaymentEntryWithAmount {
                    e: e.clone(),
                    amount_e9s: get_total_price(&e.pricing_model, &e.time_period_unit, e.quantity),
                })
                .collect()
        })
        .unwrap_or_default();

    if payment_entries.is_empty() {
        let models = pricing.keys().collect::<Vec<_>>();
        let model = models[dialoguer::Select::new()
            .with_prompt("Please select instance pricing model (ESC to exit)")
            .items(&models)
            .default(0)
            .interact()
            .expect("Failed to read input")];
        let units = pricing[model].keys().collect::<Vec<_>>();
        let time_period_unit = units[dialoguer::Select::new()
            .with_prompt("Please select time period unit")
            .items(&units)
            .report(true)
            .default(0)
            .interact()
            .expect("Failed to read input")];
        let quantity = dialoguer::Input::<u64>::new()
            .with_prompt("Please enter the number of units")
            .default(1)
            .interact()
            .expect("Failed to read input");
        payment_entries.push(PaymentEntryWithAmount {
            e: PaymentEntry::new(model, time_period_unit, quantity),
            amount_e9s: get_total_price(model, time_period_unit, quantity),
        });
    }
    payment_entries
}

fn list_local_identities(include_balances: bool) -> Result<(), Box<dyn std::error::Error>> {
    let identities_dir = DccIdentity::identities_dir();
    println!("Available identities at {}:", identities_dir.display());
    let mut identities: Vec<_> = fs_err::read_dir(identities_dir)?
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry),
            Err(e) => {
                eprintln!("Failed to read identity: {}", e);
                None
            }
        })
        .collect();

    identities.sort_by_key(|identity| identity.file_name());

    for identity in identities {
        let path = identity.path();
        if path.is_dir() {
            let identity_name = identity.file_name();
            let identity_name = identity_name.to_string_lossy();
            match DccIdentity::load_from_dir(&path) {
                Ok(dcc_identity) => {
                    print!("{} => ", identity_name);
                    println_identity(&dcc_identity, include_balances);
                }
                Err(e) => {
                    println!("{} => Error: {}", identity_name, e);
                }
            }
        }
    }
    Ok(())
}

#[derive(PartialEq)]
enum ListIdentityType {
    Providers,
    Users,
    All,
}

fn list_identities(
    ledger: &LedgerMap,
    identity_type: ListIdentityType,
    show_balances: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if identity_type == ListIdentityType::Providers || identity_type == ListIdentityType::All {
        println!("\n# Registered providers");
        for entry in ledger.iter(Some(LABEL_NP_REGISTER)) {
            let dcc_id = DccIdentity::new_verifying_from_bytes(entry.key()).unwrap();
            println_identity(&dcc_id, show_balances);
        }
    }
    if identity_type == ListIdentityType::Users || identity_type == ListIdentityType::All {
        println!("\n# Registered users");
        for entry in ledger.iter(Some(LABEL_USER_REGISTER)) {
            let dcc_id = DccIdentity::new_verifying_from_bytes(entry.key()).unwrap();
            println_identity(&dcc_id, show_balances);
        }
    }
    Ok(())
}

fn println_identity(dcc_id: &DccIdentity, show_balance: bool) {
    if show_balance {
        println!(
            "{}, reputation {}, balance {}",
            dcc_id.display_as_ic_and_pem_one_line(),
            reputation_get(dcc_id.to_bytes_verifying()),
            account_balance_get_as_string(&dcc_id.as_icrc_compatible_account())
        );
    } else {
        println!(
            "{} reputation {}",
            dcc_id.display_as_ic_and_pem_one_line(),
            reputation_get(dcc_id.to_bytes_verifying())
        );
    }
}

fn dcc_to_ic_auth(dcc_identity: &DccIdentity) -> Option<BasicIdentity> {
    dcc_identity
        .signing_key_as_ic_agent_pem_string()
        .map(|pem_key| {
            let cursor = std::io::Cursor::new(pem_key.as_bytes());
            BasicIdentity::from_pem(cursor).expect("failed to parse pem key")
        })
}

pub async fn handle_funds_transfer(
    network_url: &str,
    ledger_canister_id: IcPrincipal,
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

    let canister =
        LedgerCanister::new(ledger_canister_id, from_ic_auth, network_url.to_string()).await?;
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

async fn ledger_data_fetch(
    ledger_canister: &LedgerCanister,
    local_ledger_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut ledger_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&local_ledger_path)
        .expect("failed to open the local ledger path");

    let cursor_local = {
        let ledger = LedgerMap::new_with_path(None, Some(local_ledger_path.clone()))
            .expect("Failed to create LedgerMap");
        cursor_from_data(
            ledger_map::partition_table::get_data_partition().start_lba,
            ledger_map::platform_specific::persistent_storage_size_bytes(),
            ledger.get_next_block_start_pos(),
            ledger.get_next_block_start_pos(),
        )
    };

    let bytes_before = if cursor_local.position > DATA_PULL_BYTES_BEFORE_LEN as u64 {
        let mut buf = vec![0u8; DATA_PULL_BYTES_BEFORE_LEN as usize];
        persistent_storage_read(
            cursor_local.position - DATA_PULL_BYTES_BEFORE_LEN as u64,
            &mut buf,
        )?;
        Some(buf)
    } else {
        None
    };

    info!(
        "Fetching data from the Ledger canister {}, with local cursor: {} and bytes before: {:?}",
        ledger_canister.canister_id(),
        cursor_local,
        hex::encode(bytes_before.as_ref().unwrap_or(&vec![])),
    );
    let (cursor_remote, data) = ledger_canister
        .data_fetch(Some(cursor_local.to_request_string()), bytes_before)
        .await?;
    let cursor_remote = LedgerCursor::new_from_string(cursor_remote);
    let offset_remote = cursor_remote.position;
    info!(
        "Ledger canister returned position {:0x}, full cursor: {}",
        offset_remote, cursor_remote
    );
    if offset_remote < cursor_local.position {
        return Err(format!(
            "Ledger canister has less data than available locally {} < {} bytes",
            offset_remote, cursor_local.position
        )
        .into());
    }
    if data.len() <= 64 {
        info!("Data: {} bytes ==> {:?}", data.len(), data);
    } else {
        info!(
            "Data: {} bytes ==> {:?}...",
            data.len(),
            &data[..64.min(data.len())]
        );
    }
    let file_size_bytes = ledger_file.metadata().unwrap().len();
    let file_size_bytes_target = offset_remote + data.len() as u64 + 1024 * 1024;
    if file_size_bytes < file_size_bytes_target {
        ledger_file.set_len(file_size_bytes_target).unwrap();
        ledger_file
            .seek(std::io::SeekFrom::Start(offset_remote))
            .unwrap();
    }
    if offset_remote + cursor_remote.response_bytes > cursor_local.position {
        ledger_file.write_all(&data).unwrap();
        info!(
            "Wrote {} bytes at offset 0x{:0x} of file {}",
            data.len(),
            offset_remote,
            local_ledger_path.display()
        );
    }
    // Set the modified time to the current time, to mark that the data is up-to-date
    filetime::set_file_mtime(local_ledger_path, std::time::SystemTime::now().into())?;

    Ok(())
}

async fn get_ledger_metadata(ledger_canister: &LedgerCanister) -> HashMap<String, MetadataValue> {
    let no_args = candid::encode_one(()).expect("Failed to encode empty tuple");
    let response = ledger_canister
        .call_query("metadata", &no_args)
        .await
        .expect("Failed to call ledger canister");
    candid::decode_one::<Vec<(String, MetadataValue)>>(&response)
        .expect("Failed to decode metadata")
        .into_iter()
        .collect()
}

pub async fn ledger_data_push(
    ledger_canister: &LedgerCanister,
    local_ledger_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let ledger_local = LedgerMap::new_with_path(Some(vec![]), Some(local_ledger_path))
        .expect("Failed to create LedgerMap");
    let cursor_local = cursor_from_data(
        ledger_map::partition_table::get_data_partition().start_lba,
        ledger_map::platform_specific::persistent_storage_size_bytes(),
        ledger_local.get_next_block_start_pos(),
        ledger_local.get_next_block_start_pos(),
    );

    let remote_metadata = get_ledger_metadata(ledger_canister).await;
    let cursor_remote: LedgerCursor = remote_metadata.into();

    if cursor_local.data_end_position <= cursor_remote.data_end_position {
        info!("Nothing to push");
        return Ok(());
    }

    info!(
        "Data end position local {} remote {} ==> {} bytes to push",
        cursor_local.data_end_position,
        cursor_remote.data_end_position,
        cursor_local.data_end_position - cursor_remote.data_end_position
    );

    let last_i = (cursor_local
        .data_end_position
        .saturating_sub(cursor_local.data_begin_position))
        / PUSH_BLOCK_SIZE
        + 1;
    for i in 0..last_i {
        let position = (i * PUSH_BLOCK_SIZE).max(cursor_local.data_begin_position);

        let cursor_push = LedgerCursor::new(
            cursor_local.data_begin_position,
            position,
            cursor_local.data_end_position,
            CursorDirection::Forward,
            i + 1 < last_i,
        );

        let buf_size =
            PUSH_BLOCK_SIZE.min(cursor_local.data_end_position.saturating_sub(position)) as usize;
        let mut buf = vec![0u8; buf_size];
        persistent_storage_read(position, &mut buf).map_err(|e| e.to_string())?;
        info!(
            "Pushing block of {} bytes at position {}",
            buf_size, position,
        );
        let args = Encode!(&cursor_push.to_urlenc_string(), &buf).map_err(|e| e.to_string())?;
        let result = ledger_canister.call_update("data_push", &args).await?;
        let result = Decode!(&result, Result<String, String>).map_err(|e| e.to_string())??;
        info!("Response from pushing at position {}: {}", position, result);
    }

    Ok(())
}

pub fn init_logger(verbose: bool) {
    if std::env::var("RUST_LOG").is_err() {
        if verbose {
            std::env::set_var("RUST_LOG", "debug");
        } else {
            std::env::set_var("RUST_LOG", "info");
        }
    }
    pretty_env_logger::init();
}
