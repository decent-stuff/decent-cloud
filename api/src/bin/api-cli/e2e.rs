//! E2E subcommand: provision/lifecycle/cloud-provision/all + shared e2e helpers.
use crate::api_cli::{self, Identity, SignedClient};
use crate::{
    cancel_contract, wait_for_contract_status, AddCloudAccountRequest,
    CloudAccountResponse, Contract, CreateContractRequest, Offering,
    ProvisionResourceRequest, RentalRequestResponse,
};
use anyhow::{Context, Result};
use clap::Subcommand;
use std::env;

#[derive(Subcommand)]
pub(crate) enum E2eAction {
    /// Run full provisioning E2E test
    Provision {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Offering ID
        #[arg(long)]
        offering_id: i64,
        /// SSH public key
        #[arg(long)]
        ssh_pubkey: String,
        /// Verify SSH connectivity after provisioning
        #[arg(long)]
        verify_ssh: bool,
        /// Clean up (cancel contract) after test
        #[arg(long)]
        cleanup: bool,
    },
    /// Run contract lifecycle E2E test (create → verify → cancel → verify cancelled)
    Lifecycle {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Offering ID (auto-discovered if not provided)
        #[arg(long)]
        offering_id: Option<i64>,
        /// SSH public key (dummy value used if not provided)
        #[arg(long)]
        ssh_pubkey: Option<String>,
    },
    /// Run cloud provisioning E2E test (add account → provision VM → verify → delete → cleanup)
    CloudProvision {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
    },
    /// Run all E2E tests
    All {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Offering ID (auto-discovered if not provided)
        #[arg(long)]
        offering_id: Option<i64>,
        /// SSH public key (required for provision test)
        #[arg(long)]
        ssh_pubkey: Option<String>,
        /// Skip provisioning test (slow, needs dc-agent)
        #[arg(long)]
        skip_provision: bool,
        /// Skip DNS test (needs Cloudflare credentials)
        #[arg(long)]
        skip_dns: bool,
    },
}
// =============================================================================
// E2E handlers
// =============================================================================

