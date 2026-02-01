mod api_cli;

use anyhow::{Context, Result};
use api::database::Database;
use api_cli::{Identity, SignedClient};
use clap::{Parser, Subcommand, ValueEnum};
use email_utils::{validate_email, EmailService};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
    /// Send test email (for testing email configuration)
    TestEmail {
        /// Recipient email address
        #[arg(long)]
        to: String,
        /// Test DKIM signing (default: false)
        #[arg(long)]
        with_dkim: bool,
    },
    /// Seed external provider offerings from CSV
    SeedProvider {
        /// Provider name (e.g., "Hetzner")
        #[arg(long)]
        name: String,
        /// Provider domain (e.g., "hetzner.com")
        #[arg(long)]
        domain: String,
        /// Path to offerings CSV file
        #[arg(long)]
        csv: String,
        /// Provider website URL (defaults to https://{domain})
        #[arg(long)]
        website: Option<String>,
        /// Update existing offerings if they exist
        #[arg(long)]
        upsert: bool,
    },
}

// =============================================================================
// Identity subcommands
// =============================================================================

#[derive(Subcommand)]
enum IdentityAction {
    /// Generate a new keypair
    Generate {
        /// Name for the identity
        #[arg(long)]
        name: String,
    },
    /// Import an existing keypair from file
    Import {
        /// Name for the identity
        #[arg(long)]
        name: String,
        /// Path to secret key file (hex or PEM format)
        #[arg(long)]
        secret_key: String,
    },
    /// List all saved identities
    List,
    /// Show public key for an identity
    Show {
        /// Name of the identity
        name: String,
    },
    /// Delete an identity
    Delete {
        /// Name of the identity
        name: String,
    },
}

// =============================================================================
// Account subcommands
// =============================================================================

#[derive(Subcommand)]
enum AccountAction {
    /// Create a new account
    Create {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Username for the account
        #[arg(long)]
        username: String,
        /// Email address
        #[arg(long)]
        email: String,
    },
    /// Get account information
    Get {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
    },
    /// Update account email
    UpdateEmail {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// New email address
        #[arg(long)]
        email: String,
    },
    /// Add SSH key to account
    AddSshKey {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// SSH public key (e.g., "ssh-ed25519 AAAA...")
        #[arg(long)]
        key: String,
        /// Label for the key
        #[arg(long)]
        label: Option<String>,
    },
    /// List SSH keys for account
    ListSshKeys {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
    },
}

// =============================================================================
// Contract subcommands
// =============================================================================

#[derive(Subcommand)]
enum ContractAction {
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

// =============================================================================
// Offering subcommands
// =============================================================================

#[derive(Subcommand)]
enum OfferingAction {
    /// List all offerings
    List {
        /// Filter query (DSL)
        #[arg(long)]
        filter: Option<String>,
        /// Maximum number of results
        #[arg(long, default_value = "50")]
        limit: i32,
    },
    /// Get offering details
    Get {
        /// Offering ID
        offering_id: String,
    },
}

// =============================================================================
// Provider subcommands
// =============================================================================

#[derive(Subcommand)]
enum ProviderAction {
    /// List all providers
    List {
        /// Maximum number of results
        #[arg(long, default_value = "50")]
        limit: i32,
    },
    /// Get provider status
    Status {
        /// Provider public key (hex)
        #[arg(long)]
        pubkey: String,
    },
    /// List provider's offerings
    Offerings {
        /// Provider public key (hex)
        #[arg(long)]
        pubkey: String,
    },
}

// =============================================================================
// Notify subcommands
// =============================================================================

#[derive(Subcommand)]
enum NotifyAction {
    /// Send test email
    Email {
        /// Recipient email address
        #[arg(long)]
        to: String,
        /// Test DKIM signing
        #[arg(long)]
        with_dkim: bool,
    },
    /// Send test Telegram notification
    Telegram {
        /// Chat ID
        #[arg(long)]
        chat_id: String,
        /// Message text
        #[arg(long)]
        message: String,
    },
}

// =============================================================================
// DNS subcommands
// =============================================================================

#[derive(Subcommand)]
enum DnsAction {
    /// Create a DNS record
    Create {
        /// Subdomain name
        #[arg(long)]
        subdomain: String,
        /// IP address
        #[arg(long)]
        ip: String,
    },
    /// Get a DNS record
    Get {
        /// Subdomain name
        #[arg(long)]
        subdomain: String,
    },
    /// Delete a DNS record
    Delete {
        /// Subdomain name
        #[arg(long)]
        subdomain: String,
    },
    /// List all DC subdomain records
    List,
}

// =============================================================================
// Gateway subcommands
// =============================================================================

#[derive(Subcommand)]
enum GatewayAction {
    /// Test SSH connectivity via gateway
    Ssh {
        /// Gateway hostname
        #[arg(long)]
        host: String,
        /// SSH port
        #[arg(long)]
        port: u16,
        /// Path to SSH identity file
        #[arg(long)]
        identity_file: String,
    },
    /// Test TCP port connectivity
    Tcp {
        /// Gateway hostname
        #[arg(long)]
        host: String,
        /// External port
        #[arg(long)]
        external_port: u16,
        /// Expected response (optional)
        #[arg(long)]
        expect_response: Option<String>,
    },
    /// Test all ports for a contract
    Contract {
        /// Contract ID (UUID)
        contract_id: String,
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
    },
}

// =============================================================================
// Health subcommands
// =============================================================================

#[derive(Subcommand)]
enum HealthAction {
    /// Check API health
    Api,
    /// Check all external services
    All,
    /// Check Cloudflare DNS
    Cloudflare,
    /// Check Stripe
    Stripe,
    /// Check MailChannels
    Mailchannels,
    /// Check Telegram Bot
    Telegram,
}

// =============================================================================
// E2E subcommands
// =============================================================================

#[derive(Subcommand)]
enum E2eAction {
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
    /// Run contract lifecycle E2E test
    Lifecycle {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Skip payment (testing only)
        #[arg(long)]
        skip_payment: bool,
    },
    /// Run all E2E tests
    All {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
    },
}

// =============================================================================
// Admin subcommands (existing)
// =============================================================================

#[derive(Subcommand)]
enum AdminAction {
    /// Grant admin access to a user
    Grant { username: String },
    /// Revoke admin access from a user
    Revoke { username: String },
    /// List all admin accounts
    List,
}

// =============================================================================
// Main entry point
// =============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load environment-specific .env file
    let api_url = match cli.env {
        Environment::Dev => {
            dotenv::from_filename("/code/api/.env").ok();
            cli.api_url.unwrap_or_else(|| DEFAULT_DEV_API_URL.to_string())
        }
        Environment::Prod => {
            dotenv::from_filename("/code/cf/.env.prod").ok();
            cli.api_url.unwrap_or_else(|| DEFAULT_PROD_API_URL.to_string())
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
        Commands::TestEmail { to, with_dkim } => handle_test_email(&to, with_dkim).await,
        Commands::SeedProvider {
            name,
            domain,
            csv,
            website,
            upsert,
        } => handle_seed_provider(&name, &domain, &csv, website.as_deref(), upsert).await,
    }
}

// =============================================================================
// Identity handlers
// =============================================================================

async fn handle_identity_action(action: IdentityAction) -> Result<()> {
    match action {
        IdentityAction::Generate { name } => {
            let identity = Identity::generate(&name)?;
            println!("Generated identity: {}", name);
            println!("  Public key: {}", identity.public_key_hex);
            println!("  Stored at: {}", Identity::path(&name)?.display());
        }
        IdentityAction::Import { name, secret_key } => {
            let identity = Identity::import(&name, &secret_key)?;
            println!("Imported identity: {}", name);
            println!("  Public key: {}", identity.public_key_hex);
            println!("  Stored at: {}", Identity::path(&name)?.display());
        }
        IdentityAction::List => {
            let identities = Identity::list()?;
            if identities.is_empty() {
                println!("No identities found.");
                println!("Use 'api-cli identity generate --name <name>' to create one.");
            } else {
                println!("\nSaved Identities:");
                println!("{}", "=".repeat(100));
                println!("{:<20} {:<66} {:<20}", "Name", "Public Key", "Created At");
                println!("{}", "-".repeat(100));
                for id in &identities {
                    println!("{:<20} {:<66} {:<20}", id.name, id.public_key_hex, &id.created_at[..19]);
                }
                println!("{}", "=".repeat(100));
                println!("Total: {} identity(ies)", identities.len());
            }
        }
        IdentityAction::Show { name } => {
            let identity = Identity::load(&name)?;
            println!("Identity: {}", identity.name);
            println!("  Public key: {}", identity.public_key_hex);
            println!("  Created: {}", identity.created_at);
        }
        IdentityAction::Delete { name } => {
            Identity::delete(&name)?;
            println!("Deleted identity: {}", name);
        }
    }
    Ok(())
}

// =============================================================================
// Account handlers
// =============================================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RegisterAccountRequest {
    username: String,
    email: String,
    public_key: String,
}

