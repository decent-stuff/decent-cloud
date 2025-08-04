use crate::argparse::ContractCommands;
use crate::contracts::prompt_for_payment_entries;
use crate::utils::prompts::{prompt_bool, prompt_editor, prompt_input};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use dcc_common::{ContractSignReply, ContractSignRequest, DccIdentity};
use decent_cloud::ledger_canister_client::LedgerCanister;
use ledger_map::LedgerMap;
use provider_offering::ProviderOfferings;
use std::path::PathBuf;

pub async fn handle_contract_command(
    contract_args: ContractCommands,
    network_url: &str,
    ledger_canister_id: candid::Principal,
    identity: Option<String>,
    ledger_local: LedgerMap,
) -> Result<(), Box<dyn std::error::Error>> {
    match contract_args {
        ContractCommands::ListOpen(_list_open_args) => {
            println!("Listing all open contracts...");
            // A user may provide the identity (public key), but doesn't have to
            let contracts_open = match identity {
                Some(name) => {
                    let dcc_id = DccIdentity::load_from_dir(&PathBuf::from(&name)).unwrap();
                    let canister =
                        LedgerCanister::new_with_dcc_id(network_url, ledger_canister_id, &dcc_id)
                            .await?;
                    canister
                        .contracts_list_pending(&Some(dcc_id.to_bytes_verifying()))
                        .await
                }
                None => {
                    LedgerCanister::new_without_identity(network_url, ledger_canister_id)
                        .await?
                        .contracts_list_pending(&None)
                        .await
                }
            };
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
        }
        ContractCommands::SignRequest(sign_req_args) => {
            println!("Request to sign a contract...");
            loop {
                println!();
                let i = sign_req_args.interactive;
                let identity = prompt_input("Please enter the identity name", &identity, i, false);
                let instance_id = prompt_input(
                    "Please enter the offering id",
                    &sign_req_args.offering_id,
                    i,
                    false,
                );
                let requester_ssh_pubkey = prompt_input(
                    "Please enter your ssh public key, which will be granted access to the contract",
                    &sign_req_args.requester_ssh_pubkey,
                    i,
                    false,
                );
                let requester_contact = prompt_input(
                    "Enter your contact information (this will be public)",
                    &sign_req_args.requester_contact,
                    i,
                    true,
                );
                let provider_pubkey_pem =
                    sign_req_args
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
                let offerings = dcc_common::offerings::do_get_matching_offerings(
                    &ledger_local,
                    &format!("instance_types.id = \"{instance_id}\""),
                )
                .into_iter()
                .filter(|o| o.provider_pubkey == provider_pubkey_bytes)
                .collect::<Vec<ProviderOfferings>>();

                let offering = match offerings.len() {
                    0 => {
                        eprintln!(
                            "ERROR: No offering found for the provider {provider_dcc_id} and id: {instance_id}"
                        );
                        continue;
                    }
                    1 => {
                        // Find the specific server offering with the matching instance_id
                        let provider_offering = &offerings[0];
                        let matching_offerings: Vec<&provider_offering::ServerOffering> =
                            provider_offering
                                .server_offerings
                                .iter()
                                .filter(|so| so.unique_internal_identifier == instance_id)
                                .collect();

                        match matching_offerings.len() {
                            0 => {
                                eprintln!(
                                    "ERROR: No offering found for the provider {provider_dcc_id} and id: {instance_id}"
                                );
                                continue;
                            }
                            1 => matching_offerings[0],
                            _ => {
                                eprintln!("ERROR: Provider {provider_dcc_id} has multiple offerings with id: {instance_id}");
                                continue;
                            }
                        }
                    }
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
                    let canister =
                        LedgerCanister::new_with_dcc_id(network_url, ledger_canister_id, &dcc_id)
                            .await?;

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
        }
        ContractCommands::SignReply(sign_reply_args) => {
            println!("Reply to a contract-sign request...");
            loop {
                let i = sign_reply_args.interactive;
                let identity = prompt_input("Please enter the identity name", &identity, i, false);
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
                let canister =
                    LedgerCanister::new_with_dcc_id(network_url, ledger_canister_id, &dcc_id)
                        .await?;

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
        }
    }
    Ok(())
}