pub(crate) async fn handle_e2e_action(action: E2eAction, api_url: &str) -> Result<()> {
    match action {
        E2eAction::Provision {
            identity,
            offering_id,
            ssh_pubkey,
            verify_ssh,
            cleanup,
        } => {
            println!("\n========================================");
            println!("  E2E Provisioning Test");
            println!("========================================\n");

            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            // Step 1: Create contract (test payment auto-succeeds and auto-accepts)
            println!("Step 1: Creating contract...");
            let contract_id =
                create_contract_for_testing(&client, offering_id, &ssh_pubkey).await?;
            println!("  Contract created: {}", contract_id);

            // Step 2: Wait for provisioning
            println!("\nStep 2: Waiting for provisioning...");
            let contract =
                wait_for_contract_status(&client, &contract_id, "provisioned", 300).await?;

            // Step 3: Get gateway info
            println!("\nStep 3: Getting gateway information...");
            let gateway_host = contract
                .gateway_subdomain
                .context("No gateway subdomain assigned")?;
            let ssh_port = contract.gateway_ssh_port.context("No SSH port assigned")?;

            println!("  Gateway: {}", gateway_host);
            println!("  SSH Port: {}", ssh_port);

            // Step 4: Verify SSH (optional)
            if verify_ssh {
                println!("\nStep 4: Testing SSH connectivity...");
                verify_ssh_reachable(&gateway_host, ssh_port).await?;
            }

            // Step 5: Cleanup (optional)
            if cleanup {
                println!("\nStep 5: Cleaning up (cancelling contract)...");
                cancel_contract(&client, &contract_id, Some("E2E test cleanup")).await?;
                println!("  Contract cancelled");
            }

            println!("\n========================================");
            println!("  E2E Provisioning Test: SUCCESS");
            println!("========================================\n");
        }
        E2eAction::Lifecycle {
            identity,
            offering_id,
            ssh_pubkey,
        } => {
            println!("\n========================================");
            println!("  E2E Contract Lifecycle Test");
            println!("========================================\n");

            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            // Step 1: Discover offering
            let offering_id = match offering_id {
                Some(oid) => {
                    println!("Step 1: Using specified offering ID: {}", oid);
                    oid
                }
                None => {
                    println!("Step 1: Auto-discovering available offering...");
                    let offerings = fetch_offerings(api_url).await?;
                    // All offerings are real now (example/demo catalog was dropped at
                    // source via migration 053); pick the first available one.
                    let offering = offerings
                        .first()
                        .context("No offerings available for lifecycle test")?;
                    println!(
                        "  Found offering: {} (ID: {})",
                        offering.offer_name.as_deref().unwrap_or("N/A"),
                        offering.id
                    );
                    offering.id
                }
            };

            // Step 2: Create contract (test method auto-succeeds payment)
            let ssh_key = ssh_pubkey.unwrap_or_else(|| {
                "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDummy e2e-lifecycle-test".to_string()
            });
            println!("\nStep 2: Creating contract...");
            let contract_id = create_contract_for_testing(&client, offering_id, &ssh_key).await?;
            println!("  Contract created: {}", contract_id);

            // Step 3: Verify contract exists with expected status
            println!("\nStep 3: Verifying contract...");
            let path = format!("/contracts/{}", contract_id);
            let contract: Contract = client.get_api(&path).await?;
            println!(
                "  Status: {}, Payment: {}",
                contract.status, contract.payment_status
            );
            anyhow::ensure!(
                contract.status == "requested" || contract.status == "accepted",
                "Unexpected contract status: '{}' (expected 'requested' or 'accepted')",
                contract.status
            );

            // Step 4: Cancel contract
            println!("\nStep 4: Cancelling contract...");
            cancel_contract(&client, &contract_id, Some("E2E lifecycle test")).await?;
            println!("  Cancel request sent");

            // Step 5: Verify cancelled
            println!("\nStep 5: Verifying cancellation...");
            wait_for_contract_status(&client, &contract_id, "cancelled", 30).await?;

            println!("\n========================================");
            println!("  E2E Contract Lifecycle Test: SUCCESS");
            println!("========================================\n");
        }
        E2eAction::CloudProvision { identity } => {
            println!("\n========================================");
            println!("  E2E Cloud Provisioning Test");
            println!("========================================\n");

            // Agents MUST use the WRITE-capable dev token (HETZNER_API_TOKEN_DEV)
            // for cloud-provisioning E2E: this test creates AND deletes a real VM
            // via the cloud_account. The bare HETZNER_API_TOKEN is READ-ONLY on
            // the dev Hetzner project (GET works; POST/DELETE → HTTP 403), which
            // would strand the test VM, and it is no longer injected into agent
            // sessions — so this hard-requires _DEV and fails fast otherwise. See
            // repo/AGENTS.md "Hetzner tokens".
            let hetzner_token =
                env::var("HETZNER_API_TOKEN_DEV").context(
                    "HETZNER_API_TOKEN_DEV (read-write; required to create+delete test VMs) \
                     env var must be set for cloud provisioning E2E test",
                )?;

            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            // Step 1: Add Hetzner cloud account
            println!("Step 1: Adding Hetzner cloud account...");
            let account: CloudAccountResponse = client
                .post_api(
                    "/cloud-accounts",
                    &AddCloudAccountRequest {
                        backend_type: "hetzner".to_string(),
                        name: "e2e-cloud-test".to_string(),
                        credentials: hetzner_token,
                    },
                )
                .await?;
            let account_id = account.id.clone();
            println!(
                "  Account created: {} (valid: {})",
                account.id, account.is_valid
            );
            anyhow::ensure!(
                account.is_valid,
                "Cloud account validation failed: {:?}",
                account.validation_error
            );

            // Ensure cleanup on any failure
            let cleanup_result = async {
                // Step 2: Provision a cloud resource (cheapest: cx23, nbg1)
                println!("\nStep 2: Provisioning cloud resource (cx23/nbg1)...");
                let resource: serde_json::Value = client
                    .post_api(
                        "/cloud-resources",
                        &ProvisionResourceRequest {
                            cloud_account_id: account_id.clone(),
                            name: format!("e2e-test-{}", &uuid::Uuid::new_v4().to_string()[..8]),
                            server_type: "cx23".to_string(),
                            location: "nbg1".to_string(),
                            image: "ubuntu-24.04".to_string(),
                            ssh_pubkey: "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDummy e2e-cloud-test"
                                .to_string(),
                        },
                    )
                    .await?;
                let resource_id = resource["id"]
                    .as_str()
                    .context("No resource ID in response")?
                    .to_string();
                println!("  Resource created: {}", resource_id);

                // Step 3: Poll until running (timeout: 300s)
                println!("\nStep 3: Waiting for resource to be running...");
                let start = std::time::Instant::now();
                let timeout = std::time::Duration::from_secs(300);
                let poll_interval = std::time::Duration::from_secs(10);
                loop {
                    if start.elapsed() > timeout {
                        anyhow::bail!(
                            "Timeout waiting for resource {} to reach 'running' status",
                            resource_id
                        );
                    }
                    let path = format!("/cloud-resources/{}", resource_id);
                    let res: serde_json::Value = client.get_api(&path).await?;
                    let status = res["status"].as_str().unwrap_or("unknown");
                    println!(
                        "  Status: {} ({}s elapsed)",
                        status,
                        start.elapsed().as_secs()
                    );
                    match status {
                        "running" => {
                            let ip = res["publicIp"].as_str().unwrap_or("N/A");
                            let gw = res["gatewaySlug"].as_str().unwrap_or("N/A");
                            println!("  Public IP: {}, Gateway slug: {}", ip, gw);
                            break;
                        }
                        "failed" => anyhow::bail!("Resource provisioning failed"),
                        _ => tokio::time::sleep(poll_interval).await,
                    }
                }

                // Step 4: Verify SSH port 22 is reachable on the public IP
                println!("\nStep 4: Verifying SSH reachability...");
                let path = format!("/cloud-resources/{}", resource_id);
                let res: serde_json::Value = client.get_api(&path).await?;
                let public_ip = res["publicIp"]
                    .as_str()
                    .context("No public IP on resource")?;
                verify_ssh_reachable(public_ip, 22).await?;

                // Step 5: Delete resource
                println!("\nStep 5: Deleting resource...");
                let path = format!("/cloud-resources/{}", resource_id);
                let _: serde_json::Value = client.delete_api(&path).await?;
                println!("  Delete requested");

                // Step 6: Poll until deleted (timeout: 120s)
                println!("\nStep 6: Waiting for resource deletion...");
                let start = std::time::Instant::now();
                let timeout = std::time::Duration::from_secs(120);
                loop {
                    if start.elapsed() > timeout {
                        anyhow::bail!("Timeout waiting for resource deletion");
                    }
                    let path = format!("/cloud-resources/{}", resource_id);
                    match client.get_api::<serde_json::Value>(&path).await {
                        Ok(res) => {
                            let status = res["status"].as_str().unwrap_or("unknown");
                            println!(
                                "  Status: {} ({}s elapsed)",
                                status,
                                start.elapsed().as_secs()
                            );
                            if status == "deleted" {
                                break;
                            }
                            tokio::time::sleep(poll_interval).await;
                        }
                        Err(_) => {
                            // Resource gone (404) = success
                            println!("  Resource no longer found (deleted)");
                            break;
                        }
                    }
                }

                Ok::<String, anyhow::Error>(resource_id)
            }
            .await;

            // Step 7: Always cleanup the cloud account
            println!("\nStep 7: Cleaning up cloud account...");
            let delete_path = format!("/cloud-accounts/{}", account_id);
            if let Err(e) = client.delete_api::<serde_json::Value>(&delete_path).await {
                eprintln!(
                    "  WARNING: Failed to delete cloud account {}: {:#}",
                    account_id, e
                );
            } else {
                println!("  Cloud account deleted");
            }

            cleanup_result?;

            println!("\n========================================");
            println!("  E2E Cloud Provisioning Test: SUCCESS");
            println!("========================================\n");
        }
        E2eAction::All {
            identity,
            offering_id,
            ssh_pubkey,
            skip_provision,
            skip_dns,
        } => {
            println!("\n========================================");
            println!("  Running All E2E Tests");
            println!("========================================\n");

            let id = Identity::load(&identity)?;
            println!("Using identity: {}", identity);

            let mut passed = 0u32;
            let mut failed = 0u32;
            let mut skipped = 0u32;

            // Test 1: Health check
            println!("\n--- Test 1: Health Check ---");
            let http = api::http_util::http_client();
            let url = format!("{}/api/v1/offerings?limit=1", api_url);
            match http.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    println!("  PASSED");
                    passed += 1;
                }
                Ok(resp) => {
                    println!("  FAILED: API returned status {}", resp.status());
                    anyhow::bail!("API health check failed, aborting E2E suite");
                }
                Err(e) => {
                    println!("  FAILED: {}", e);
                    anyhow::bail!("API health check failed, aborting E2E suite");
                }
            }

            // Test 2: Contract lifecycle
            println!("\n--- Test 2: Contract Lifecycle ---");
            let client = SignedClient::new(&id, api_url)?;
            let discovered_offering_id = match offering_id {
                Some(oid) => Some(oid),
                None => match fetch_offerings(api_url).await {
                    Ok(offerings) if !offerings.is_empty() => {
                        // All offerings are real now (example/demo catalog was dropped
                        // at source via migration 053); use the first available.
                        let offering = offerings
                            .first()
                            .unwrap();
                        println!(
                            "  Auto-discovered offering: {} (ID: {})",
                            offering.offer_name.as_deref().unwrap_or("N/A"),
                            offering.id
                        );
                        Some(offering.id)
                    }
                    Ok(_) => {
                        println!("  SKIPPED: No offerings available");
                        skipped += 1;
                        None
                    }
                    Err(e) => {
                        println!("  FAILED: Could not fetch offerings: {}", e);
                        failed += 1;
                        None
                    }
                },
            };

            if let Some(oid) = discovered_offering_id {
                let ssh_key = ssh_pubkey
                    .as_deref()
                    .unwrap_or("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDummy e2e-all-test");
                match async {
                    let cid = create_contract_for_testing(&client, oid, ssh_key).await?;
                    println!("  Contract created: {}", cid);
                    let path = format!("/contracts/{}", cid);
                    let contract: Contract = client.get_api(&path).await?;
                    anyhow::ensure!(
                        contract.status == "requested" || contract.status == "accepted",
                        "Unexpected status: '{}'",
                        contract.status
                    );
                    cancel_contract(&client, &cid, Some("E2E all test")).await?;
                    wait_for_contract_status(&client, &cid, "cancelled", 30).await?;
                    Ok::<(), anyhow::Error>(())
                }
                .await
                {
                    Ok(()) => {
                        println!("  PASSED");
                        passed += 1;
                    }
                    Err(e) => {
                        println!("  FAILED: {}", e);
                        failed += 1;
                    }
                }
            }

            // Test 3: Provisioning
            println!("\n--- Test 3: Provisioning ---");
            if skip_provision {
                println!("  SKIPPED: --skip-provision flag set");
                skipped += 1;
            } else if ssh_pubkey.is_none() {
                println!("  SKIPPED: --ssh-pubkey not provided (required for provision test)");
                skipped += 1;
            } else if let Some(oid) = discovered_offering_id {
                let ssh_key = ssh_pubkey.as_deref().unwrap();
                match async {
                    let cid = create_contract_for_testing(&client, oid, ssh_key).await?;
                    println!("  Contract created: {}", cid);

                    let contract =
                        wait_for_contract_status(&client, &cid, "provisioned", 300).await?;
                    let gateway_host = contract
                        .gateway_subdomain
                        .context("No gateway subdomain assigned")?;
                    let port = contract.gateway_ssh_port.context("No SSH port assigned")?;
                    println!("  Gateway: {}:{}", gateway_host, port);

                    // Verify SSH port reachable via gateway hostname (with DNS propagation retries)
                    verify_ssh_reachable(&gateway_host, port).await?;

                    // Cleanup
                    cancel_contract(&client, &cid, Some("E2E all provision cleanup")).await?;
                    Ok::<(), anyhow::Error>(())
                }
                .await
                {
                    Ok(()) => {
                        println!("  PASSED");
                        passed += 1;
                    }
                    Err(e) => {
                        println!("  FAILED: {}", e);
                        failed += 1;
                    }
                }
            } else {
                println!("  SKIPPED: No offering available");
                skipped += 1;
            }

            // Test 4: DNS
            println!("\n--- Test 4: DNS ---");
            if skip_dns {
                println!("  SKIPPED: --skip-dns flag set");
                skipped += 1;
            } else if env::var("CLOUDFLARE_API_TOKEN").is_err()
                || env::var("CLOUDFLARE_ZONE_ID").is_err()
            {
                println!("  SKIPPED: CLOUDFLARE_API_TOKEN or CLOUDFLARE_ZONE_ID not set");
                skipped += 1;
            } else {
                match run_dns_e2e_test().await {
                    Ok(()) => {
                        println!("  PASSED");
                        passed += 1;
                    }
                    Err(e) => {
                        println!("  FAILED: {}", e);
                        failed += 1;
                    }
                }
            }

            // Summary
            println!("\n========================================");
            println!("  E2E Test Summary");
            println!("========================================");
            println!("  Passed:  {}", passed);
            println!("  Failed:  {}", failed);
            println!("  Skipped: {}", skipped);
            println!("========================================\n");

            anyhow::ensure!(failed == 0, "{} E2E test(s) failed", failed);
        }
    }
    Ok(())
}