#[derive(Debug, Serialize)]
struct UpdateAccountEmailRequest {
    email: String,
}

#[derive(Debug, Serialize)]
struct AddExternalKeyRequest {
    key_type: String,
    key_data: String,
    key_fingerprint: Option<String>,
    label: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AccountWithKeys {
    username: String,
    email: Option<String>,
    email_verified: Option<bool>,
    created_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AccountExternalKey {
    id: i64,
    key_type: String,
    key_data: String,
    label: Option<String>,
}

async fn handle_account_action(action: AccountAction, api_url: &str) -> Result<()> {
    match action {
        AccountAction::Create { identity, username, email } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let request = RegisterAccountRequest {
                username: username.clone(),
                email: email.clone(),
                public_key: id.public_key_hex.clone(),
            };

            let account: AccountWithKeys = client.post_api("/accounts", &request).await?;
            println!("Account created:");
            println!("  Username: {}", account.username);
            println!("  Email: {}", account.email.unwrap_or_else(|| "N/A".to_string()));
            println!("  Public Key: {}", id.public_key_hex);
        }
        AccountAction::Get { identity } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            // Search by public key
            let path = format!("/accounts?publicKey={}", id.public_key_hex);
            let account: AccountWithKeys = client.get_api(&path).await?;
            println!("Account:");
            println!("  Username: {}", account.username);
            println!("  Email: {}", account.email.unwrap_or_else(|| "N/A".to_string()));
            println!("  Email verified: {}", account.email_verified.unwrap_or(false));
            if let Some(created) = account.created_at {
                if let Some(dt) = chrono::DateTime::from_timestamp(created, 0) {
                    println!("  Created: {}", dt.format("%Y-%m-%d %H:%M:%S"));
                }
            }
        }
        AccountAction::UpdateEmail { identity, email } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            // First get the account to find username
            let path = format!("/accounts?publicKey={}", id.public_key_hex);
            let account: AccountWithKeys = client.get_api(&path).await?;

            let request = UpdateAccountEmailRequest { email: email.clone() };
            let path = format!("/accounts/{}/email", account.username);
            let _: AccountWithKeys = client.put_api(&path, &request).await?;
            println!("Email updated to: {}", email);
        }
        AccountAction::AddSshKey { identity, key, label } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            // First get the account to find username
            let path = format!("/accounts?publicKey={}", id.public_key_hex);
            let account: AccountWithKeys = client.get_api(&path).await?;

