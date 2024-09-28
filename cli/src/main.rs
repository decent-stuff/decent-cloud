mod argparse;
mod keygen;

// use borsh::{BorshDeserialize, BorshSerialize};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use bip39::Seed;
use candid::{Decode, Encode, Nat, Principal as IcPrincipal};
use chrono::DateTime;
use dcc_common::{
    account_balance_get_as_string, amount_as_string, cursor_from_data, refresh_caches_from_ledger,
    reputation_get, Balance, CursorDirection, DccIdentity, FundsTransfer, IcrcCompatibleAccount,
    LedgerCursor, NodeProviderProfile, UpdateProfilePayload, DATA_PULL_BYTES_BEFORE_LEN,
    DC_TOKEN_DECIMALS_DIV, LABEL_DC_TOKEN_TRANSFER,
};
use decent_cloud::ledger_canister_client::LedgerCanister;
use decent_cloud_canister::DC_TOKEN_TRANSFER_FEE_E9S;
use fs_err::{File, OpenOptions};
use ic_agent::identity::BasicIdentity;
use icrc_ledger_types::{
    icrc::generic_metadata_value::MetadataValue, icrc1::transfer::TransferArg,
    icrc1::transfer::TransferError as Icrc1TransferError,
};
use ledger_map::{platform_specific::persistent_storage_read, LedgerMap};
use log::{info, Level, LevelFilter, Metadata, Record};
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

    let args = argparse::parse_args();

    let ledger_path = match args.get_one::<String>("local-ledger-dir") {
        Some(value) => PathBuf::from(value),
        None => dirs::home_dir()
            .expect("Could not get home directory")
            .join(".dcc/ledger/main.bin"),
    };
    let ledger_local =
        LedgerMap::new_with_path(None, Some(ledger_path)).expect("Failed to load the local ledger");
    refresh_caches_from_ledger(&ledger_local).expect("Failed to get balances");

    let network = args
        .get_one::<String>("network")
        .expect("missing required argument '--network'");
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

    Ok(match args.subcommand() {
        Some(("keygen", arg_matches)) => {
            let identity = arg_matches
                .get_one::<String>("identity")
                .expect("is present");

            let mnemonic = if arg_matches.contains_id("mnemonic") {
                let mnemonic_string = arg_matches
                    .get_many::<String>("mnemonic")
                    .expect("contains mnemonic")
                    .map(|s| s.into())
                    .collect::<Vec<_>>();
                if mnemonic_string.len() < 12 {
                    let reader = BufReader::new(io::stdin());
                    keygen::mnemonic_from_stdin(reader, io::stdout())?
                } else {
                    keygen::mnemonic_from_strings(mnemonic_string)?
                }
            } else if arg_matches.get_flag("generate") {
                let mnemonic =
                    bip39::Mnemonic::new(bip39::MnemonicType::Words12, bip39::Language::English);
                info!("Mnemonic:\n{}", mnemonic);
                mnemonic
            } else {
                panic!("Neither mnemonic nor generate specified");
            };

            let seed = Seed::new(&mnemonic, "");
            let dcc_identity = DccIdentity::new_from_seed(seed.as_bytes())?;
            info!("Generated identity: {}", dcc_identity);
            dcc_identity.save_to_dir(identity)
        }
        Some(("account", arg_matches)) => {
            let identities_dir = DccIdentity::identities_dir();
            let identity = arg_matches
                .get_one::<String>("identity")
                .expect("is present");
            let dcc_identity = DccIdentity::load_from_dir(&identities_dir.join(identity))?;
            let account = dcc_identity.as_icrc_compatible_account();

            if arg_matches.get_flag("balance") {
                println!(
                    "Account {} balance {}",
                    account,
                    account_balance_get_as_string(&account)
                );
            }

            if let Some(transfer_to_account) = arg_matches.get_one::<String>("transfer-to") {
                let transfer_to_account = IcrcCompatibleAccount::from(transfer_to_account);
                let transfer_amount_e9s = match arg_matches.get_one::<String>("amount-dct") {
                    Some(value) => value.parse::<Balance>()? * DC_TOKEN_DECIMALS_DIV,
                    None => match arg_matches.get_one::<String>("amount-e9s") {
                        Some(value) => value.parse::<Balance>()?,
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
        Some(("np", arg_matches)) => {
            if arg_matches.get_flag("list") || arg_matches.get_flag("balances") {
                list_identities(arg_matches.get_flag("balances"))?;
            } else if arg_matches.contains_id("register") {
                if let Some(np_desc) = arg_matches.get_one::<String>("register") {
                    let dcc_ident = DccIdentity::load_from_dir(&PathBuf::from(np_desc))?;
                    let ic_auth = dcc_to_ic_auth(&dcc_ident);

                    info!("Registering principal: {} as {}", np_desc, dcc_ident);
                    let result = ledger_canister(ic_auth)
                        .await?
                        .node_provider_register(
                            &dcc_ident.to_bytes_verifying(),
                            dcc_ident.verifying_key().as_ref(),
                        )
                        .await?;
                    println!("Register: {}", result);
                } else {
                    panic!("You must specify an identity to register");
                }
            } else if arg_matches.contains_id("check-in") {
                if let Some(np_desc) = arg_matches.get_one::<String>("check-in") {
                    let dcc_ident = DccIdentity::load_from_dir(&PathBuf::from(np_desc))?;
                    let ic_auth = dcc_to_ic_auth(&dcc_ident);

                    let nonce_bytes = ledger_canister(ic_auth)
                        .await?
                        .get_np_check_in_nonce()
                        .await;
                    let nonce_string = hex::encode(&nonce_bytes);

                    info!(
                        "Checking-in NP identity {} ({}), using nonce: {} ({} bytes)",
                        np_desc,
                        dcc_ident,
                        nonce_string,
                        nonce_bytes.len()
                    );
                    let ic_auth = dcc_to_ic_auth(&dcc_ident);
                    let result = ledger_canister(ic_auth)
                        .await?
                        .node_provider_check_in(
                            &dcc_ident.to_bytes_verifying(),
                            &dcc_ident.sign(&nonce_bytes)?.to_bytes(),
                        )
                        .await
                        .map_err(|e| format!("Check-in failed: {}", e))?;
                    info!("Check-in success: {}", result);
                } else {
                    panic!("You must specify an identity to register");
                }
            } else if arg_matches.contains_id("update-profile") {
                if let Some(values) = arg_matches.get_many::<String>("update-profile") {
                    let values = values.collect::<Vec<_>>();
                    let np_desc = values[0];
                    let profile_file = values[1];

                    let dcc_ident = DccIdentity::load_from_dir(&PathBuf::from(np_desc))?;
                    let ic_auth = dcc_to_ic_auth(&dcc_ident);
                    let np_profile: NodeProviderProfile =
                        serde_yaml_ng::from_reader(File::open(profile_file)?)?;

                    // Serialize the profile and sign it
                    let profile_payload = serde_json::to_vec(&np_profile)?;
                    let signature = dcc_ident.sign(&profile_payload)?.to_vec();

                    // Send the payload
                    let payload = UpdateProfilePayload {
                        profile_payload,
                        signature,
                    };
                    let result = ledger_canister(ic_auth)
                        .await?
                        .node_provider_update_profile(
                            &dcc_ident.to_bytes_verifying(),
                            &serde_json::to_vec(&payload)?,
                        )
                        .await
                        .map_err(|e| format!("Update profile failed: {}", e))?;
                    info!("Profile update response: {}", result);
                } else {
                    panic!("You must specify an identity of the node provider");
                }
            }
            Ok(())
        }
        Some(("user", arg_matches)) => {
            if arg_matches.get_flag("list") || arg_matches.get_flag("balances") {
                list_identities(arg_matches.get_flag("balances"))?
            } else if arg_matches.contains_id("register") {
                match arg_matches.get_one::<String>("register") {
                    Some(np_desc) => {
                        let dcc_ident = DccIdentity::load_from_dir(&PathBuf::from(np_desc))?;
                        let ic_auth = dcc_to_ic_auth(&dcc_ident);
                        let canister = ledger_canister(ic_auth).await?;
                        let args = Encode!(
                            &dcc_ident.to_bytes_verifying(),
                            &dcc_ident.verifying_key().as_ref()
                        )?;
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
                    None => panic!("You must specify an identity to register"),
                }
            }
            Ok(())
        }
        Some(("ledger_local", arg_matches)) => {
            if arg_matches.get_flag("list_entries") {
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
            } else if arg_matches.get_flag("list_entries_raw") {
                println!("Raw Entries:");
                for entry in ledger_local.iter_raw() {
                    let (blk_header, ledger_block) = entry?;
                    println!("{}", blk_header);
                    println!("{}", ledger_block)
                }
            }
            Ok(())
        }
        Some(("ledger_remote", arg_matches)) => {
            let local_identity = arg_matches.get_one::<String>("identity");
            let local_ledger_path = match arg_matches.get_one::<String>("dir") {
                Some(value) => PathBuf::from(value),
                None => dirs::home_dir()
                    .expect("Could not get home directory")
                    .join(".dcc/ledger/main.bin"),
            };
            let push_auth = arg_matches.get_flag("data-push-authorize");
            let push = arg_matches.get_flag("data-push");
            if push_auth || push {
                let local_identity = match local_identity {
                    Some(ident) => ident.to_string(),
                    None => panic!("You must specify an identity to authorize"),
                };

                let dcc_ident = DccIdentity::load_from_dir(&PathBuf::from(local_identity))?;

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

            let canister_function = match arg_matches.get_one::<String>("canister_function") {
                Some(value) => value,
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
                "get_np_check_in_nonce" => {
                    let nonce_bytes = ledger_canister(None).await?.get_np_check_in_nonce().await;
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
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }?)
}

fn list_identities(include_balances: bool) -> Result<(), Box<dyn std::error::Error>> {
    let identities_dir = DccIdentity::identities_dir();
    println!("Available identities at {}:", identities_dir.display());
    for identity in std::fs::read_dir(identities_dir)? {
        match identity {
            Ok(identity) => {
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
            Err(e) => {
                eprintln!("Error: {}", e);
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
        "Fetching data from the Ledger canister, with local cursor: {} and bytes before: {:?}",
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