async fn fetch_offerings(api_url: &str) -> Result<Vec<Offering>> {
    let http = api::http_util::http_client();
    let url = format!("{}/api/v1/offerings?limit=50&in_stock_only=true", api_url);
    let response = http.get(&url).send().await?;
    let text = response.text().await?;
    let api_response: api_cli::client::ApiResponse<Vec<Offering>> = serde_json::from_str(&text)?;
    api_response.into_result()
}
async fn verify_ssh_reachable(gateway_host: &str, port: i32) -> Result<()> {
    use tokio::net::TcpStream;

    let addr = format!("{}:{}", gateway_host, port);
    let max_attempts = 6; // 60s total (enough for DNS propagation)
    let retry_interval = std::time::Duration::from_secs(10);

    for attempt in 1..=max_attempts {
        match tokio::time::timeout(std::time::Duration::from_secs(5), TcpStream::connect(&addr))
            .await
        {
            Ok(Ok(_)) => {
                println!("  SSH port reachable at {}", addr);
                return Ok(());
            }
            Ok(Err(e)) if attempt < max_attempts => {
                println!(
                    "  Attempt {}/{}: {} (retrying in {}s...)",
                    attempt,
                    max_attempts,
                    e,
                    retry_interval.as_secs()
                );
                tokio::time::sleep(retry_interval).await;
            }
            Ok(Err(e)) => {
                anyhow::bail!(
                    "SSH not reachable at {} after {} attempts: {}\n\
                     Troubleshooting:\n\
                     - Check DNS: dig {} A\n\
                     - Check gateway iptables: ssh <provider> iptables -t nat -L DC_GATEWAY -n\n\
                     - Check dc-agent logs: ssh <provider> journalctl -u dc-agent --since '5 min ago'\n\
                     - Check Caddy config: ssh <provider> ls /etc/caddy/sites/",
                    addr, max_attempts, e, gateway_host
                );
            }
            Err(_) if attempt < max_attempts => {
                println!(
                    "  Attempt {}/{}: connection timeout (retrying in {}s...)",
                    attempt,
                    max_attempts,
                    retry_interval.as_secs()
                );
                tokio::time::sleep(retry_interval).await;
            }
            Err(_) => {
                anyhow::bail!(
                    "SSH connection to {} timed out after {} attempts\n\
                     Troubleshooting:\n\
                     - Check DNS: dig {} A\n\
                     - Verify port {} is open on the gateway\n\
                     - Check dc-agent logs: ssh <provider> journalctl -u dc-agent --since '5 min ago'",
                    addr, max_attempts, gateway_host, port
                );
            }
        }
    }
    anyhow::bail!("SSH wait loop exited unexpectedly - this is a bug")
}