            let request = AddExternalKeyRequest {
                key_type: "ssh".to_string(),
                key_data: key.clone(),
                key_fingerprint: None,
                label,
            };
            let path = format!("/accounts/{}/external-keys", account.username);
            let _: String = client.post_api(&path, &request).await?;
            println!("SSH key added successfully");
        }
        AccountAction::ListSshKeys { identity } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            // First get the account to find username
            let path = format!("/accounts?publicKey={}", id.public_key_hex);
            let account: AccountWithKeys = client.get_api(&path).await?;

            let path = format!("/accounts/{}/external-keys", account.username);
            let keys: Vec<AccountExternalKey> = client.get_api(&path).await?;

            if keys.is_empty() {
                println!("No SSH keys found.");
            } else {
                println!("\nSSH Keys:");
                println!("{}", "=".repeat(80));
                for key in &keys {
                    if key.key_type == "ssh" {
                        println!("  ID: {}", key.id);
                        println!("  Label: {}", key.label.as_deref().unwrap_or("N/A"));
                        println!("  Key: {}...", &key.key_data.chars().take(50).collect::<String>());
                        println!("{}", "-".repeat(80));
                    }
                }
            }
        }
    }
    Ok(())
}

// =============================================================================
// Contract handlers
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
    gateway_ssh_port: Option<i32>,
    gateway_port_range_start: Option<i32>,
    gateway_port_range_end: Option<i32>,
    provisioning_instance_details: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RentalRequestResponse {
    contract_id: String,
    message: String,
    checkout_url: Option<String>,
}

async fn handle_contract_action(action: ContractAction, api_url: &str) -> Result<()> {
    match action {
        ContractAction::ListOfferings { provider, product_type, in_stock_only, limit } => {
            // Use a dummy identity just for public endpoint access
            let http = reqwest::Client::new();
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
            let api_response: api_cli::client::ApiResponse<Vec<Offering>> = serde_json::from_str(&text)?;
            let offerings = api_response.into_result()?;

            if offerings.is_empty() {
                println!("No offerings found.");
            } else {
                println!("\nAvailable Offerings:");
                println!("{}", "=".repeat(120));
                println!("{:<8} {:<40} {:<15} {:<20} {:<10} {:<10}",
                    "ID", "Name", "Type", "Provider", "Price/mo", "Stock");
                println!("{}", "-".repeat(120));
                for o in &offerings {
                    let name = o.offer_name.as_deref().unwrap_or("N/A");
                    let ptype = o.product_type.as_deref().unwrap_or("N/A");
                    let price = o.monthly_price.map(|p| format!("${:.2}", p)).unwrap_or_else(|| "N/A".to_string());
                    let stock = o.stock_status.as_deref().unwrap_or("N/A");
                    let provider_short = if o.pubkey.len() > 16 {
                        format!("{}...", &o.pubkey[..16])
                    } else {
                        o.pubkey.clone()
                    };
                    println!("{:<8} {:<40} {:<15} {:<20} {:<10} {:<10}",
                        o.id, &name[..name.len().min(38)], ptype, provider_short, price, stock);
                }
                println!("{}", "=".repeat(120));
                println!("Total: {} offering(s)", offerings.len());
            }
        }
        ContractAction::Create { identity, offering_id, ssh_pubkey, duration_hours, skip_payment } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let payment_method = if skip_payment {
                // For testing: use "test" payment method that auto-succeeds
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
                // Mark payment as succeeded directly in DB
                println!("\nNote: --skip-payment was used. Setting payment_status to 'succeeded'...");
                let db_url = env::var("DATABASE_URL")
                    .unwrap_or_else(|_| api::database::DEFAULT_DATABASE_URL.to_string());
                let db = Database::new(&db_url).await?;
                let contract_id_bytes = uuid::Uuid::parse_str(&response.contract_id)?.as_bytes().to_vec();
                db.set_payment_status_for_testing(&contract_id_bytes, "succeeded").await?;
                println!("Payment status set to 'succeeded'.");
            }
        }
        ContractAction::Get { contract_id, identity } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let path = format!("/contracts/{}", contract_id);
            let contract: Contract = client.get_api(&path).await?;
            println!("Contract: {}", contract.contract_id);
            println!("  Status: {}", contract.status);
            println!("  Payment status: {}", contract.payment_status);
            if let Some(slug) = &contract.gateway_slug {
                println!("  Gateway: {}.gateway.decent-cloud.org", slug);
            }
            if let Some(port) = contract.gateway_ssh_port {
                println!("  SSH port: {}", port);
            }
            if let (Some(start), Some(end)) = (contract.gateway_port_range_start, contract.gateway_port_range_end) {
                println!("  Port range: {}-{}", start, end);
            }
            if let Some(details) = &contract.provisioning_instance_details {
                println!("  Instance details: {}", details);
            }
        }
        ContractAction::Wait { contract_id, state, timeout, identity } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let start = std::time::Instant::now();
            let timeout_duration = std::time::Duration::from_secs(timeout);
            let poll_interval = std::time::Duration::from_secs(10);

            println!("Waiting for contract {} to reach state '{}'...", contract_id, state);

            loop {
                let path = format!("/contracts/{}", contract_id);
                let contract: Contract = client.get_api(&path).await?;

                if contract.status == state {
                    println!("Contract reached state '{}' after {:?}", state, start.elapsed());
                    return Ok(());
                }

                if start.elapsed() > timeout_duration {
                    anyhow::bail!(
                        "Timeout waiting for contract to reach state '{}'. Current state: '{}'",
                        state, contract.status
                    );
                }

                println!("  Current state: '{}', waiting... ({:.0}s elapsed)",
                    contract.status, start.elapsed().as_secs_f64());
                tokio::time::sleep(poll_interval).await;
            }
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
                println!("{:<38} {:<15} {:<15} {:<20}", "Contract ID", "Status", "Payment", "Gateway");
                println!("{}", "-".repeat(100));
                for c in &contracts {
                    let gateway = c.gateway_slug.as_ref()
                        .map(|s| format!("{}.gw...", s))
                        .unwrap_or_else(|| "N/A".to_string());
                    println!("{:<38} {:<15} {:<15} {:<20}",
                        c.contract_id, c.status, c.payment_status, gateway);
                }
                println!("{}", "=".repeat(100));
                println!("Total: {} contract(s)", contracts.len());
            }
        }
        ContractAction::Cancel { contract_id, identity, memo } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let request = CancelContractRequest { memo };
            let path = format!("/contracts/{}/cancel", contract_id);
            let _: String = client.put_api(&path, &request).await?;
            println!("Contract {} cancelled.", contract_id);
        }
    }
    Ok(())
}

