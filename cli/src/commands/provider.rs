use crate::argparse::ProviderCommands;
use crate::identity::{list_identities, list_local_identities, ListIdentityType};
use dcc_common::DccIdentity;
use decent_cloud::ledger_canister_client::LedgerCanister;
use ledger_map::LedgerMap;
use log::info;
use std::{collections::HashMap, path::PathBuf, time::SystemTime};

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

                    dcc_common::refresh_ledger_and_caches(&mut ledger_local).map_err(|e| {
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
        ProviderCommands::PoolSuggestOfferings(args) => {
            let identity = identity.ok_or_else(|| {
                "Identity must be specified for this command. Use --identity <name>".to_string()
            })?;
            let dcc_id = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;

            pool_suggest_offerings(&dcc_id, &args.pool_id, &args.api_url).await?;
        }
        ProviderCommands::PoolGenerateOfferings(args) => {
            let identity = identity.ok_or_else(|| {
                "Identity must be specified for this command. Use --identity <name>".to_string()
            })?;
            let dcc_id = DccIdentity::load_from_dir(&PathBuf::from(&identity))?;

            pool_generate_offerings(
                &dcc_id,
                &args.pool_id,
                args.tiers.as_deref(),
                &args.pricing_file,
                &args.visibility,
                args.dry_run,
                &args.api_url,
            )
            .await?;
        }
    }
    Ok(())
}

/// Sign an API request using the shared dcc_common signer. Returns
/// `(timestamp_nanos, nonce, signature_hex)` — the values for the `X-Timestamp`,
/// `X-Nonce`, and `X-Signature` headers. The caller already holds `pubkey_hex`
/// for the `X-Public-Key` header.
fn sign_api_request(
    dcc_id: &DccIdentity,
    method: &str,
    path: &str,
    body: &[u8],
) -> Result<(String, String, String), Box<dyn std::error::Error>> {
    let signed = dcc_common::api_auth::sign_request(dcc_id, method, path, body)
        .map_err(|e| format!("Failed to sign request: {e}"))?;
    Ok((signed.timestamp, signed.nonce, signed.signature_hex))
}

/// Get offering suggestions for a pool
async fn pool_suggest_offerings(
    dcc_id: &DccIdentity,
    pool_id: &str,
    api_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let pubkey_hex = hex::encode(dcc_id.to_bytes_verifying());
    let path = format!(
        "/api/v1/providers/{}/pools/{}/offering-suggestions",
        pubkey_hex, pool_id
    );

    let (timestamp, nonce, signature) = sign_api_request(dcc_id, "GET", &path, &[])?;

    let client = crate::utils::http_client();
    let url = format!("{}{}", api_url, path);

    let response = client
        .get(&url)
        .header(dcc_common::api_auth::HEADER_PUBLIC_KEY, &pubkey_hex)
        .header(dcc_common::api_auth::HEADER_TIMESTAMP, &timestamp)
        .header(dcc_common::api_auth::HEADER_NONCE, &nonce)
        .header(dcc_common::api_auth::HEADER_SIGNATURE, &signature)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("API request failed ({}): {}", status, body).into());
    }

    let json: serde_json::Value = response.json().await?;

    if json.get("success").and_then(|v| v.as_bool()) != Some(true) {
        let error = json
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        return Err(format!("API error: {}", error).into());
    }

    // Pretty print the suggestions
    println!("{}", serde_json::to_string_pretty(&json.get("data"))?);

    Ok(())
}

/// Generate offerings for a pool
async fn pool_generate_offerings(
    dcc_id: &DccIdentity,
    pool_id: &str,
    tiers: Option<&str>,
    pricing_file: &str,
    visibility: &str,
    dry_run: bool,
    api_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read pricing from file
    let pricing_content = std::fs::read_to_string(pricing_file)
        .map_err(|e| format!("Failed to read pricing file '{}': {}", pricing_file, e))?;

    let pricing: HashMap<String, serde_json::Value> = serde_json::from_str(&pricing_content)
        .map_err(|e| format!("Invalid JSON in pricing file: {}", e))?;

    // API contract: `tiers` is a `Vec<String>` defaulting to empty (= generate
    // all applicable tiers). Always send the array — poem-openapi's payload
    // parser rejects a missing/null `tiers` field before serde runs.
    let tier_vec: Vec<String> = match tiers {
        Some(list) => list.split(',').map(|s| s.trim().to_string()).collect(),
        None => Vec::new(),
    };

    let request = serde_json::json!({
        "pricing": pricing,
        "visibility": visibility,
        "dryRun": dry_run,
        "tiers": tier_vec,
    });

    let body = serde_json::to_string(&request)?;
    let pubkey_hex = hex::encode(dcc_id.to_bytes_verifying());
    let path = format!(
        "/api/v1/providers/{}/pools/{}/generate-offerings",
        pubkey_hex, pool_id
    );

    let (timestamp, nonce, signature) =
        sign_api_request(dcc_id, "POST", &path, body.as_bytes())?;

    let client = crate::utils::http_client();
    let url = format!("{}{}", api_url, path);

    let response = client
        .post(&url)
        .header(dcc_common::api_auth::HEADER_PUBLIC_KEY, &pubkey_hex)
        .header(dcc_common::api_auth::HEADER_TIMESTAMP, &timestamp)
        .header(dcc_common::api_auth::HEADER_NONCE, &nonce)
        .header(dcc_common::api_auth::HEADER_SIGNATURE, &signature)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("API request failed ({}): {}", status, body).into());
    }

    let json: serde_json::Value = response.json().await?;

    if json.get("success").and_then(|v| v.as_bool()) != Some(true) {
        let error = json
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        return Err(format!("API error: {}", error).into());
    }

    // Pretty print the result
    let data = json.get("data");
    if dry_run {
        println!("Preview mode - would create:");
    } else {
        println!("Successfully generated offerings:");
    }
    println!("{}", serde_json::to_string_pretty(&data)?);

    Ok(())
}
