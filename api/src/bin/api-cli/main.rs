//! `api-cli` — admin / testing / E2E CLI for the Decent Cloud API.
//!
//! Each subcommand domain lives in its own module (`identity`, `account`,
//! `contract`, …, `recipe`). This file holds only the top-level `clap` wiring
//! (`Cli` / `Environment` / `Commands` / `main` dispatch) plus the shared
//! request/response DTOs and cross-domain helpers (DB connect, contract
//! lifecycle wait/cancel, test-email). Those shared items stay private here;
//! the subcommand modules are descendants of this crate root and reach them via
//! `crate::…`, mirroring how `providers.rs` keeps its shared helpers.

mod account;
mod admin;
mod api_cli;
mod cloud;
mod contract;
mod dns;
mod e2e;
mod gateway;
mod health;
mod identity;
mod notify;
mod offering;
mod provider;
mod recipe;

use account::{handle_account_action, AccountAction};
use admin::{handle_admin_action, AdminAction};
use cloud::{handle_cloud_action, CloudAction};
use contract::{handle_contract_action, ContractAction};
use dns::{handle_dns_action, DnsAction};
use e2e::{handle_e2e_action, E2eAction};
use gateway::{handle_gateway_action, GatewayAction};
use health::{handle_health_action, HealthAction};
use identity::{handle_identity_action, IdentityAction};
use notify::{handle_notify_action, NotifyAction};
use offering::{handle_offering_action, OfferingAction};
use provider::{handle_provider_action, ProviderAction};
use recipe::{handle_recipe_action, RecipeAction};

use anyhow::{Context, Result};
use api::database::Database;
use clap::{Parser, Subcommand, ValueEnum};
use email_utils::{validate_email, EmailService};
use serde::{Deserialize, Serialize};
use std::env;

const DEFAULT_DEV_API_URL: &str = "http://localhost:3000";
const DEFAULT_PROD_API_URL: &str = "https://api.decent-cloud.org";

#[derive(Parser)]
#[command(name = "api-cli")]
#[command(about = "Decent Cloud API CLI for admin, testing, and E2E scenarios")]
struct Cli {
    /// Environment (dev or prod)
    #[arg(long, default_value = "dev")]
    env: Environment,

    /// API base URL (overrides environment default)
    #[arg(long)]
    api_url: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, ValueEnum)]
enum Environment {
    Dev,
    Prod,
}

