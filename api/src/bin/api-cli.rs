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
    /// Cloud account and resource management (Hetzner, Proxmox)
    Cloud {
        #[command(subcommand)]
        action: CloudAction,
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
// Cloud subcommands
// =============================================================================

#[derive(Subcommand)]
enum CloudAction {
    /// List cloud accounts
    ListAccounts {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
    },
    /// Add a cloud account
    AddAccount {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Backend type (hetzner or proxmox_api)
        #[arg(long)]
        backend: String,
        /// Display name for the account
        #[arg(long)]
        name: String,
        /// Credentials (API token for Hetzner, JSON config for Proxmox)
        #[arg(long)]
        credentials: String,
    },
    /// Delete a cloud account
    DeleteAccount {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Cloud account ID (UUID)
        #[arg(long)]
        id: String,
    },
    /// Show available server types, locations, and images
    Catalog {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Cloud account ID (UUID)
        #[arg(long)]
        account_id: String,
    },
    /// List cloud resources
    ListResources {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
    },
    /// Provision a new cloud resource (VM)
    Provision {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Cloud account ID (UUID)
        #[arg(long)]
        account_id: String,
        /// VM name
        #[arg(long)]
        name: String,
        /// Server type (e.g., cx22)
        #[arg(long)]
        server_type: String,
        /// Location (e.g., fsn1)
        #[arg(long)]
        location: String,
        /// OS image (e.g., ubuntu-24.04)
        #[arg(long)]
        image: String,
        /// SSH public key for VM access
        #[arg(long)]
        ssh_pubkey: String,
    },
    /// Delete a cloud resource
    DeleteResource {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Cloud resource ID (UUID)
        #[arg(long)]
        id: String,
    },
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
                    println!(
                        "{:<20} {:<66} {:<20}",
                        id.name,
                        id.public_key_hex,
                        &id.created_at[..19]
                    );
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
        AccountAction::Create {
            identity,
            username,
            email,
        } => {
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
            println!(
                "  Email: {}",
                account.email.unwrap_or_else(|| "N/A".to_string())
            );
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
            println!(
                "  Email: {}",
                account.email.unwrap_or_else(|| "N/A".to_string())
            );
            println!(
                "  Email verified: {}",
                account.email_verified.unwrap_or(false)
            );
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

            let request = UpdateAccountEmailRequest {
                email: email.clone(),
            };
            let path = format!("/accounts/{}/email", account.username);
            let _: AccountWithKeys = client.put_api(&path, &request).await?;
            println!("Email updated to: {}", email);
        }
        AccountAction::AddSshKey {
            identity,
            key,
            label,
        } => {
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
                        println!(
                            "  Key: {}...",
                            &key.key_data.chars().take(50).collect::<String>()
                        );
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
    #[serde(default)]
    is_example: bool,
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

async fn handle_contract_action(action: ContractAction, api_url: &str) -> Result<()> {
    match action {
        ContractAction::ListOfferings {
            provider,
            product_type,
            in_stock_only,
            limit,
        } => {
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
                // For testing: use "icpay" payment method that auto-succeeds without checkout
                Some("icpay".to_string())
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
                println!("\nNote: --skip-payment used icpay method (payment auto-succeeds).");
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
            let api_response: api_cli::client::ApiResponse<Vec<Offering>> =
                serde_json::from_str(&text)?;
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
            println!(
                "  Name: {}",
                offering.offer_name.as_deref().unwrap_or("N/A")
            );
            println!(
                "  Type: {}",
                offering.product_type.as_deref().unwrap_or("N/A")
            );
            println!("  Price: ${:.2}/mo", offering.monthly_price.unwrap_or(0.0));
            println!(
                "  Stock: {}",
                offering.stock_status.as_deref().unwrap_or("N/A")
            );
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
            let api_response: api_cli::client::ApiResponse<Vec<ProviderProfile>> =
                serde_json::from_str(&text)?;
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
            let api_response: api_cli::client::ApiResponse<ProviderProfile> =
                serde_json::from_str(&text)?;
            let provider = api_response.into_result()?;

            println!(
                "Provider: {}",
                provider.pubkey.as_deref().unwrap_or(&pubkey)
            );
            println!("  Name: {}", provider.name.as_deref().unwrap_or("N/A"));
            println!(
                "  Website: {}",
                provider.website_url.as_deref().unwrap_or("N/A")
            );
        }
        ProviderAction::Offerings { pubkey } => {
            let url = format!("{}/api/v1/providers/{}/offerings", api_url, pubkey);
            let response = http.get(&url).send().await?;
            let text = response.text().await?;
            let api_response: api_cli::client::ApiResponse<Vec<Offering>> =
                serde_json::from_str(&text)?;
            let offerings = api_response.into_result()?;

            if offerings.is_empty() {
                println!("No offerings found for this provider.");
            } else {
                println!("\nProvider Offerings:");
                println!("{}", "=".repeat(100));
                for o in &offerings {
                    println!(
                        "ID: {} - {}",
                        o.id,
                        o.offer_name.as_deref().unwrap_or("N/A")
                    );
                    println!(
                        "  Type: {}, Price: ${:.2}/mo, Stock: {}",
                        o.product_type.as_deref().unwrap_or("N/A"),
                        o.monthly_price.unwrap_or(0.0),
                        o.stock_status.as_deref().unwrap_or("N/A")
                    );
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
        NotifyAction::Email { to, with_dkim } => handle_test_email(&to, with_dkim).await,
        NotifyAction::Telegram { chat_id, message } => {
            let bot_token = env::var("TELEGRAM_BOT_TOKEN").context("TELEGRAM_BOT_TOKEN not set")?;

            let http = reqwest::Client::new();
            let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);

            let params = serde_json::json!({
                "chat_id": chat_id,
                "text": message,
            });

            let response = http.post(&url).json(&params).send().await?;

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
    let api_token = env::var("CLOUDFLARE_API_TOKEN").context("CLOUDFLARE_API_TOKEN not set")?;
    let zone_id = env::var("CLOUDFLARE_ZONE_ID").context("CLOUDFLARE_ZONE_ID not set")?;
    let gw_prefix = env::var("CF_GW_PREFIX").unwrap_or_else(|_| "gw".to_string());
    let domain = env::var("CF_DOMAIN").unwrap_or_else(|_| "decent-cloud.org".to_string());
    let base_domain = format!("{}.{}", gw_prefix, domain);

    let http = reqwest::Client::new();
    let base_url = format!(
        "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
        zone_id
    );

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

            let response = http
                .post(&base_url)
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

            let response = http
                .get(&url)
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
            let response = http
                .get(&url)
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
                            let response = http
                                .delete(&delete_url)
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
            let response = http
                .get(&base_url)
                .header("Authorization", format!("Bearer {}", api_token))
                .send()
                .await?;

            let text = response.text().await?;
            let json: serde_json::Value = serde_json::from_str(&text)?;

            if let Some(records) = json["result"].as_array() {
                let dc_records: Vec<_> = records
                    .iter()
                    .filter(|r| {
                        r["name"]
                            .as_str()
                            .map(|n| n.contains(&base_domain))
                            .unwrap_or(false)
                    })
                    .collect();

                if dc_records.is_empty() {
                    println!("No DC gateway DNS records found.");
                } else {
                    println!("\nDC Gateway DNS Records:");
                    println!("{}", "=".repeat(80));
                    println!("{:<40} {:<10} {:<20}", "Name", "Type", "Content");
                    println!("{}", "-".repeat(80));
                    for record in &dc_records {
                        println!(
                            "{:<40} {:<10} {:<20}",
                            record["name"].as_str().unwrap_or("N/A"),
                            record["type"].as_str().unwrap_or("N/A"),
                            record["content"].as_str().unwrap_or("N/A")
                        );
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
        GatewayAction::Ssh {
            host,
            port,
            identity_file,
        } => {
            println!("Testing SSH connectivity to {}:{}", host, port);

            // Use ssh command to test connectivity
            let output = tokio::process::Command::new("ssh")
                .args([
                    "-o",
                    "StrictHostKeyChecking=no",
                    "-o",
                    "ConnectTimeout=10",
                    "-i",
                    &identity_file,
                    "-p",
                    &port.to_string(),
                    &format!("root@{}", host),
                    "echo",
                    "SSH_CONNECTION_OK",
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
        GatewayAction::Tcp {
            host,
            external_port,
            expect_response,
        } => {
            println!("Testing TCP connectivity to {}:{}", host, external_port);

            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            use tokio::net::TcpStream;

            let addr = format!("{}:{}", host, external_port);
            let mut stream = TcpStream::connect(&addr)
                .await
                .with_context(|| format!("Failed to connect to {}", addr))?;

            println!("TCP connection established.");

            if let Some(expected) = expect_response {
                // Send a simple ping and wait for response
                stream.write_all(b"ping\n").await?;

                let mut buffer = [0u8; 1024];
                let n = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    stream.read(&mut buffer),
                )
                .await
                .context("Timeout waiting for response")??;

                let response = String::from_utf8_lossy(&buffer[..n]);
                if response.contains(&expected) {
                    println!("Expected response received: {}", response.trim());
                } else {
                    anyhow::bail!(
                        "Unexpected response: expected '{}', got '{}'",
                        expected,
                        response.trim()
                    );
                }
            } else {
                println!("TCP connectivity OK (no response check requested).");
            }
        }
        GatewayAction::Contract {
            contract_id,
            identity,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let path = format!("/contracts/{}", contract_id);
            let contract: Contract = client.get_api(&path).await?;

            println!("Testing gateway connectivity for contract: {}", contract_id);

            let gateway_host = contract
                .gateway_subdomain
                .context("Contract has no gateway subdomain")?;

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

            if let (Some(start), Some(end)) = (
                contract.gateway_port_range_start,
                contract.gateway_port_range_end,
            ) {
                println!("\nTesting port range {}-{}...", start, end);
                use tokio::net::TcpStream;
                for port in start..=end.min(start + 5) {
                    // Test first 5 ports max
                    let addr = format!("{}:{}", gateway_host, port);
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(2),
                        TcpStream::connect(&addr),
                    )
                    .await
                    {
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
                    println!(
                        "API Server: ✓ healthy ({:.0}ms)",
                        start.elapsed().as_millis()
                    );
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
            let api_result = http
                .get(&url)
                .send()
                .await
                .map(|_| format!("{:.0}ms", start.elapsed().as_millis()))
                .map_err(|e| anyhow::anyhow!("{}", e));
            check_health("API Server", api_result).await;

            // Database (via API health)
            let start = std::time::Instant::now();
            let url = format!("{}/api/v1/providers?limit=1", api_url);
            let db_result = http
                .get(&url)
                .send()
                .await
                .map(|_| format!("{:.0}ms", start.elapsed().as_millis()))
                .map_err(|e| anyhow::anyhow!("{}", e));
            check_health("Database", db_result).await;

            // Cloudflare
            if env::var("CLOUDFLARE_API_TOKEN").is_ok() {
                let start = std::time::Instant::now();
                let cf_result = http
                    .get("https://api.cloudflare.com/client/v4/user/tokens/verify")
                    .header(
                        "Authorization",
                        format!("Bearer {}", env::var("CLOUDFLARE_API_TOKEN").unwrap()),
                    )
                    .send()
                    .await
                    .map(|_| format!("{:.0}ms", start.elapsed().as_millis()))
                    .map_err(|e| anyhow::anyhow!("{}", e));
                check_health("Cloudflare DNS", cf_result).await;
            } else {
                println!("Cloudflare DNS: - not configured");
            }

            // Stripe
            if env::var("STRIPE_SECRET_KEY").is_ok() {
                let start = std::time::Instant::now();
                let stripe_result = http
                    .get("https://api.stripe.com/v1/balance")
                    .header(
                        "Authorization",
                        format!("Bearer {}", env::var("STRIPE_SECRET_KEY").unwrap()),
                    )
                    .send()
                    .await
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
                let tg_result = http
                    .get(&url)
                    .send()
                    .await
                    .map(|_| format!("{:.0}ms", start.elapsed().as_millis()))
                    .map_err(|e| anyhow::anyhow!("{}", e));
                check_health("Telegram Bot", tg_result).await;
            } else {
                println!("Telegram Bot: - not configured");
            }

            println!("{}", "=".repeat(60));
        }
        HealthAction::Cloudflare => {
            let token = env::var("CLOUDFLARE_API_TOKEN").context("CLOUDFLARE_API_TOKEN not set")?;
            let start = std::time::Instant::now();
            let response = http
                .get("https://api.cloudflare.com/client/v4/user/tokens/verify")
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await?;
            if response.status().is_success() {
                println!(
                    "Cloudflare DNS: ✓ healthy ({:.0}ms)",
                    start.elapsed().as_millis()
                );
            } else {
                let text = response.text().await?;
                println!("Cloudflare DNS: ✗ unhealthy - {}", text);
            }
        }
        HealthAction::Stripe => {
            let key = env::var("STRIPE_SECRET_KEY").context("STRIPE_SECRET_KEY not set")?;
            let start = std::time::Instant::now();
            let response = http
                .get("https://api.stripe.com/v1/balance")
                .header("Authorization", format!("Bearer {}", key))
                .send()
                .await?;
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
            let token = env::var("TELEGRAM_BOT_TOKEN").context("TELEGRAM_BOT_TOKEN not set")?;
            let start = std::time::Instant::now();
            let url = format!("https://api.telegram.org/bot{}/getMe", token);
            let response = http.get(&url).send().await?;
            if response.status().is_success() {
                println!(
                    "Telegram Bot: ✓ healthy ({:.0}ms)",
                    start.elapsed().as_millis()
                );
            } else {
                let text = response.text().await?;
                println!("Telegram Bot: ✗ unhealthy - {}", text);
            }
        }
    }
    Ok(())
}

// =============================================================================
// Shared helpers (used across multiple handlers)
// =============================================================================

async fn connect_db() -> Result<Database> {
    let db_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| api::database::DEFAULT_DATABASE_URL.to_string());
    Database::connect(&db_url).await
}

async fn fetch_offerings(api_url: &str) -> Result<Vec<Offering>> {
    let http = reqwest::Client::new();
    let url = format!("{}/api/v1/offerings?limit=50&in_stock_only=true", api_url);
    let response = http.get(&url).send().await?;
    let text = response.text().await?;
    let api_response: api_cli::client::ApiResponse<Vec<Offering>> = serde_json::from_str(&text)?;
    api_response.into_result()
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
    client: &SignedClient,
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
    client: &SignedClient,
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

/// Verify SSH port is reachable via gateway hostname.
/// Retries with 10s intervals to allow DNS propagation.
/// Fails loudly with diagnostic information if unreachable.
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
    unreachable!()
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
        payment_method: Some("icpay".to_string()),
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

    let http = reqwest::Client::new();
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

// =============================================================================
// E2E handlers
// =============================================================================

async fn handle_e2e_action(action: E2eAction, api_url: &str) -> Result<()> {
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

            // Step 1: Create contract (icpay payment auto-succeeds and auto-accepts)
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
                    // Prefer non-example offerings (example offerings have fake pubkeys)
                    let offering = offerings
                        .iter()
                        .find(|o| !o.is_example)
                        .or(offerings.first())
                        .context("No offerings available for lifecycle test")?;
                    println!(
                        "  Found offering: {} (ID: {})",
                        offering.offer_name.as_deref().unwrap_or("N/A"),
                        offering.id
                    );
                    offering.id
                }
            };

            // Step 2: Create contract (icpay auto-succeeds payment)
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
            let http = reqwest::Client::new();
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
                        // Prefer non-example offerings (example offerings have fake pubkeys)
                        let offering = offerings
                            .iter()
                            .find(|o| !o.is_example)
                            .unwrap_or(&offerings[0]);
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

// =============================================================================
// Admin handlers (existing)
// =============================================================================

async fn handle_admin_action(action: AdminAction) -> Result<()> {
    let db = connect_db().await?;

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
// Cloud handlers
// =============================================================================

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CloudAccountListResponse {
    accounts: Vec<CloudAccountResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct CloudResourceResponse {
    id: String,
    name: String,
    server_type: String,
    location: String,
    image: String,
    status: String,
    public_ip: Option<String>,
    cloud_account_name: String,
    cloud_account_backend: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CloudResourceListResponse {
    resources: Vec<CloudResourceResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogServerType {
    id: String,
    name: String,
    cores: u32,
    memory_gb: f64,
    disk_gb: u32,
    price_monthly: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogLocation {
    id: String,
    name: String,
    city: String,
    country: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogImage {
    id: String,
    name: String,
    os_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogResponse {
    server_types: Vec<CatalogServerType>,
    locations: Vec<CatalogLocation>,
    images: Vec<CatalogImage>,
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

async fn handle_cloud_action(action: CloudAction, api_url: &str) -> Result<()> {
    match action {
        CloudAction::ListAccounts { identity } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let resp: CloudAccountListResponse = client.get_api("/cloud-accounts").await?;

            if resp.accounts.is_empty() {
                println!("No cloud accounts found.");
            } else {
                println!("\nCloud Accounts:");
                println!("{}", "=".repeat(110));
                println!(
                    "{:<38} {:<20} {:<12} {:<8} {:<20}",
                    "ID", "Name", "Backend", "Valid?", "Created"
                );
                println!("{}", "-".repeat(110));
                for a in &resp.accounts {
                    let valid = if a.is_valid { "yes" } else { "NO" };
                    let created = &a.created_at[..a.created_at.len().min(19)];
                    println!(
                        "{:<38} {:<20} {:<12} {:<8} {:<20}",
                        a.id,
                        &a.name[..a.name.len().min(18)],
                        a.backend_type,
                        valid,
                        created
                    );
                    if let Some(err) = &a.validation_error {
                        println!("  Error: {}", err);
                    }
                }
                println!("{}", "=".repeat(110));
                println!("Total: {} account(s)", resp.accounts.len());
            }
        }
        CloudAction::AddAccount {
            identity,
            backend,
            name,
            credentials,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let request = AddCloudAccountRequest {
                backend_type: backend,
                name: name.clone(),
                credentials,
            };

            let account: CloudAccountResponse =
                client.post_api("/cloud-accounts", &request).await?;
            println!("Cloud account created:");
            println!("  ID: {}", account.id);
            println!("  Name: {}", account.name);
            println!("  Backend: {}", account.backend_type);
            println!("  Valid: {}", account.is_valid);
        }
        CloudAction::DeleteAccount { identity, id: account_id } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let path = format!("/cloud-accounts/{}", account_id);
            let _: serde_json::Value = client.delete_api(&path).await?;
            println!("Cloud account {} deleted.", account_id);
        }
        CloudAction::Catalog {
            identity,
            account_id,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let path = format!("/cloud-accounts/{}/catalog", account_id);
            let catalog: CatalogResponse = client.get_api(&path).await?;

            // Server types
            println!("\nServer Types:");
            println!("{}", "=".repeat(90));
            println!(
                "{:<12} {:<25} {:<8} {:<12} {:<10} {:<12}",
                "ID", "Name", "Cores", "Memory GB", "Disk GB", "Price/mo"
            );
            println!("{}", "-".repeat(90));
            for st in &catalog.server_types {
                let price = st
                    .price_monthly
                    .map(|p| format!("${:.2}", p))
                    .unwrap_or_else(|| "N/A".to_string());
                println!(
                    "{:<12} {:<25} {:<8} {:<12.1} {:<10} {:<12}",
                    st.id,
                    &st.name[..st.name.len().min(23)],
                    st.cores,
                    st.memory_gb,
                    st.disk_gb,
                    price
                );
            }
            println!("{}", "=".repeat(90));
            println!("Total: {} server type(s)", catalog.server_types.len());

            // Locations
            println!("\nLocations:");
            println!("{}", "=".repeat(70));
            println!(
                "{:<12} {:<20} {:<20} {:<15}",
                "ID", "Name", "City", "Country"
            );
            println!("{}", "-".repeat(70));
            for loc in &catalog.locations {
                println!(
                    "{:<12} {:<20} {:<20} {:<15}",
                    loc.id, loc.name, loc.city, loc.country
                );
            }
            println!("{}", "=".repeat(70));
            println!("Total: {} location(s)", catalog.locations.len());

            // Images
            println!("\nImages:");
            println!("{}", "=".repeat(60));
            println!("{:<25} {:<20} {:<12}", "ID", "Name", "OS Type");
            println!("{}", "-".repeat(60));
            for img in &catalog.images {
                println!(
                    "{:<25} {:<20} {:<12}",
                    &img.id[..img.id.len().min(23)],
                    &img.name[..img.name.len().min(18)],
                    img.os_type
                );
            }
            println!("{}", "=".repeat(60));
            println!("Total: {} image(s)", catalog.images.len());
        }
        CloudAction::ListResources { identity } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let resp: CloudResourceListResponse = client.get_api("/cloud-resources").await?;

            if resp.resources.is_empty() {
                println!("No cloud resources found.");
            } else {
                println!("\nCloud Resources:");
                println!("{}", "=".repeat(130));
                println!(
                    "{:<38} {:<15} {:<12} {:<16} {:<12} {:<15} {:<12}",
                    "ID", "Name", "Status", "IP", "Type", "Account", "Backend"
                );
                println!("{}", "-".repeat(130));
                for r in &resp.resources {
                    let ip = r.public_ip.as_deref().unwrap_or("N/A");
                    println!(
                        "{:<38} {:<15} {:<12} {:<16} {:<12} {:<15} {:<12}",
                        r.id,
                        &r.name[..r.name.len().min(13)],
                        r.status,
                        ip,
                        r.server_type,
                        &r.cloud_account_name[..r.cloud_account_name.len().min(13)],
                        r.cloud_account_backend
                    );
                }
                println!("{}", "=".repeat(130));
                println!("Total: {} resource(s)", resp.resources.len());
            }
        }
        CloudAction::Provision {
            identity,
            account_id,
            name,
            server_type,
            location,
            image,
            ssh_pubkey,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let request = ProvisionResourceRequest {
                cloud_account_id: account_id,
                name,
                server_type,
                location,
                image,
                ssh_pubkey,
            };

            let resource: serde_json::Value =
                client.post_api("/cloud-resources", &request).await?;
            println!("Cloud resource provisioning started:");
            println!("  ID: {}", resource["id"].as_str().unwrap_or("N/A"));
            println!("  Name: {}", resource["name"].as_str().unwrap_or("N/A"));
            println!(
                "  Status: {}",
                resource["status"].as_str().unwrap_or("N/A")
            );
        }
        CloudAction::DeleteResource { identity, id: resource_id } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let path = format!("/cloud-resources/{}", resource_id);
            let _: serde_json::Value = client.delete_api(&path).await?;
            println!("Cloud resource {} deleted.", resource_id);
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

    let db = connect_db().await?;

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
    println!(
        "Total: {}, Success: {}, Errors: {}",
        line_count,
        success_count,
        errors.len()
    );

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
        println!(
            "\n✓ Successfully seeded {} offerings for {}",
            success_count, name
        );
    }

    Ok(())
}