async fn create_contract_for_testing(
    client: &SignedClient,
    offering_id: i64,
    ssh_pubkey: &str,
) -> Result<String> {
    let request = CreateContractRequest {
        offering_db_id: offering_id,
        ssh_pubkey: Some(ssh_pubkey.to_string()),
        duration_hours: Some(1),
        payment_method: Some("test".to_string()),
    };
    let response: RentalRequestResponse = client.post_api("/contracts", &request).await?;
    Ok(response.contract_id)
}

async fn run_dns_e2e_test() -> Result<()> {
    let api_token = env::var("CLOUDFLARE_API_TOKEN").context("CLOUDFLARE_API_TOKEN not set")?;
    let zone_id = env::var("CLOUDFLARE_ZONE_ID").context("CLOUDFLARE_ZONE_ID not set")?;
    let gw_prefix = env::var("CF_GW_PREFIX").unwrap_or_else(|_| "gw".to_string());
    let domain = env::var("CF_DOMAIN").unwrap_or_else(|_| "decent-cloud.org".to_string());
    let base_domain = format!("{}.{}", gw_prefix, domain);

    let http = api::http_util::http_client();
    let base_url = format!(
        "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
        zone_id
    );

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let subdomain = format!("e2e-test-{}", timestamp);
    let full_name = format!("{}.{}", subdomain, base_domain);
    let test_ip = "127.0.0.1";
    let lookup_url = format!("{}?name={}", base_url, urlencoding::encode(&full_name));

    // Create test A record
    println!("  Creating DNS record: {} -> {}", full_name, test_ip);
    let params = serde_json::json!({
        "type": "A",
        "name": full_name,
        "content": test_ip,
        "ttl": 300,
        "proxied": false,
    });
    let response = http
        .post(&base_url)
        .header("Authorization", format!("Bearer {}", api_token))
        .json(&params)
        .send()
        .await?;
    let text = response.text().await?;
    let json: serde_json::Value = serde_json::from_str(&text)?;
    anyhow::ensure!(
        json["success"].as_bool().unwrap_or(false),
        "Failed to create DNS record: {}",
        text
    );
    let record_id = json["result"]["id"]
        .as_str()
        .context("No record ID in create response")?
        .to_string();
    println!("  Created record: {}", record_id);

    // Verify record exists
    println!("  Verifying record exists...");
    let response = http
        .get(&lookup_url)
        .header("Authorization", format!("Bearer {}", api_token))
        .send()
        .await?;
    let text = response.text().await?;
    let json: serde_json::Value = serde_json::from_str(&text)?;
    let records = json["result"]
        .as_array()
        .context("No result array in response")?;
    anyhow::ensure!(!records.is_empty(), "Record not found after creation");
    println!("  Record verified");

    // Delete record
    println!("  Deleting record...");
    let delete_url = format!("{}/{}", base_url, record_id);
    let response = http
        .delete(&delete_url)
        .header("Authorization", format!("Bearer {}", api_token))
        .send()
        .await?;
    anyhow::ensure!(
        response.status().is_success(),
        "Failed to delete DNS record: {}",
        response.status()
    );
    println!("  Record deleted");

    // Verify deletion
    println!("  Verifying deletion...");
    let response = http
        .get(&lookup_url)
        .header("Authorization", format!("Bearer {}", api_token))
        .send()
        .await?;
    let text = response.text().await?;
    let json: serde_json::Value = serde_json::from_str(&text)?;
    let records = json["result"]
        .as_array()
        .context("No result array in response")?;
    anyhow::ensure!(records.is_empty(), "Record still exists after deletion");
    println!("  Deletion verified");

    Ok(())
}