#[derive(Subcommand)]
enum Commands {
    /// Keypair management for testing
    Identity {
        #[command(subcommand)]
        action: IdentityAction,
    },
    /// Account operations
    Account {
        #[command(subcommand)]
        action: AccountAction,
    },
    /// Contract lifecycle management
    Contract {
        #[command(subcommand)]
        action: ContractAction,
    },
    /// Offering management
    Offering {
        #[command(subcommand)]
        action: OfferingAction,
    },
    /// Provider operations
    Provider {
        #[command(subcommand)]
        action: ProviderAction,
    },
    /// Test notifications
    Notify {
        #[command(subcommand)]
        action: NotifyAction,
    },
    /// Cloudflare DNS operations
    Dns {
        #[command(subcommand)]
        action: DnsAction,
    },
    /// Gateway connectivity testing
    Gateway {
        #[command(subcommand)]
        action: GatewayAction,
    },
    /// Service health checks
    Health {
        #[command(subcommand)]
        action: HealthAction,
    },
    /// End-to-end test scenarios
    E2e {
        #[command(subcommand)]
        action: E2eAction,
    },
    /// Admin account management
    Admin {
        #[command(subcommand)]
        action: AdminAction,
    },
    /// Cloud account and resource management (Hetzner, Proxmox)
    Cloud {
        #[command(subcommand)]
        action: CloudAction,
    },
    /// Recipe validation and dry-run
    Recipe {
        #[command(subcommand)]
        action: RecipeAction,
    },
    /// Send test email (for testing email configuration)
    TestEmail {
        /// Recipient email address
        #[arg(long)]
        to: String,
        /// Test DKIM signing (default: false)
        #[arg(long)]
        with_dkim: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load environment-specific .env file
    let api_url = match cli.env {
        Environment::Dev => {
            dotenv::from_filename("/code/api/.env").ok();
            cli.api_url
                .unwrap_or_else(|| DEFAULT_DEV_API_URL.to_string())
        }
        Environment::Prod => {
            dotenv::from_filename("/code/cf/.env.prod").ok();
            cli.api_url
                .unwrap_or_else(|| DEFAULT_PROD_API_URL.to_string())
        }
    };

    match cli.command {
        Commands::Identity { action } => handle_identity_action(action).await,
        Commands::Account { action } => handle_account_action(action, &api_url).await,
        Commands::Contract { action } => handle_contract_action(action, &api_url).await,
        Commands::Offering { action } => handle_offering_action(action, &api_url).await,
        Commands::Provider { action } => handle_provider_action(action, &api_url).await,
        Commands::Notify { action } => handle_notify_action(action).await,
        Commands::Dns { action } => handle_dns_action(action).await,
        Commands::Gateway { action } => handle_gateway_action(action, &api_url).await,
        Commands::Health { action } => handle_health_action(action, &api_url).await,
        Commands::E2e { action } => handle_e2e_action(action, &api_url).await,
        Commands::Admin { action } => handle_admin_action(action).await,
        Commands::Cloud { action } => handle_cloud_action(action, &api_url).await,
        Commands::Recipe { action } => handle_recipe_action(action, &api_url).await,
        Commands::TestEmail { to, with_dkim } => handle_test_email(&to, with_dkim).await,
    }
}

// =============================================================================
// Shared request/response DTOs (used by contract / offering / provider / gateway / e2e)
// =============================================================================

#[derive(Debug, Serialize)]
struct CreateContractRequest {
    offering_db_id: i64,
    ssh_pubkey: Option<String>,
    duration_hours: Option<i64>,
    payment_method: Option<String>,
}

#[derive(Debug, Serialize)]
struct CancelContractRequest {
    memo: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Offering {
    id: i64,
    offering_id: String,
    #[serde(alias = "provider_pubkey")]
    pubkey: String,
    product_type: Option<String>,
    #[serde(alias = "name")]
    offer_name: Option<String>,
    #[serde(alias = "price_monthly_usd")]
    monthly_price: Option<f64>,
    stock_status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Contract {
    contract_id: String,
    status: String,
    payment_status: String,
    gateway_slug: Option<String>,
    gateway_subdomain: Option<String>,
    gateway_ssh_port: Option<i32>,
    gateway_port_range_start: Option<i32>,
    gateway_port_range_end: Option<i32>,
    provisioning_instance_details: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RentalRequestResponse {
    contract_id: String,
    message: String,
    checkout_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CloudAccountResponse {
    id: String,
    backend_type: String,
    name: String,
    is_valid: bool,
    validation_error: Option<String>,
    created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AddCloudAccountRequest {
    backend_type: String,
    name: String,
    credentials: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProvisionResourceRequest {
    cloud_account_id: String,
    name: String,
    server_type: String,
    location: String,
    image: String,
    ssh_pubkey: String,
}

// =============================================================================
// Shared helpers (DB connect, contract lifecycle wait/cancel, test email)
// =============================================================================

async fn connect_db() -> Result<Database> {
    let db_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| api::database::DEFAULT_DATABASE_URL.to_string());
    Database::connect(&db_url).await
}

/// Status progression order for matching "at least" semantics.
/// A contract waiting for "provisioned" should also succeed if it reaches "active".
const STATUS_PROGRESSION: &[&str] = &[
    "requested",
    "pending",
    "accepted",
    "provisioning",
    "provisioned",
    "active",
];

fn status_rank(status: &str) -> Option<usize> {
    STATUS_PROGRESSION.iter().position(|&s| s == status)
}

async fn wait_for_contract_status(
    client: &api_cli::SignedClient,
    contract_id: &str,
    target: &str,
    timeout_secs: u64,
) -> Result<Contract> {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs);
    let poll_interval = std::time::Duration::from_secs(10);
    let target_rank = status_rank(target);

    println!(
        "Waiting for contract {} to reach state '{}'...",
        contract_id, target
    );

    loop {
        let path = format!("/contracts/{}", contract_id);
        let contract: Contract = client.get_api(&path).await?;

        // Exact match always succeeds
        if contract.status == target {
            println!(
                "Contract reached state '{}' after {:?}",
                target,
                start.elapsed()
            );
            return Ok(contract);
        }

        // If the contract has progressed past the target state, also succeed
        if let (Some(current_rank), Some(target_r)) = (status_rank(&contract.status), target_rank) {
            if current_rank > target_r {
                println!(
                    "Contract reached state '{}' (past target '{}') after {:?}",
                    contract.status,
                    target,
                    start.elapsed()
                );
                return Ok(contract);
            }
        }

        // Bail on terminal states (unless we're waiting for that state)
        let terminal_states = ["cancelled", "rejected", "failed"];
        if terminal_states.contains(&contract.status.as_str()) && contract.status != target {
            anyhow::bail!(
                "Contract reached terminal state '{}' while waiting for '{}'",
                contract.status,
                target
            );
        }

        if start.elapsed() > timeout {
            anyhow::bail!(
                "Timeout waiting for contract to reach state '{}'. Current state: '{}'",
                target,
                contract.status
            );
        }

        println!(
            "  Current state: '{}', waiting... ({:.0}s elapsed)",
            contract.status,
            start.elapsed().as_secs_f64()
        );
        tokio::time::sleep(poll_interval).await;
    }
}

async fn cancel_contract(
    client: &api_cli::SignedClient,
    contract_id: &str,
    memo: Option<&str>,
) -> Result<()> {
    let request = CancelContractRequest {
        memo: memo.map(|m| m.to_string()),
    };
    let path = format!("/contracts/{}/cancel", contract_id);
    let _: String = client.put_api(&path, &request).await?;
    Ok(())
}

async fn handle_test_email(to: &str, with_dkim: bool) -> Result<()> {
    println!("\n========================================");
    println!("  Email Configuration Test");
    println!("========================================\n");

    if let Err(e) = validate_email(to) {
        anyhow::bail!("Invalid email address: {}", e);
    }

    let api_key = env::var("MAILCHANNELS_API_KEY").context("MAILCHANNELS_API_KEY not set")?;

    if api_key.is_empty() {
        anyhow::bail!("MAILCHANNELS_API_KEY is empty");
    }
    println!("✓ MailChannels API key found");

    let (dkim_domain, dkim_selector, dkim_private_key) = if with_dkim {
        let domain = env::var("DKIM_DOMAIN").ok();
        let selector = env::var("DKIM_SELECTOR").ok();
        let private_key = env::var("DKIM_PRIVATE_KEY").ok();

        match (&domain, &selector, &private_key) {
            (Some(d), Some(s), Some(k)) if !d.is_empty() && !s.is_empty() && !k.is_empty() => {
                println!("✓ DKIM configuration found:");
                println!("  - Domain: {}", d);
                println!("  - Selector: {}", s);
                (domain, selector, private_key)
            }
            _ => {
                eprintln!("DKIM requested but incomplete. Proceeding without DKIM.");
                (None, None, None)
            }
        }
    } else {
        println!("✓ DKIM signing: disabled");
        (None, None, None)
    };

    let email_service = EmailService::new(api_key, dkim_domain, dkim_selector, dkim_private_key);

    let from_addr = "noreply@decent-cloud.org";
    let subject = "Decent Cloud Email Test";
    let body = format!(
        "This is a test email from the Decent Cloud API CLI.\n\n\
        Test details:\n\
        - Recipient: {}\n\
        - DKIM signing: {}\n\
        - Timestamp: {}\n\n\
        If you received this email, your configuration is working!\n\n\
        Best regards,\n\
        The Decent Cloud Team",
        to,
        if with_dkim { "enabled" } else { "disabled" },
        chrono::Utc::now().to_rfc3339()
    );

    println!("\nSending test email...");
    println!("  From: {}", from_addr);
    println!("  To: {}", to);

    email_service
        .send_email(from_addr, to, subject, &body, false)
        .await?;
    println!("\n✓ SUCCESS! Test email sent.");
    println!("Please check your inbox at: {}", to);
    Ok(())
}