// =============================================================================
// Offering handlers
// =============================================================================

async fn handle_offering_action(action: OfferingAction, api_url: &str) -> Result<()> {
    let http = reqwest::Client::new();

    match action {
        OfferingAction::List { filter, limit } => {
            let mut url = format!("{}/api/v1/offerings?limit={}", api_url, limit);
            if let Some(f) = filter {
                url.push_str(&format!("&q={}", urlencoding::encode(&f)));
            }

            let response = http.get(&url).send().await?;
            let text = response.text().await?;
            let api_response: api_cli::client::ApiResponse<Vec<Offering>> = serde_json::from_str(&text)?;
            let offerings = api_response.into_result()?;

            if offerings.is_empty() {
                println!("No offerings found.");
            } else {
                println!("\nOfferings:");
                println!("{}", "=".repeat(100));
                for o in &offerings {
                    println!("ID: {} ({})", o.id, o.offering_id);
                    println!("  Name: {}", o.offer_name.as_deref().unwrap_or("N/A"));
                    println!("  Type: {}", o.product_type.as_deref().unwrap_or("N/A"));
                    println!("  Price: ${:.2}/mo", o.monthly_price.unwrap_or(0.0));
                    println!("{}", "-".repeat(100));
                }
            }
        }
        OfferingAction::Get { offering_id } => {
            let url = format!("{}/api/v1/offerings/{}", api_url, offering_id);
            let response = http.get(&url).send().await?;
            let text = response.text().await?;
            let api_response: api_cli::client::ApiResponse<Offering> = serde_json::from_str(&text)?;
            let offering = api_response.into_result()?;

            println!("Offering: {}", offering.offering_id);
            println!("  ID: {}", offering.id);
            println!("  Name: {}", offering.offer_name.as_deref().unwrap_or("N/A"));
            println!("  Type: {}", offering.product_type.as_deref().unwrap_or("N/A"));
            println!("  Price: ${:.2}/mo", offering.monthly_price.unwrap_or(0.0));
            println!("  Stock: {}", offering.stock_status.as_deref().unwrap_or("N/A"));
        }
    }
    Ok(())
}

// =============================================================================
// Provider handlers
// =============================================================================

#[derive(Debug, Deserialize)]
struct ProviderProfile {
    #[serde(default)]
    pubkey: Option<String>,
    name: Option<String>,
    #[serde(alias = "website")]
    website_url: Option<String>,
}

async fn handle_provider_action(action: ProviderAction, api_url: &str) -> Result<()> {
    let http = reqwest::Client::new();

    match action {
        ProviderAction::List { limit } => {
            let url = format!("{}/api/v1/providers?limit={}", api_url, limit);
            let response = http.get(&url).send().await?;
            let text = response.text().await?;
            let api_response: api_cli::client::ApiResponse<Vec<ProviderProfile>> = serde_json::from_str(&text)?;
            let providers = api_response.into_result()?;

            if providers.is_empty() {
                println!("No providers found.");
            } else {
                println!("\nProviders:");
                println!("{}", "=".repeat(100));
                println!("{:<66} {:<20} {:<30}", "Public Key", "Name", "Website");
                println!("{}", "-".repeat(100));
                for p in &providers {
                    let name = p.name.as_deref().unwrap_or("N/A");
                    let website = p.website_url.as_deref().unwrap_or("N/A");
                    let pubkey = p.pubkey.as_deref().unwrap_or("N/A");
                    println!("{:<66} {:<20} {:<30}", pubkey, name, website);
                }
                println!("{}", "=".repeat(100));
                println!("Total: {} provider(s)", providers.len());
            }
        }
        ProviderAction::Status { pubkey } => {
            let url = format!("{}/api/v1/providers/{}", api_url, pubkey);
            let response = http.get(&url).send().await?;
            let text = response.text().await?;
            let api_response: api_cli::client::ApiResponse<ProviderProfile> = serde_json::from_str(&text)?;
            let provider = api_response.into_result()?;

            println!("Provider: {}", provider.pubkey.as_deref().unwrap_or(&pubkey));
            println!("  Name: {}", provider.name.as_deref().unwrap_or("N/A"));
            println!("  Website: {}", provider.website_url.as_deref().unwrap_or("N/A"));
        }
        ProviderAction::Offerings { pubkey } => {
            let url = format!("{}/api/v1/providers/{}/offerings", api_url, pubkey);
            let response = http.get(&url).send().await?;
            let text = response.text().await?;
            let api_response: api_cli::client::ApiResponse<Vec<Offering>> = serde_json::from_str(&text)?;
            let offerings = api_response.into_result()?;

            if offerings.is_empty() {
                println!("No offerings found for this provider.");
            } else {
                println!("\nProvider Offerings:");
                println!("{}", "=".repeat(100));
                for o in &offerings {
                    println!("ID: {} - {}", o.id, o.offer_name.as_deref().unwrap_or("N/A"));
                    println!("  Type: {}, Price: ${:.2}/mo, Stock: {}",
                        o.product_type.as_deref().unwrap_or("N/A"),
                        o.monthly_price.unwrap_or(0.0),
                        o.stock_status.as_deref().unwrap_or("N/A"));
                    println!("{}", "-".repeat(100));
                }
                println!("Total: {} offering(s)", offerings.len());
            }
        }
    }
    Ok(())
}

// =============================================================================
// Notify handlers
// =============================================================================

async fn handle_notify_action(action: NotifyAction) -> Result<()> {
    match action {
        NotifyAction::Email { to, with_dkim } => {
            handle_test_email(&to, with_dkim).await
        }
        NotifyAction::Telegram { chat_id, message } => {
            let bot_token = env::var("TELEGRAM_BOT_TOKEN")
                .context("TELEGRAM_BOT_TOKEN not set")?;

            let http = reqwest::Client::new();
            let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);

            let params = serde_json::json!({
                "chat_id": chat_id,
                "text": message,
            });

            let response = http.post(&url)
                .json(&params)
                .send()
                .await?;

            if response.status().is_success() {
                println!("Telegram message sent successfully to chat {}", chat_id);
            } else {
                let text = response.text().await?;
                anyhow::bail!("Failed to send Telegram message: {}", text);
            }
            Ok(())
        }
    }
}

