//! Contract subcommand: list-offerings/create/get/wait/list/cancel.
use crate::api_cli::{self, Identity, SignedClient};
use crate::{cancel_contract, wait_for_contract_status, Contract, CreateContractRequest, Offering, RentalRequestResponse};
use anyhow::Result;
use clap::Subcommand;
#[derive(Subcommand)]
pub(crate) enum ContractAction {
    /// List available offerings
    ListOfferings {
        /// Filter by provider public key
        #[arg(long)]
        provider: Option<String>,
        /// Filter by product type
        #[arg(long)]
        product_type: Option<String>,
        /// Only show in-stock offerings
        #[arg(long)]
        in_stock_only: bool,
        /// Maximum number of results
        #[arg(long, default_value = "50")]
        limit: i32,
    },
    /// Create a rental contract
    Create {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Offering database ID
        #[arg(long)]
        offering_id: i64,
        /// SSH public key for VM access
        #[arg(long)]
        ssh_pubkey: String,
        /// Duration in hours
        #[arg(long, default_value = "1")]
        duration_hours: i64,
        /// Skip payment (testing only - marks payment as succeeded)
        #[arg(long)]
        skip_payment: bool,
    },
    /// Get contract details
    Get {
        /// Contract ID (UUID)
        contract_id: String,
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
    },
    /// Wait for contract to reach a state
    Wait {
        /// Contract ID (UUID)
        contract_id: String,
        /// Target state (pending, provisioned, cancelled, etc.)
        #[arg(long)]
        state: String,
        /// Timeout in seconds
        #[arg(long, default_value = "300")]
        timeout: u64,
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
    },
    /// List contracts for the authenticated user
    List {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
    },
    /// Cancel a contract
    Cancel {
        /// Contract ID (UUID)
        contract_id: String,
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Cancellation memo
        #[arg(long)]
        memo: Option<String>,
    },
}
pub(crate) async fn handle_contract_action(action: ContractAction, api_url: &str) -> Result<()> {
    match action {
        ContractAction::ListOfferings {
            provider,
            product_type,
            in_stock_only,
            limit,
        } => {
            // Use a dummy identity just for public endpoint access
            let http = api::http_util::http_client();
            let mut url = format!("{}/api/v1/offerings?limit={}", api_url, limit);
            if let Some(p) = provider {
                url.push_str(&format!("&provider={}", p));
            }
            if let Some(pt) = product_type {
                url.push_str(&format!("&product_type={}", pt));
            }
            if in_stock_only {
                url.push_str("&in_stock_only=true");
            }

            let response = http.get(&url).send().await?;
            let text = response.text().await?;
            let api_response: api_cli::client::ApiResponse<Vec<Offering>> =
                serde_json::from_str(&text)?;
            let offerings = api_response.into_result()?;

            if offerings.is_empty() {
                println!("No offerings found.");
            } else {
                println!("\nAvailable Offerings:");
                println!("{}", "=".repeat(120));
                println!(
                    "{:<8} {:<40} {:<15} {:<20} {:<10} {:<10}",
                    "ID", "Name", "Type", "Provider", "Price/mo", "Stock"
                );
                println!("{}", "-".repeat(120));
                for o in &offerings {
                    let name = o.offer_name.as_deref().unwrap_or("N/A");
                    let ptype = o.product_type.as_deref().unwrap_or("N/A");
                    let price = o
                        .monthly_price
                        .map(|p| format!("${:.2}", p))
                        .unwrap_or_else(|| "N/A".to_string());
                    let stock = o.stock_status.as_deref().unwrap_or("N/A");
                    let provider_short = if o.pubkey.len() > 16 {
                        format!("{}...", &o.pubkey[..16])
                    } else {
                        o.pubkey.clone()
                    };
                    println!(
                        "{:<8} {:<40} {:<15} {:<20} {:<10} {:<10}",
                        o.id,
                        &name[..name.len().min(38)],
                        ptype,
                        provider_short,
                        price,
                        stock
                    );
                }
                println!("{}", "=".repeat(120));
                println!("Total: {} offering(s)", offerings.len());
            }
        }
        ContractAction::Create {
            identity,
            offering_id,
            ssh_pubkey,
            duration_hours,
            skip_payment,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let payment_method = if skip_payment {
                // For testing: use the "test" payment method that auto-succeeds without checkout
                Some("test".to_string())
            } else {
                Some("stripe".to_string())
            };

            let request = CreateContractRequest {
                offering_db_id: offering_id,
                ssh_pubkey: Some(ssh_pubkey),
                duration_hours: Some(duration_hours),
                payment_method,
            };

            let response: RentalRequestResponse = client.post_api("/contracts", &request).await?;
            println!("Contract created:");
            println!("  Contract ID: {}", response.contract_id);
            println!("  Message: {}", response.message);
            if let Some(url) = response.checkout_url {
                println!("  Checkout URL: {}", url);
            }
            if skip_payment {
                println!("\nNote: --skip-payment used the test method (payment auto-succeeds).");
            }
        }
        ContractAction::Get {
            contract_id,
            identity,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let path = format!("/contracts/{}", contract_id);
            let contract: Contract = client.get_api(&path).await?;
            println!("Contract: {}", contract.contract_id);
            println!("  Status: {}", contract.status);
            println!("  Payment status: {}", contract.payment_status);
            if let Some(subdomain) = &contract.gateway_subdomain {
                println!("  Gateway: {}", subdomain);
            } else if let Some(slug) = &contract.gateway_slug {
                println!("  Gateway slug: {} (no subdomain stored)", slug);
            }
            if let Some(port) = contract.gateway_ssh_port {
                println!("  SSH port: {}", port);
            }
            if let (Some(start), Some(end)) = (
                contract.gateway_port_range_start,
                contract.gateway_port_range_end,
            ) {
                println!("  Port range: {}-{}", start, end);
            }
            if let Some(details) = &contract.provisioning_instance_details {
                println!("  Instance details: {}", details);
            }
        }
        ContractAction::Wait {
            contract_id,
            state,
            timeout,
            identity,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;
            wait_for_contract_status(&client, &contract_id, &state, timeout).await?;
        }
        ContractAction::List { identity } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let path = format!("/users/{}/contracts", id.public_key_hex);
            let contracts: Vec<Contract> = client.get_api(&path).await?;

            if contracts.is_empty() {
                println!("No contracts found.");
            } else {
                println!("\nContracts:");
                println!("{}", "=".repeat(100));
                println!(
                    "{:<38} {:<15} {:<15} {:<20}",
                    "Contract ID", "Status", "Payment", "Gateway"
                );
                println!("{}", "-".repeat(100));
                for c in &contracts {
                    let gateway = c
                        .gateway_slug
                        .as_ref()
                        .map(|s| format!("{}.gw...", s))
                        .unwrap_or_else(|| "N/A".to_string());
                    println!(
                        "{:<38} {:<15} {:<15} {:<20}",
                        c.contract_id, c.status, c.payment_status, gateway
                    );
                }
                println!("{}", "=".repeat(100));
                println!("Total: {} contract(s)", contracts.len());
            }
        }
        ContractAction::Cancel {
            contract_id,
            identity,
            memo,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;
            cancel_contract(&client, &contract_id, memo.as_deref()).await?;
            println!("Contract {} cancelled.", contract_id);
        }
    }
    Ok(())
}

