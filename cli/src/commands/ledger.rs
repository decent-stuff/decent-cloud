use crate::argparse::{LedgerLocalArgs, LedgerRemoteCommands};
use crate::identity::{list_identities, ListIdentityType};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use candid::{Decode, Encode};
use chrono::{TimeZone, Utc};
use dcc_common::{DccIdentity, FundsTransfer, LABEL_DC_TOKEN_TRANSFER};
use decent_cloud::ledger_canister_client::LedgerCanister;
use icrc_ledger_types::icrc::generic_metadata_value::MetadataValue;
use ledger_map::LedgerMap;
use log::Level;
use serde::Deserialize;
use std::{convert::TryFrom, io, path::PathBuf};
use tabular::{Row, Table};

pub async fn handle_ledger_local_command(
    local_args: LedgerLocalArgs,
    ledger_local: LedgerMap,
) -> Result<(), Box<dyn std::error::Error>> {
    if local_args.list_accounts {
        return list_identities(&ledger_local, ListIdentityType::All, true);
    } else if local_args.list_entries {
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
    mut ledger_local: LedgerMap,
) -> Result<(), Box<dyn std::error::Error>> {
    let local_ledger_path = ledger_local
        .get_file_path()
        .expect("Failed to get local ledger path");

    match subcmd {
        LedgerRemoteCommands::DataFetch => {
            let canister =
                LedgerCanister::new_without_identity(network_url, ledger_canister_id).await?;
            return crate::ledger::ledger_data_fetch(&canister, &mut ledger_local).await;
        }
        #[allow(clippy::double_parens)]
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
            #[allow(clippy::double_parens)]
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
            fetch_and_print_logs(Level::Debug, network_url, ledger_canister_id).await?;
        }
        LedgerRemoteCommands::GetLogsInfo => {
            println!("Ledger canister INFO logs:");
            fetch_and_print_logs(Level::Info, network_url, ledger_canister_id).await?;
        }
        LedgerRemoteCommands::GetLogsWarn => {
            println!("Ledger canister WARN logs:");
            fetch_and_print_logs(Level::Warn, network_url, ledger_canister_id).await?;
        }
        LedgerRemoteCommands::GetLogsError => {
            println!("Ledger canister ERROR logs:");
            fetch_and_print_logs(Level::Error, network_url, ledger_canister_id).await?;
        }
    }

    Ok(())
}

fn print_logs(log_level: Level, logs_json: &str) -> Result<(), serde_json::Error> {
    for line in format_log_lines(log_level, logs_json)? {
        println!("{}", line);
    }

    Ok(())
}

fn format_log_lines(log_level: Level, logs_json: &str) -> Result<Vec<String>, serde_json::Error> {
    let entries: Vec<LogEntry> = serde_json::from_str(logs_json)?;

    Ok(entries
        .into_iter()
        .map(|entry| format_log_line(log_level, entry))
        .collect())
}

fn format_log_line(log_level: Level, entry: LogEntry) -> String {
    let timestamp = entry
        .timestamp
        .and_then(format_timestamp)
        .unwrap_or_else(|| "unknown".to_string());
    let message = entry.message.unwrap_or_else(|| "<no message>".to_string());

    format!("{} [{}] - {}", timestamp, log_level, message)
}

fn format_timestamp(timestamp_ns: u64) -> Option<String> {
    let seconds = timestamp_ns / 1_000_000_000;
    let nanoseconds = (timestamp_ns % 1_000_000_000) as u32;
    let seconds = i64::try_from(seconds).ok()?;

    Utc.timestamp_opt(seconds, nanoseconds)
        .single()
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string())
}

#[derive(Debug, Deserialize)]
struct LogEntry {
    timestamp: Option<u64>,
    message: Option<String>,
}

async fn fetch_and_print_logs(
    level: Level,
    network_url: &str,
    ledger_canister_id: candid::Principal,
) -> Result<(), Box<dyn std::error::Error>> {
    let canister = LedgerCanister::new_without_identity(network_url, ledger_canister_id).await?;
    let logs = canister
        .get_logs(level)
        .await
        .map_err(|err| -> Box<dyn std::error::Error> { Box::new(io::Error::other(err)) })?;
    print_logs(level, &logs).map_err(|err| -> Box<dyn std::error::Error> { Box::new(err) })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_timestamp_and_message() {
        let logs_json = "[{\"timestamp\": 1680000000000000000, \"message\": \"hello\"}]";
        let lines = format_log_lines(Level::Info, logs_json).expect("log parsing failed");

        assert_eq!(
            lines,
            vec!["2023-03-28T10:40:00 [INFO] - hello".to_string()]
        );
    }

    #[test]
    fn handles_missing_fields() {
        let logs_json = "[{\"timestamp\": null}, {\"message\": null}]";
        let lines = format_log_lines(Level::Warn, logs_json).expect("log parsing failed");

        assert_eq!(
            lines,
            vec![
                "unknown [WARN] - <no message>".to_string(),
                "unknown [WARN] - <no message>".to_string()
            ]
        );
    }
}