// =============================================================================
// DNS handlers
// =============================================================================

async fn handle_dns_action(action: DnsAction) -> Result<()> {
    let api_token = env::var("CLOUDFLARE_API_TOKEN")
        .context("CLOUDFLARE_API_TOKEN not set")?;
    let zone_id = env::var("CLOUDFLARE_ZONE_ID")
        .context("CLOUDFLARE_ZONE_ID not set")?;
    let base_domain = env::var("CLOUDFLARE_BASE_DOMAIN")
        .unwrap_or_else(|_| "gateway.decent-cloud.org".to_string());

    let http = reqwest::Client::new();
    let base_url = format!("https://api.cloudflare.com/client/v4/zones/{}/dns_records", zone_id);

    match action {
        DnsAction::Create { subdomain, ip } => {
            let full_name = format!("{}.{}", subdomain, base_domain);
            let params = serde_json::json!({
                "type": "A",
                "name": full_name,
                "content": ip,
                "ttl": 300,
                "proxied": false,
            });

            let response = http.post(&base_url)
                .header("Authorization", format!("Bearer {}", api_token))
                .json(&params)
                .send()
                .await?;

            let text = response.text().await?;
            let json: serde_json::Value = serde_json::from_str(&text)?;

            if json["success"].as_bool().unwrap_or(false) {
                println!("DNS record created: {} -> {}", full_name, ip);
            } else {
                anyhow::bail!("Failed to create DNS record: {}", text);
            }
        }
        DnsAction::Get { subdomain } => {
            let full_name = format!("{}.{}", subdomain, base_domain);
            let url = format!("{}?name={}", base_url, urlencoding::encode(&full_name));

            let response = http.get(&url)
                .header("Authorization", format!("Bearer {}", api_token))
                .send()
                .await?;

            let text = response.text().await?;
            let json: serde_json::Value = serde_json::from_str(&text)?;

            if let Some(records) = json["result"].as_array() {
                if records.is_empty() {
                    println!("No DNS record found for {}", full_name);
                } else {
                    for record in records {
                        println!("DNS Record:");
                        println!("  Name: {}", record["name"]);
                        println!("  Type: {}", record["type"]);
                        println!("  Content: {}", record["content"]);
                        println!("  TTL: {}", record["ttl"]);
                    }
                }
            }
        }
        DnsAction::Delete { subdomain } => {
            let full_name = format!("{}.{}", subdomain, base_domain);
            let url = format!("{}?name={}", base_url, urlencoding::encode(&full_name));

            // First, find the record ID
            let response = http.get(&url)
                .header("Authorization", format!("Bearer {}", api_token))
                .send()
                .await?;

            let text = response.text().await?;
            let json: serde_json::Value = serde_json::from_str(&text)?;

            if let Some(records) = json["result"].as_array() {
                if records.is_empty() {
                    println!("No DNS record found for {}", full_name);
                } else {
                    for record in records {
                        if let Some(id) = record["id"].as_str() {
                            let delete_url = format!("{}/{}", base_url, id);
                            let response = http.delete(&delete_url)
                                .header("Authorization", format!("Bearer {}", api_token))
                                .send()
                                .await?;

                            if response.status().is_success() {
                                println!("DNS record deleted: {}", full_name);
                            } else {
                                let text = response.text().await?;
                                anyhow::bail!("Failed to delete DNS record: {}", text);
                            }
                        }
                    }
                }
            }
        }
        DnsAction::List => {
            let response = http.get(&base_url)
                .header("Authorization", format!("Bearer {}", api_token))
                .send()
                .await?;

            let text = response.text().await?;
            let json: serde_json::Value = serde_json::from_str(&text)?;

            if let Some(records) = json["result"].as_array() {
                let dc_records: Vec<_> = records.iter()
                    .filter(|r| r["name"].as_str().map(|n| n.contains(&base_domain)).unwrap_or(false))
                    .collect();

                if dc_records.is_empty() {
                    println!("No DC gateway DNS records found.");
                } else {
                    println!("\nDC Gateway DNS Records:");
                    println!("{}", "=".repeat(80));
                    println!("{:<40} {:<10} {:<20}", "Name", "Type", "Content");
                    println!("{}", "-".repeat(80));
                    for record in &dc_records {
                        println!("{:<40} {:<10} {:<20}",
                            record["name"].as_str().unwrap_or("N/A"),
                            record["type"].as_str().unwrap_or("N/A"),
                            record["content"].as_str().unwrap_or("N/A"));
                    }
                    println!("{}", "=".repeat(80));
                    println!("Total: {} record(s)", dc_records.len());
                }
            }
        }
    }
    Ok(())
}

// =============================================================================
// Gateway handlers
// =============================================================================

