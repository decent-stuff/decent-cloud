mod argparse;
mod keygen;

use argparse::{Commands, ContractCommands};
// use borsh::{BorshDeserialize, BorshSerialize};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use bip39::Seed;
use borsh::BorshDeserialize;
use candid::{Decode, Encode, Nat, Principal as IcPrincipal};
use chrono::DateTime;
use dcc_common::{
    account_balance_get_as_string, amount_as_string, cursor_from_data,
    offerings::do_get_matching_offerings, refresh_caches_from_ledger, reputation_get,
    CursorDirection, DccIdentity, FundsTransfer, IcrcCompatibleAccount, LedgerCursor, TokenAmount,
    DATA_PULL_BYTES_BEFORE_LEN, DC_TOKEN_DECIMALS_DIV, LABEL_DC_TOKEN_TRANSFER,
};
use dcc_common::{ContractSignRequest, ContractSignRequestPayload};
use decent_cloud::ledger_canister_client::LedgerCanister;
use decent_cloud_canister::DC_TOKEN_TRANSFER_FEE_E9S;
use fs_err::OpenOptions;
use ic_agent::identity::BasicIdentity;
use icrc_ledger_types::{
    icrc::generic_metadata_value::MetadataValue, icrc1::transfer::TransferArg,
    icrc1::transfer::TransferError as Icrc1TransferError,
};
use ledger_map::{platform_specific::persistent_storage_read, LedgerMap};
use log::{info, Level, LevelFilter, Metadata, Record};
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
    init_logger()?;

    let cli = argparse::parse_args();

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

    let canister_id = match network.as_str() {
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
        LedgerCanister::new(canister_id, identity, network_url.to_string()).await
    };
    let identity_name = cli.identity.clone();

    match cli.command {
        Commands::Keygen(ref keygen_args) => {
            let identity = identity_name.expect("is present");

            let mnemonic = if keygen_args.generate {
                let mnemonic =
                    bip39::Mnemonic::new(bip39::MnemonicType::Words12, bip39::Language::English);
                info!("Mnemonic:\n{}", mnemonic);
                mnemonic
            } else if !keygen_args.mnemonic.is_empty() {
                let length = keygen_args.mnemonic.len();
                if length != 12 && length != 24 {
                    panic!("Mnemonic must be exactly 12 or 24 words, but got {length}.");
                }
                let mnemonic_string = keygen_args
                    .mnemonic
                    .iter()
                    .map(|s| s.into())
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
            let identity = identity_name.expect("is present");
            let dcc_identity = DccIdentity::load_from_dir(&identities_dir.join(identity))?;
            let account = dcc_identity.as_icrc_compatible_account();

            if account_args.balance {
                println!(
                    "Account {} balance {}",
                    account,
                    account_balance_get_as_string(&account)
                );
            }

            if let Some(transfer_to_account) = &account_args.transfer_to {
                let transfer_to_account = IcrcCompatibleAccount::from(transfer_to_account);
                let transfer_amount_e9s = match &account_args.amount_dct {
                    Some(value) => value.parse::<TokenAmount>()? * DC_TOKEN_DECIMALS_DIV,
                    None => match &account_args.amount_e9s {
                        Some(value) => value.parse::<TokenAmount>()?,
                        None => {
                            panic!("You must specify either --amount-dct or --amount-e9s")
                        }
                    },
                };
                println!(
                    "Transferring {} tokens from {} \t to account {}",
                    amount_as_string(transfer_amount_e9s),
                    account,
                    transfer_to_account,
                );
                let ic_auth = dcc_to_ic_auth(&dcc_identity);
                let canister = ledger_canister(ic_auth).await?;
                let transfer_args = TransferArg {
                    amount: transfer_amount_e9s.into(),
                    fee: Some(DC_TOKEN_TRANSFER_FEE_E9S.into()),
                    from_subaccount: None,
                    to: transfer_to_account.into(),
                    created_at_time: None,
                    memo: None,
                };
                let args = Encode!(&transfer_args).map_err(|e| e.to_string())?;
                let result = canister.call_update("icrc1_transfer", &args).await?;
                let response =
                    Decode!(&result, Result<Nat, Icrc1TransferError>).map_err(|e| e.to_string())?;

                match response {
                    Ok(response) => {
                        println!(
                            "Transfer request successful, will be included in block: {}",
                            response
                        );
                    }
                    Err(e) => {
                        println!("Transfer error: {}", e);
                    }
                }
            }

            Ok(())
        }
        Commands::Np(ref np_args) => {
            if np_args.list || np_args.balances {
                list_identities(np_args.balances)?;
            } else if np_args.register {
                let identity = identity_name.expect("You must specify an identity");
                let dcc_ident = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;
                let ic_auth = dcc_to_ic_auth(&dcc_ident);

                info!("Registering principal: {} as {}", identity, dcc_ident);
                let pubkey_bytes = dcc_ident.to_bytes_verifying();
                let pubkey_signature = dcc_ident.sign(pubkey_bytes.as_ref())?;
                let result = ledger_canister(ic_auth)
                    .await?
                    .node_provider_register(&pubkey_bytes, pubkey_signature.to_bytes().as_slice())
                    .await?;
                println!("Register: {}", result);
            } else if np_args.check_in_nonce {
                let nonce_bytes = ledger_canister(None).await?.get_check_in_nonce().await;
                let nonce_string = hex::encode(&nonce_bytes);

                println!("0x{}", nonce_string);
            } else if np_args.check_in {
                let identity = identity_name.expect("You must specify an identity");

                let dcc_ident = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;
                let ic_auth = dcc_to_ic_auth(&dcc_ident);

                // Check the local ledger timestamp
                let local_ledger_path = ledger_local
                    .get_file_path()
                    .expect("Failed to get local ledger path");
                let local_ledger_file_mtime = local_ledger_path.metadata()?.modified()?;

                // If the local ledger is older than 1 minute, refresh it automatically before proceeding
                // If needed, the local ledger can also be refreshed manually from the command line
                if local_ledger_file_mtime < SystemTime::now() - std::time::Duration::from_secs(60)
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
                let check_in_memo = np_args.check_in_memo.clone().unwrap_or_else(|| {
                    println!("No memo specified, did you know that you can specify one? Try out --check-in-memo");
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
            } else if let Some(ref profile_file_name) = np_args.update_profile {
                let identity = identity_name.expect("You must specify an identity");

                let dcc_id = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;
                let ic_auth = dcc_to_ic_auth(&dcc_id);

                let np_profile = np_profile::Profile::new_from_file(profile_file_name)?;
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
            } else if let Some(ref offering_file_name) = np_args.update_offering {
                let identity = identity_name.expect("You must specify an identity");
                let dcc_id = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;
                let ic_auth = dcc_to_ic_auth(&dcc_id);

                // Offering::new_from_file returns an error if the schema validation fails
                let np_offering = np_offering::Offering::new_from_file(offering_file_name)?;
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

            Ok(())
        }
        Commands::User(ref user_args) => {
            if user_args.list || user_args.balances {
                list_identities(user_args.balances)?
            } else if user_args.register {
                let identity = identity_name.expect("You must specify an identity");
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
        Commands::LedgerRemote(ref remote_args) => {
            let local_ledger_path = match remote_args.dir {
                Some(ref value) => PathBuf::from(value),
                None => dirs::home_dir()
                    .expect("Could not get home directory")
                    .join(".dcc")
                    .join("ledger")
                    .join("main.bin"),
            };
            let push_auth = remote_args.data_push_authorize;
            let push = remote_args.data_push;
            if push_auth || push {
                let identity = identity_name.expect("You must specify an identity");

                let dcc_ident = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;

                if push_auth {
                    let ic_auth = dcc_to_ic_auth(&dcc_ident);
                    let canister = ledger_canister(ic_auth).await?;
                    let args = Encode!(&()).map_err(|e| e.to_string())?;
                    let result = canister.call_update("data_push_auth", &args).await?;
                    let response =
                        Decode!(&result, Result<String, String>).map_err(|e| e.to_string())??;

                    println!("Push auth: {}", response);
                }

                // After authorizing, we can push the data
                let ic_auth = dcc_to_ic_auth(&dcc_ident);
                let canister = ledger_canister(ic_auth).await?;

                return ledger_data_push(&canister, local_ledger_path).await;
            }

            let canister_function = match remote_args.canister_function {
                Some(ref value) => value,
                None => {
                    println!("Available canister functions:");
                    for f in ledger_canister(None).await?.list_functions_updates() {
                        println!("UPDATE:\t{}", f);
                    }
                    for f in ledger_canister(None).await?.list_functions_queries() {
                        println!("QUERY:\t{}", f);
                    }
                    return Ok(());
                }
            };
            println!("Calling canister function: {}", canister_function);

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

            match canister_function.as_str() {
                "init_ledger_map" => {
                    let canister = ledger_canister(None).await?;
                    println!("{}", canister.init_ledger_map().await?);
                }
                "data_fetch" | "fetch" => {
                    let canister = ledger_canister(None).await?;
                    ledger_data_fetch(&canister, local_ledger_path).await?;
                    println!("Done fetching data from the Ledger canister");
                }
                "metadata" => {
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
                "get_check_in_nonce" => {
                    let nonce_bytes = ledger_canister(None).await?.get_check_in_nonce().await;
                    println!("{}", hex::encode(nonce_bytes));
                }
                "get_logs_debug" => {
                    println!("Ledger canister DEBUG logs:");
                    for entry in serde_json::from_str::<Vec<serde_json::Value>>(
                        &ledger_canister(None).await?.get_logs_debug().await?,
                    )?
                    .into_iter()
                    {
                        log_with_level(entry, Level::Debug);
                    }
                }
                "get_logs_info" => {
                    println!("Ledger canister INFO logs:");
                    for entry in serde_json::from_str::<Vec<serde_json::Value>>(
                        &ledger_canister(None).await?.get_logs_info().await?,
                    )?
                    .into_iter()
                    {
                        log_with_level(entry, Level::Info);
                    }
                }
                "get_logs_warn" => {
                    println!("Ledger canister WARN logs:");
                    for entry in serde_json::from_str::<Vec<serde_json::Value>>(
                        &ledger_canister(None).await?.get_logs_warn().await?,
                    )?
                    .into_iter()
                    {
                        log_with_level(entry, Level::Warn);
                    }
                }
                "get_logs_error" => {
                    println!("Ledger canister ERROR logs:");
                    for entry in serde_json::from_str::<Vec<serde_json::Value>>(
                        &ledger_canister(None).await?.get_logs_error().await?,
                    )?
                    .into_iter()
                    {
                        log_with_level(entry, Level::Error);
                    }
                }
                _ => panic!("Unknown canister function: {}", canister_function),
            };

            Ok(())
        }
        Commands::Offering(ref offering_args) => {
            let query = offering_args.query.clone().unwrap_or_default();
            if offering_args.list || !query.is_empty() {
                let offerings = do_get_matching_offerings(&ledger_local, &query);
                println!("Found {} matching offerings:", offerings.len());
                for (dcc_id, offering) in offerings {
                    println!(
                        "{} ==>\n{}",
                        dcc_id,
                        &offering.as_json_string_pretty().unwrap_or_default()
                    );
                }
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
                let contracts_open = canister.contracts_list_pending(pubkey_bytes).await;
                if contracts_open.is_empty() {
                    println!("No open contracts");
                } else {
                    for (contract_id, contract_req_bytes) in contracts_open {
                        println!("{}", hex::encode(contract_id));
                        let contract_req =
                            ContractSignRequestPayload::try_from_slice(&contract_req_bytes)
                                .unwrap();
                        let contract_req =
                            contract_req.deserialize_contract_sign_request().unwrap();
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&contract_req).unwrap_or_default()
                        );
                    }
                }
                Ok(())
            }
            ContractCommands::SignRequest(sign_req_args) => {
                println!("Request to sign a contract...");
                loop {
                    let identity;
                    let offering_id;
                    let requester_ssh_pubkey;
                    let requester_contact;
                    let provider_pubkey_pem;
                    let memo;

                    if sign_req_args.interactive {
                        println!();
                        identity = match &cli.identity {
                            Some(name) => name.clone(),
                            None => dialoguer::Input::<String>::new()
                                .with_prompt("Please enter the identity name")
                                .allow_empty(false)
                                .show_default(false)
                                .interact()
                                .unwrap(),
                        };
                        offering_id = match &sign_req_args.offering_id {
                            Some(s) => s.clone(),
                            None => dialoguer::Input::<String>::new()
                                .with_prompt("Please enter the offering id")
                                .allow_empty(false)
                                .show_default(false)
                                .interact()
                                .unwrap_or_default(),
                        };
                        requester_ssh_pubkey = match &sign_req_args.requester_ssh_pubkey {
                            Some(s) => s.clone(),
                            None => dialoguer::Input::<String>::new()
                                .with_prompt("Please enter your ssh public key, which will be granted access to the contract")
                                .allow_empty(false)
                                .show_default(false)
                                .interact()
                                .unwrap_or_default(),
                        };
                        requester_contact = match &sign_req_args.requester_contact {
                            Some(s) => s.clone(),
                            None => dialoguer::Input::<String>::new()
                                .with_prompt("Enter your contact information (this will be public)")
                                .allow_empty(false)
                                .show_default(false)
                                .interact()
                                .unwrap_or_default(),
                        };
                        provider_pubkey_pem = match &sign_req_args.provider_pubkey_pem {
                            Some(s) => s.clone(),
                            None => match dialoguer::Editor::new()
                                .edit("# Enter the provider's public key below, as a PEM string")
                            {
                                Ok(Some(content)) => content,
                                Ok(None) => {
                                    println!("No input received.");
                                    continue;
                                }
                                Err(err) => {
                                    eprintln!("Error opening editor: {}", err);
                                    continue;
                                }
                            },
                        };
                        memo = match &sign_req_args.memo {
                            Some(s) => s.clone(),
                            None => dialoguer::Input::<String>::new()
                                .with_prompt(
                                    "Please enter a memo for the contract (this will be public)",
                                )
                                .allow_empty(true)
                                .show_default(false)
                                .interact()
                                .unwrap_or_default(),
                        };
                    } else {
                        identity = cli.identity.clone().expect("You must specify an identity");
                        offering_id = sign_req_args
                            .offering_id
                            .clone()
                            .expect("You must specify an offering id");
                        requester_ssh_pubkey = sign_req_args
                            .requester_ssh_pubkey
                            .clone()
                            .expect("You must specify your ssh pubkey");
                        requester_contact = sign_req_args
                            .requester_contact
                            .clone()
                            .expect("You must specify your contact info");
                        provider_pubkey_pem = sign_req_args
                            .provider_pubkey_pem
                            .clone()
                            .expect("You must specify the provider's pubkey");
                        memo = sign_req_args.memo.clone().unwrap_or_default();
                    };
                    let dcc_id = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;
                    let provider_dcc_ident =
                        match DccIdentity::new_verifying_from_pem(&provider_pubkey_pem) {
                            Ok(ident) => ident,
                            Err(e) => {
                                eprintln!("ERROR: Failed to parse provider pubkey: {}", e);
                                continue;
                            }
                        };
                    let provider_pubkey_bytes = provider_dcc_ident.to_bytes_verifying();

                    let requester_pubkey_bytes = dcc_id.to_bytes_verifying();
                    let req = ContractSignRequest::new(
                        &requester_pubkey_bytes,
                        requester_ssh_pubkey,
                        requester_contact,
                        &provider_pubkey_bytes,
                        offering_id.clone(),
                        None,
                        None,
                        None,
                        100,
                        3600,
                        None,
                        memo.clone(),
                    );
                    println!("The following contract sign request will be sent:");
                    println!("{}", serde_json::to_string_pretty(&req)?);
                    if dialoguer::Confirm::new()
                        .with_prompt("Is this correct? If so, press enter to send.")
                        .default(false)
                        .show_default(true)
                        .interact()
                        .unwrap()
                    {
                        let payload_bytes = borsh::to_vec(&req).unwrap();
                        let payload_sig_bytes = dcc_id.sign(&payload_bytes)?.to_bytes();
                        let ic_auth = dcc_to_ic_auth(&dcc_id);
                        let canister = ledger_canister(ic_auth).await?;

                        let response = canister
                            .contract_sign_request(
                                &requester_pubkey_bytes,
                                &payload_bytes,
                                &payload_sig_bytes,
                            )
                            .await;

                        match response {
                            Ok(response) => {
                                println!("Contract sign request successful: {}", response);
                            }
                            Err(e) => {
                                println!("Contract sign request failed: {}", e);
                            }
                        }
                    }
                }
            }
            ContractCommands::SignReply(_sign_reply_args) => {
                println!("Reply to a contract-sign request...");
                unimplemented!();
            }
        },
    }?;
    Ok(())
}

fn list_identities(include_balances: bool) -> Result<(), Box<dyn std::error::Error>> {
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
                    if include_balances {
                        println!(
                            "{} => {}, reputation {}, balance {}",
                            identity_name,
                            dcc_identity,
                            reputation_get(dcc_identity.to_bytes_verifying()),
                            account_balance_get_as_string(
                                &dcc_identity.as_icrc_compatible_account()
                            )
                        );
                    } else {
                        println!(
                            "{} => {} reputation {}",
                            identity_name,
                            dcc_identity,
                            reputation_get(dcc_identity.to_bytes_verifying())
                        );
                    }
                }
                Err(e) => {
                    println!("{} => Error: {}", identity_name, e);
                }
            }
        }
    }
    Ok(())
}

