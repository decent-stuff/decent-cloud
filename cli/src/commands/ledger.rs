use crate::argparse::{LedgerLocalArgs, LedgerRemoteCommands};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use candid::{Decode, Encode};
use chrono::DateTime;
use dcc_common::{DccIdentity, FundsTransfer, LABEL_DC_TOKEN_TRANSFER};
use decent_cloud::ledger_canister_client::LedgerCanister;
use icrc_ledger_types::icrc::generic_metadata_value::MetadataValue;
use ledger_map::LedgerMap;
use log::Level;
use std::path::PathBuf;
use tabular::{Row, Table};

pub async fn handle_ledger_local_command(
    local_args: LedgerLocalArgs,
    ledger_local: LedgerMap,
) -> Result<(), Box<dyn std::error::Error>> {
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

pub async fn handle_ledger_remote_command(
    subcmd: LedgerRemoteCommands,
    network_url: &str,
    ledger_canister_id: candid::Principal,
    identity: Option<String>,
    ledger_local: LedgerMap,
) -> Result<(), Box<dyn std::error::Error>> {
    let local_ledger_path = ledger_local
        .get_file_path()
        .expect("Failed to get local ledger path");

    match subcmd {
        LedgerRemoteCommands::DataFetch => {
            let canister =
                LedgerCanister::new_without_identity(network_url, ledger_canister_id).await?;
            return crate::ledger::ledger_data_fetch(&canister, &ledger_local).await;
        }
        LedgerRemoteCommands::DataPushAuthorize | LedgerRemoteCommands::DataPush => {
            let identity =
                identity.expect("Identity must be specified for this command, use --identity");

            let dcc_id = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;

            let push_auth = subcmd == LedgerRemoteCommands::DataPushAuthorize;

            if push_auth {
                let canister =
                    LedgerCanister::new_with_dcc_id(network_url, ledger_canister_id, &dcc_id)
                        .await?;
                let args = Encode!(&()).map_err(|e| e.to_string())?;
                let result = canister.call_update("data_push_auth", &args).await?;
                let response =
                    Decode!(&result, Result<String, String>).map_err(|e| e.to_string())??;

                println!("Push auth: {}", response);
            }

            // After authorizing, we can push the data
            let canister =
                LedgerCanister::new_with_dcc_id(network_url, ledger_canister_id, &dcc_id).await?;

            return crate::ledger::ledger_data_push(&canister, local_ledger_path).await;
        }
        LedgerRemoteCommands::Metadata => {
            let canister =
                LedgerCanister::new_without_identity(network_url, ledger_canister_id).await?;

            #[allow(clippy::literal_string_with_formatting_args)]
            let mut table = Table::new("{:<}  {:<}")
                .with_row(Row::from_cells(["Key", "Value"].iter().cloned()));

            for md_entry in crate::ledger::get_ledger_metadata(&canister).await {
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
            let canister =
                LedgerCanister::new_without_identity(network_url, ledger_canister_id).await?;
            let noargs = Encode!(&()).expect("Failed to encode args");
            let response = canister.call_query("get_registration_fee", &noargs).await?;
            let fee_e9s = Decode!(response.as_slice(), u64).map_err(|e| e.to_string())?;
            println!(
                "Registration fee: {}",
                dcc_common::amount_as_string(fee_e9s)
            );
        }
        LedgerRemoteCommands::GetCheckInNonce => {
            let nonce_bytes = LedgerCanister::new_without_identity(network_url, ledger_canister_id)
                .await?
                .get_check_in_nonce()
                .await;
            println!("{}", hex::encode(nonce_bytes));
        }
        LedgerRemoteCommands::GetLogsDebug => {
            println!("Ledger canister DEBUG logs:");
            print_logs(
                Level::Debug,
                &LedgerCanister::new_without_identity(network_url, ledger_canister_id)
                    .await?
                    .get_logs_debug()
                    .await?,
            )?;
        }
        LedgerRemoteCommands::GetLogsInfo => {
            println!("Ledger canister INFO logs:");
            print_logs(
                Level::Info,
                &LedgerCanister::new_without_identity(network_url, ledger_canister_id)
                    .await?
                    .get_logs_info()
                    .await?,
            )?;
        }
        LedgerRemoteCommands::GetLogsWarn => {
            println!("Ledger canister WARN logs:");
            print_logs(
                Level::Warn,
                &LedgerCanister::new_without_identity(network_url, ledger_canister_id)
                    .await?
                    .get_logs_warn()
                    .await?,
            )?;
        }
        LedgerRemoteCommands::GetLogsError => {
            println!("Ledger canister ERROR logs:");
            print_logs(
                Level::Error,
                &LedgerCanister::new_without_identity(network_url, ledger_canister_id)
                    .await?
                    .get_logs_error()
                    .await?,
            )?;
        }
    }

    Ok(())
}

fn print_logs(log_level: Level, logs_json: &str) -> Result<(), Box<dyn std::error::Error>> {
    for entry in serde_json::from_str::<Vec<serde_json::Value>>(logs_json)?.into_iter() {
        let timestamp_ns = entry["timestamp"].as_u64().unwrap_or_default();
        let timestamp_s = (timestamp_ns / 1_000_000_000) as i64;
        // Create DateTime from the timestamp
        let dt = DateTime::from_timestamp(timestamp_s, 0).unwrap_or_default();
        println!(
            "{} [{}] - {}",
            dt.format("%Y-%m-%dT%H:%M:%S"),
            log_level,
            entry["message"].as_str().expect("Invalid message field")
        );
    }
    Ok(())
}