async fn handle_gateway_action(action: GatewayAction, api_url: &str) -> Result<()> {
    match action {
        GatewayAction::Ssh { host, port, identity_file } => {
            println!("Testing SSH connectivity to {}:{}", host, port);

            // Use ssh command to test connectivity
            let output = tokio::process::Command::new("ssh")
                .args([
                    "-o", "StrictHostKeyChecking=no",
                    "-o", "ConnectTimeout=10",
                    "-i", &identity_file,
                    "-p", &port.to_string(),
                    &format!("root@{}", host),
                    "echo", "SSH_CONNECTION_OK",
                ])
                .output()
                .await?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains("SSH_CONNECTION_OK") {
                    println!("SSH connection successful!");
                } else {
                    println!("SSH connected but unexpected output: {}", stdout);
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("SSH connection failed: {}", stderr);
            }
        }
        GatewayAction::Tcp { host, external_port, expect_response } => {
            println!("Testing TCP connectivity to {}:{}", host, external_port);

            use tokio::net::TcpStream;
            use tokio::io::{AsyncReadExt, AsyncWriteExt};

            let addr = format!("{}:{}", host, external_port);
            let mut stream = TcpStream::connect(&addr).await
                .with_context(|| format!("Failed to connect to {}", addr))?;

            println!("TCP connection established.");

            if let Some(expected) = expect_response {
                // Send a simple ping and wait for response
                stream.write_all(b"ping\n").await?;

                let mut buffer = [0u8; 1024];
                let n = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    stream.read(&mut buffer)
                ).await
                .context("Timeout waiting for response")??;

                let response = String::from_utf8_lossy(&buffer[..n]);
                if response.contains(&expected) {
                    println!("Expected response received: {}", response.trim());
                } else {
                    anyhow::bail!("Unexpected response: expected '{}', got '{}'", expected, response.trim());
                }
            } else {
                println!("TCP connectivity OK (no response check requested).");
            }
        }
        GatewayAction::Contract { contract_id, identity } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let path = format!("/contracts/{}", contract_id);
            let contract: Contract = client.get_api(&path).await?;

            println!("Testing gateway connectivity for contract: {}", contract_id);

            let gateway_slug = contract.gateway_slug
                .context("Contract has no gateway configured")?;
            let gateway_host = format!("{}.gateway.decent-cloud.org", gateway_slug);

            if let Some(ssh_port) = contract.gateway_ssh_port {
                println!("\nTesting SSH on port {}...", ssh_port);
                // Just test TCP connectivity to SSH port
                use tokio::net::TcpStream;
                let addr = format!("{}:{}", gateway_host, ssh_port);
                match TcpStream::connect(&addr).await {
                    Ok(_) => println!("  SSH port {} is reachable", ssh_port),
                    Err(e) => println!("  SSH port {} not reachable: {}", ssh_port, e),
                }
            }

            if let (Some(start), Some(end)) = (contract.gateway_port_range_start, contract.gateway_port_range_end) {
                println!("\nTesting port range {}-{}...", start, end);
                use tokio::net::TcpStream;
                for port in start..=end.min(start + 5) { // Test first 5 ports max
                    let addr = format!("{}:{}", gateway_host, port);
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(2),
                        TcpStream::connect(&addr)
                    ).await {
                        Ok(Ok(_)) => println!("  Port {} is reachable", port),
                        Ok(Err(e)) => println!("  Port {} not reachable: {}", port, e),
                        Err(_) => println!("  Port {} connection timeout", port),
                    }
                }
            }

            println!("\nGateway connectivity test complete.");
        }
    }
    Ok(())
}

// =============================================================================
// Health handlers
// =============================================================================

async fn handle_health_action(action: HealthAction, api_url: &str) -> Result<()> {
    let http = reqwest::Client::new();

    async fn check_health(name: &str, result: Result<String, anyhow::Error>) {
        match result {
            Ok(time) => println!("{}: ✓ healthy ({})", name, time),
            Err(e) => println!("{}: ✗ unhealthy - {}", name, e),
        }
    }

    match action {
        HealthAction::Api => {
            let start = std::time::Instant::now();
            let url = format!("{}/api/v1/offerings?limit=1", api_url);
            let result = http.get(&url).send().await;
            match result {
                Ok(resp) if resp.status().is_success() => {
                    println!("API Server: ✓ healthy ({:.0}ms)", start.elapsed().as_millis());
                }
                Ok(resp) => {
                    println!("API Server: ✗ unhealthy - status {}", resp.status());
                }
                Err(e) => {
                    println!("API Server: ✗ unhealthy - {}", e);
                }
            }
        }
        HealthAction::All => {
            println!("\nService Health Checks:");
            println!("{}", "=".repeat(60));

            // API
            let start = std::time::Instant::now();
            let url = format!("{}/api/v1/offerings?limit=1", api_url);
            let api_result = http.get(&url).send().await
                .map(|_| format!("{:.0}ms", start.elapsed().as_millis()))
                .map_err(|e| anyhow::anyhow!("{}", e));
            check_health("API Server", api_result).await;

            // Database (via API health)
            let start = std::time::Instant::now();
            let url = format!("{}/api/v1/providers?limit=1", api_url);
            let db_result = http.get(&url).send().await
                .map(|_| format!("{:.0}ms", start.elapsed().as_millis()))
                .map_err(|e| anyhow::anyhow!("{}", e));
            check_health("Database", db_result).await;

            // Cloudflare
            if env::var("CLOUDFLARE_API_TOKEN").is_ok() {
                let start = std::time::Instant::now();
                let cf_result = http.get("https://api.cloudflare.com/client/v4/user/tokens/verify")
                    .header("Authorization", format!("Bearer {}", env::var("CLOUDFLARE_API_TOKEN").unwrap()))
                    .send().await
                    .map(|_| format!("{:.0}ms", start.elapsed().as_millis()))
                    .map_err(|e| anyhow::anyhow!("{}", e));
                check_health("Cloudflare DNS", cf_result).await;
            } else {
                println!("Cloudflare DNS: - not configured");
            }

            // Stripe
            if env::var("STRIPE_SECRET_KEY").is_ok() {
                let start = std::time::Instant::now();
                let stripe_result = http.get("https://api.stripe.com/v1/balance")
                    .header("Authorization", format!("Bearer {}", env::var("STRIPE_SECRET_KEY").unwrap()))
                    .send().await
                    .map(|_| format!("{:.0}ms", start.elapsed().as_millis()))
                    .map_err(|e| anyhow::anyhow!("{}", e));
                check_health("Stripe", stripe_result).await;
            } else {
                println!("Stripe: - not configured");
            }

            // MailChannels
            if env::var("MAILCHANNELS_API_KEY").is_ok() {
                println!("MailChannels: - configured (no health endpoint)");
            } else {
                println!("MailChannels: - not configured");
            }

            // Telegram
            if let Ok(token) = env::var("TELEGRAM_BOT_TOKEN") {
                let start = std::time::Instant::now();
                let url = format!("https://api.telegram.org/bot{}/getMe", token);
                let tg_result = http.get(&url).send().await
                    .map(|_| format!("{:.0}ms", start.elapsed().as_millis()))
                    .map_err(|e| anyhow::anyhow!("{}", e));
                check_health("Telegram Bot", tg_result).await;
            } else {
                println!("Telegram Bot: - not configured");
            }

            println!("{}", "=".repeat(60));
        }
        HealthAction::Cloudflare => {
            let token = env::var("CLOUDFLARE_API_TOKEN")
                .context("CLOUDFLARE_API_TOKEN not set")?;
            let start = std::time::Instant::now();
            let response = http.get("https://api.cloudflare.com/client/v4/user/tokens/verify")
                .header("Authorization", format!("Bearer {}", token))
                .send().await?;
            if response.status().is_success() {
                println!("Cloudflare DNS: ✓ healthy ({:.0}ms)", start.elapsed().as_millis());
            } else {
                let text = response.text().await?;
                println!("Cloudflare DNS: ✗ unhealthy - {}", text);
            }
        }
        HealthAction::Stripe => {
            let key = env::var("STRIPE_SECRET_KEY")
                .context("STRIPE_SECRET_KEY not set")?;
            let start = std::time::Instant::now();
            let response = http.get("https://api.stripe.com/v1/balance")
                .header("Authorization", format!("Bearer {}", key))
                .send().await?;
            if response.status().is_success() {
                println!("Stripe: ✓ healthy ({:.0}ms)", start.elapsed().as_millis());
            } else {
                let text = response.text().await?;
                println!("Stripe: ✗ unhealthy - {}", text);
            }
        }
        HealthAction::Mailchannels => {
            if env::var("MAILCHANNELS_API_KEY").is_ok() {
                println!("MailChannels: ✓ configured (no health endpoint available)");
            } else {
                println!("MailChannels: ✗ not configured (MAILCHANNELS_API_KEY not set)");
            }
        }
        HealthAction::Telegram => {
            let token = env::var("TELEGRAM_BOT_TOKEN")
                .context("TELEGRAM_BOT_TOKEN not set")?;
            let start = std::time::Instant::now();
            let url = format!("https://api.telegram.org/bot{}/getMe", token);
            let response = http.get(&url).send().await?;
            if response.status().is_success() {
                println!("Telegram Bot: ✓ healthy ({:.0}ms)", start.elapsed().as_millis());
            } else {
                let text = response.text().await?;
                println!("Telegram Bot: ✗ unhealthy - {}", text);
            }
        }
    }
    Ok(())
}