fn dcc_to_ic_auth(dcc_identity: &DccIdentity) -> Option<BasicIdentity> {
    dcc_identity
        .signing_key_as_ic_agent_pem_string()
        .map(|pem_key| {
            let cursor = std::io::Cursor::new(pem_key.as_bytes());
            BasicIdentity::from_pem(cursor).expect("failed to parse pem key")
        })
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

    println!(
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
    println!(
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
        println!("Data: {} bytes ==> {:?}", data.len(), data);
    } else {
        println!(
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
        println!(
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
        println!("Nothing to push");
        return Ok(());
    }

    println!(
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
        println!(
            "Pushing block of {} bytes at position {}",
            buf_size, position,
        );
        let args = Encode!(&cursor_push.to_urlenc_string(), &buf).map_err(|e| e.to_string())?;
        let result = ledger_canister.call_update("data_push", &args).await?;
        let result = Decode!(&result, Result<String, String>).map_err(|e| e.to_string())??;
        println!("Response from pushing at position {}: {}", position, result);
    }

    Ok(())
}

struct SimpleStderrLogger;

impl log::Log for SimpleStderrLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            eprintln!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

static LOGGER: SimpleStderrLogger = SimpleStderrLogger;

pub fn init_logger() -> anyhow::Result<()> {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Info))
        .map_err(|e| anyhow::anyhow!(e))
}