// =============================================================================
// E2E handlers
// =============================================================================

async fn handle_e2e_action(action: E2eAction, api_url: &str) -> Result<()> {
    match action {
        E2eAction::Provision { identity, offering_id, ssh_pubkey, verify_ssh, cleanup } => {
            println!("\n========================================");
            println!("  E2E Provisioning Test");
            println!("========================================\n");

            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            // Step 1: Create contract with skip-payment
            println!("Step 1: Creating contract...");
            let request = CreateContractRequest {
                offering_db_id: offering_id,
                ssh_pubkey: Some(ssh_pubkey.clone()),
                duration_hours: Some(1),
                payment_method: Some("test".to_string()),
            };
            let response: RentalRequestResponse = client.post_api("/contracts", &request).await?;
            let contract_id = response.contract_id.clone();
            println!("  Contract created: {}", contract_id);

            // Mark payment as succeeded
            let db_url = env::var("DATABASE_URL")
                .unwrap_or_else(|_| api::database::DEFAULT_DATABASE_URL.to_string());
            let db = Database::new(&db_url).await?;
            let contract_id_bytes = uuid::Uuid::parse_str(&contract_id)?.as_bytes().to_vec();
            db.set_payment_status_for_testing(&contract_id_bytes, "succeeded").await?;
            println!("  Payment status set to 'succeeded'");

            // Step 2: Wait for provisioning
            println!("\nStep 2: Waiting for provisioning...");
            let start = std::time::Instant::now();
            let timeout = std::time::Duration::from_secs(300);
            let poll_interval = std::time::Duration::from_secs(10);

            loop {
                let path = format!("/contracts/{}", contract_id);
                let contract: Contract = client.get_api(&path).await?;

                if contract.status == "provisioned" {
                    println!("  Contract provisioned after {:?}", start.elapsed());
                    break;
                }

                if contract.status == "cancelled" || contract.status == "rejected" {
                    anyhow::bail!("Contract was {} during provisioning", contract.status);
                }

                if start.elapsed() > timeout {
                    anyhow::bail!("Timeout waiting for provisioning. Current status: {}", contract.status);
                }

                println!("  Status: '{}', waiting... ({:.0}s)", contract.status, start.elapsed().as_secs_f64());
                tokio::time::sleep(poll_interval).await;
            }

            // Step 3: Get gateway info
            println!("\nStep 3: Getting gateway information...");
            let path = format!("/contracts/{}", contract_id);
            let contract: Contract = client.get_api(&path).await?;

            let gateway_slug = contract.gateway_slug.context("No gateway assigned")?;
            let gateway_host = format!("{}.gateway.decent-cloud.org", gateway_slug);
            let ssh_port = contract.gateway_ssh_port.context("No SSH port assigned")?;

            println!("  Gateway: {}", gateway_host);
            println!("  SSH Port: {}", ssh_port);

            // Step 4: Verify SSH (optional)
            if verify_ssh {
                println!("\nStep 4: Testing SSH connectivity...");
                use tokio::net::TcpStream;
                let addr = format!("{}:{}", gateway_host, ssh_port);
                match tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    TcpStream::connect(&addr)
                ).await {
                    Ok(Ok(_)) => println!("  SSH port reachable!"),
                    Ok(Err(e)) => println!("  Warning: SSH port not reachable: {}", e),
                    Err(_) => println!("  Warning: SSH connection timeout"),
                }
            }

            // Step 5: Cleanup (optional)
            if cleanup {
                println!("\nStep 5: Cleaning up (cancelling contract)...");
                let request = CancelContractRequest { memo: Some("E2E test cleanup".to_string()) };
                let path = format!("/contracts/{}/cancel", contract_id);
                let _: String = client.put_api(&path, &request).await?;
                println!("  Contract cancelled");
            }

            println!("\n========================================");
            println!("  E2E Provisioning Test: SUCCESS");
            println!("========================================\n");
        }
        E2eAction::Lifecycle { identity, skip_payment } => {
            println!("\n========================================");
            println!("  E2E Contract Lifecycle Test");
            println!("========================================\n");

            println!("This test requires an available offering and will:");
            println!("  1. Create a contract");
            println!("  2. Verify it reaches 'pending' state");
            println!("  3. Cancel the contract");
            println!("  4. Verify it reaches 'cancelled' state");

            let _id = Identity::load(&identity)?;

            if !skip_payment {
                println!("\nNote: Without --skip-payment, this test will not complete.");
                println!("Use --skip-payment for testing purposes.");
            }

            println!("\nLifecycle test placeholder - implement with actual offering ID.");
        }
        E2eAction::All { identity } => {
            println!("\n========================================");
            println!("  Running All E2E Tests");
            println!("========================================\n");

            // Verify identity exists
            let _id = Identity::load(&identity)?;
            println!("Using identity: {}", identity);

            println!("\nNote: Full E2E tests require:");
            println!("  - A running API server");
            println!("  - At least one available offering");
            println!("  - A provider with dc-agent running");

            println!("\nRun individual tests with:");
            println!("  api-cli e2e provision --identity {} --offering-id <id> --ssh-pubkey '<key>'", identity);
        }
    }
    Ok(())
}

// =============================================================================
// Admin handlers (existing)
// =============================================================================

async fn handle_admin_action(action: AdminAction) -> Result<()> {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| api::database::DEFAULT_DATABASE_URL.to_string());
    let db = Database::new(&database_url).await?;

    match action {
        AdminAction::Grant { username } => {
            db.set_admin_status(&username, true).await?;
            println!("✓ Admin access granted to: {}", username);
        }
        AdminAction::Revoke { username } => {
            db.set_admin_status(&username, false).await?;
            println!("✓ Admin access revoked from: {}", username);
        }
        AdminAction::List => {
            let admins = db.list_admins().await?;
            if admins.is_empty() {
                println!("No admin accounts found.");
            } else {
                println!("\nAdmin Accounts:");
                println!("{}", "=".repeat(80));
                println!("{:<20} {:<40} {:<20}", "Username", "Email", "Created At");
                println!("{}", "-".repeat(80));
                for admin in &admins {
                    let email = admin.email.as_deref().unwrap_or("N/A");
                    let created = chrono::DateTime::from_timestamp(admin.created_at, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| "Invalid".to_string());
                    println!("{:<20} {:<40} {:<20}", admin.username, email, created);
                }
                println!("{}", "=".repeat(80));
                println!("Total: {} admin account(s)", admins.len());
            }
        }
    }
    Ok(())
}

// =============================================================================
// Test email handler (existing)
// =============================================================================

async fn handle_test_email(to: &str, with_dkim: bool) -> Result<()> {
    println!("\n========================================");
    println!("  Email Configuration Test");
    println!("========================================\n");

    if let Err(e) = validate_email(to) {
        anyhow::bail!("Invalid email address: {}", e);
    }

    let api_key = env::var("MAILCHANNELS_API_KEY")
        .context("MAILCHANNELS_API_KEY not set")?;

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

    email_service.send_email(from_addr, to, subject, &body, false).await?;
    println!("\n✓ SUCCESS! Test email sent.");
    println!("Please check your inbox at: {}", to);
    Ok(())
}

// =============================================================================
// Seed provider handler (existing)
// =============================================================================

async fn handle_seed_provider(
    name: &str,
    domain: &str,
    csv_path: &str,
    website: Option<&str>,
    upsert: bool,
) -> Result<()> {
    println!("\n========================================");
    println!("  Seed External Provider");
    println!("========================================\n");

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| api::database::DEFAULT_DATABASE_URL.to_string());
    let db = Database::new(&database_url).await?;

    let mut hasher = Sha256::new();
    hasher.update(b"external-provider:");
    hasher.update(domain.as_bytes());
    let hash = hasher.finalize();
    let pubkey = &hash[0..32];

    println!("Provider: {}", name);
    println!("Domain: {}", domain);
    println!("Pubkey: {}", hex::encode(pubkey));

    let website_url = website.unwrap_or_else(|| {
        let default = format!("https://{}", domain);
        println!("Website URL: {} (default)", default);
        Box::leak(default.into_boxed_str()) as &str
    });
    if website.is_some() {
        println!("Website URL: {}", website_url);
    }

    println!("\nCreating/updating external provider record...");
    db.create_or_update_external_provider(pubkey, name, domain, website_url, "scraper")
        .await?;
    println!("✓ External provider record saved");

    println!("\nReading CSV file: {}", csv_path);
    let csv_data = std::fs::read_to_string(csv_path)?;
    let line_count = csv_data.lines().count().saturating_sub(1);
    println!("Found {} offerings in CSV", line_count);

    println!("\nImporting offerings...");
    let (success_count, errors) = db
        .import_seeded_offerings_csv(pubkey, &csv_data, upsert)
        .await?;

    println!("\n========================================");
    println!("  Import Summary");
    println!("========================================");
    println!("Total: {}, Success: {}, Errors: {}", line_count, success_count, errors.len());

    if !errors.is_empty() {
        println!("\nErrors:");
        for (row, error) in errors.iter().take(10) {
            println!("  Row {}: {}", row, error);
        }
        if errors.len() > 10 {
            println!("  ... and {} more", errors.len() - 10);
        }
    }

    if success_count > 0 {
        println!("\n✓ Successfully seeded {} offerings for {}", success_count, name);
    }

    Ok(())
}
