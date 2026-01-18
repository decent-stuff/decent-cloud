use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dc_agent::{
    api_client::{setup_agent, ApiClient, ReconcileResponse},
    config::{Config, ProvisionerConfig},
    gateway::GatewayManager,
    provisioner::{
        manual::ManualProvisioner, proxmox::ProxmoxProvisioner, script::ScriptProvisioner,
        ProvisionRequest, Provisioner,
    },
    registration::{default_agent_dir, generate_agent_keypair},
    setup::{proxmox::OsTemplate, GatewaySetup, ProxmoxSetup},
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Map of provisioner type name to provisioner instance
type ProvisionerMap = HashMap<String, Box<dyn Provisioner>>;
/// Optional gateway manager wrapped in Arc<Mutex> for shared async access
type OptionalGatewayManager = Option<std::sync::Arc<tokio::sync::Mutex<GatewayManager>>>;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

/// Tracks when orphan VMs were first detected for automatic pruning
#[derive(Debug, Default)]
struct OrphanTracker {
    /// Map of external_id -> timestamp when first detected
    first_seen: HashMap<String, u64>,
}

#[derive(Parser)]
#[command(name = "dc-agent", version)]
#[command(about = "Decent Cloud Provider Provisioning Agent", long_about = None)]
struct Cli {
    /// Path to configuration file
    #[arg(long, default_value = "dc-agent.toml")]
    config: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the agent polling loop
    Run,
    /// Check agent configuration and connectivity
    Doctor {
        /// Skip API authentication verification
        #[arg(long, default_value = "false")]
        no_verify_api: bool,

        /// Skip provisioning test (cloning and deleting a test VM)
        #[arg(long, default_value = "false")]
        no_test_provision: bool,
    },
    /// Set up a new provisioner
    Setup {
        #[command(subcommand)]
        provisioner: Box<SetupProvisioner>,
    },
    /// Test provisioning by creating and optionally destroying a test VM
    TestProvision {
        /// SSH public key to inject into the test VM
        #[arg(long)]
        ssh_pubkey: Option<String>,

        /// Keep the VM running after provisioning (don't terminate)
        #[arg(long, default_value = "false")]
        keep: bool,

        /// Custom contract ID for the test (default: test-<timestamp>)
        #[arg(long)]
        contract_id: Option<String>,
    },
}

#[derive(Subcommand)]
enum SetupProvisioner {
    /// Set up agent using a setup token from the provider
    Token {
        /// Setup token from provider's pool management UI
        #[arg(long)]
        token: String,

        /// API endpoint (default: https://api.decent-cloud.org)
        #[arg(long, default_value = "https://api.decent-cloud.org")]
        api_url: String,

        /// Output config file path
        #[arg(long, default_value = "dc-agent.toml")]
        output: PathBuf,

        /// Force registration even if detected location doesn't match pool location
        #[arg(long, default_value = "false")]
        force: bool,

        // === Optional: Automated Proxmox setup ===
        /// Proxmox host IP or hostname (enables automatic Proxmox setup)
        #[arg(long)]
        proxmox_host: Option<String>,

        /// Proxmox SSH port (default: 22)
        #[arg(long, default_value = "22")]
        proxmox_ssh_port: u16,

        /// Proxmox SSH username (default: root)
        #[arg(long, default_value = "root")]
        proxmox_ssh_user: String,

        /// Proxmox API username (default: root@pam)
        #[arg(long, default_value = "root@pam")]
        proxmox_user: String,

        /// Storage for VM disks (default: local-lvm)
        #[arg(long, default_value = "local-lvm")]
        proxmox_storage: String,

        /// OS templates to create (comma-separated: ubuntu-24.04,debian-12,rocky-9)
        #[arg(long, default_value = "ubuntu-24.04")]
        proxmox_templates: String,

        /// Skip interactive prompts (use with --proxmox-host for non-interactive setup)
        #[arg(long, default_value = "false")]
        non_interactive: bool,

        // === Optional: Gateway setup (Caddy reverse proxy) ===
        /// Datacenter identifier for gateway (e.g., dc-lk). Enables gateway setup.
        #[arg(long)]
        gateway_datacenter: Option<String>,

        /// Host's public IPv4 address (required if --gateway-datacenter is set)
        #[arg(long)]
        gateway_public_ip: Option<String>,

        /// Base domain for gateway (default: decent-cloud.org)
        #[arg(long, default_value = "decent-cloud.org")]
        gateway_domain: String,

        /// Start of port range for VM allocation (default: 20000)
        #[arg(long, default_value = "20000")]
        gateway_port_start: u16,

        /// End of port range for VM allocation (default: 59999)
        #[arg(long, default_value = "59999")]
        gateway_port_end: u16,

        /// Number of ports per VM (default: 10)
        #[arg(long, default_value = "10")]
        gateway_ports_per_vm: u16,
    },
    /// Set up Proxmox VE provisioner (creates templates and API token).
    /// Note: Agent registration now requires a setup token from a pool.
    /// Create a pool via the provider dashboard, then run: dc-agent setup token --token <TOKEN>
    Proxmox {
        /// Proxmox host IP or hostname
        #[arg(long)]
        host: String,

        /// SSH port (default: 22)
        #[arg(long, default_value = "22")]
        ssh_port: u16,

        /// SSH username (default: root)
        #[arg(long, default_value = "root")]
        ssh_user: String,

        /// Proxmox API username (default: root@pam)
        #[arg(long, default_value = "root@pam")]
        proxmox_user: String,

        /// Storage for VM disks (default: local-lvm)
        #[arg(long, default_value = "local-lvm")]
        storage: String,

        /// OS templates to create (comma-separated: ubuntu-24.04,debian-12,rocky-9)
        /// Available: ubuntu-24.04, ubuntu-22.04, debian-12, rocky-9
        #[arg(long, default_value = "ubuntu-24.04")]
        templates: String,

        /// Output config file path
        #[arg(long, default_value = "dc-agent.toml")]
        output: PathBuf,
    },
    // Note: Gateway setup is integrated into the Token command via --gateway-* flags.
    // Use: dc-agent setup token --gateway-datacenter <DC> --gateway-public-ip <IP> ...
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run | Commands::Doctor { .. } | Commands::TestProvision { .. } => {
            let config = Config::load(&cli.config)?;
            match cli.command {
                Commands::Run => run_agent(config).await,
                Commands::Doctor {
                    no_verify_api,
                    no_test_provision,
                } => run_doctor(config, !no_verify_api, !no_test_provision).await,
                Commands::TestProvision {
                    ssh_pubkey,
                    keep,
                    contract_id,
                } => run_test_provision(config, ssh_pubkey, keep, contract_id).await,
                _ => unreachable!(),
            }
        }
        Commands::Setup { provisioner } => run_setup(*provisioner).await,
    }
}

/// Setup agent using a one-time setup token.
#[allow(clippy::too_many_arguments)]
async fn run_setup_token(
    token: &str,
    api_url: &str,
    output: &std::path::Path,
    force: bool,
    proxmox_host: Option<String>,
    proxmox_ssh_port: u16,
    proxmox_ssh_user: &str,
    proxmox_user: &str,
    proxmox_storage: &str,
    proxmox_templates: &str,
    non_interactive: bool,
    // Gateway parameters
    gateway_datacenter: Option<String>,
    gateway_public_ip: Option<String>,
    gateway_domain: &str,
    gateway_port_start: u16,
    gateway_port_end: u16,
    gateway_ports_per_vm: u16,
    gateway_ssh_host: Option<String>,
) -> Result<()> {
    use dc_agent::geolocation::{country_to_region, detect_country, region_display_name};
    use std::io::Write;

    println!("dc-agent setup token");
    println!("====================\n");

    // Step 1: Detect agent's location via IP geolocation
    println!("Detecting agent location...");
    let detected_country = match detect_country().await {
        Ok(Some(country)) => {
            let region = country_to_region(&country);
            let region_name = region
                .and_then(region_display_name)
                .unwrap_or("Unknown region");
            println!("[ok] Detected location: {} ({})", country, region_name);
            Some((country, region))
        }
        Ok(None) => {
            println!("[warn] Could not determine country from IP address");
            None
        }
        Err(e) => {
            println!("[warn] Failed to detect location: {}", e);
            None
        }
    };

    // Step 2: Generate agent keypair
    let agent_dir = default_agent_dir()?;
    let (key_path, agent_pubkey) = generate_agent_keypair(&agent_dir, false)?;
    println!("[ok] Agent keypair generated: {}", key_path.display());

    // Step 3: Register with API using token
    println!("\nRegistering with API...");
    let response = setup_agent(api_url, token, &agent_pubkey).await?;

    println!("[ok] Agent registered successfully!");
    println!();
    println!("Pool: {} ({})", response.pool_name, response.pool_id);
    println!("Location: {}", response.pool_location);

    // Step 4: Check if detected location matches pool location
    if let Some((country, Some(detected))) = detected_country {
        if detected != response.pool_location {
            let detected_name = region_display_name(detected).unwrap_or(detected);
            let pool_name =
                region_display_name(&response.pool_location).unwrap_or(&response.pool_location);

            println!();
            println!("WARNING: Location mismatch detected!");
            println!("  Detected region: {} ({})", detected_name, country);
            println!("  Pool region: {}", pool_name);
            println!();

            if !force {
                anyhow::bail!(
                    "Agent location ({}) does not match pool location ({}). \
                     Use --force to override this check.",
                    detected,
                    response.pool_location
                );
            }

            println!("[forced] Proceeding despite location mismatch (--force specified)");
        } else {
            println!("[ok] Location matches pool: {}", detected);
        }
    }

    println!("Provisioner type: {}", response.provisioner_type);
    println!("Permissions: {}", response.permissions.join(", "));

    // Step 5: If pool uses Proxmox, optionally run automated setup
    let proxmox_host_for_gateway = proxmox_host.clone();
    let proxmox_config = if response.provisioner_type == "proxmox" {
        run_proxmox_setup_if_requested(
            proxmox_host,
            proxmox_ssh_port,
            proxmox_ssh_user,
            proxmox_user,
            proxmox_storage,
            proxmox_templates,
            non_interactive,
        )
        .await?
    } else {
        None
    };

    // Step 6: Write config file with appropriate template based on provisioner type
    let provisioner_template = match response.provisioner_type.as_str() {
        "proxmox" => {
            if let Some(ref pconfig) = proxmox_config {
                // Use actual Proxmox config from automated setup
                format!(
                    r#"
# Proxmox VE provisioner configuration (auto-configured)
[provisioner.proxmox]
api_url = "{}"
api_token_id = "{}"
api_token_secret = "{}"
node = "{}"
template_vmid = {}
storage = "{}"
verify_ssl = false
"#,
                    pconfig.api_url,
                    pconfig.api_token_id,
                    pconfig.api_token_secret,
                    pconfig.node,
                    pconfig.template_vmid,
                    pconfig.storage
                )
            } else {
                // Use placeholder template
                r#"
# Proxmox VE provisioner configuration
[provisioner.proxmox]
api_url = "https://YOUR-PROXMOX-HOST:8006"
api_token_id = "root@pam!dc-agent"
api_token_secret = "REPLACE-WITH-YOUR-API-TOKEN-SECRET"
node = "pve1"                    # Target Proxmox node name
template_vmid = 9000             # VM template ID to clone from
storage = "local-lvm"            # Storage for VM disks
# pool = "dc-vms"                # Optional: Resource pool for VMs
verify_ssl = false               # Set to true if using valid SSL cert
"#
                .to_string()
            }
        }
        "script" => r#"
# Script-based provisioner configuration
[provisioner.script]
provision = "/opt/dc-agent/provision.sh"      # Script to provision a VM
terminate = "/opt/dc-agent/terminate.sh"      # Script to terminate a VM
health_check = "/opt/dc-agent/health.sh"      # Script to check VM health
timeout_seconds = 300
"#
        .to_string(),
        "manual" => r#"
# Manual provisioner (notification-only)
[provisioner.manual]
# notification_webhook = "https://hooks.slack.com/services/YOUR/WEBHOOK/URL"
"#
        .to_string(),
        _ => r#"
# Unknown provisioner type - please configure manually
"#
        .to_string(),
    };

    let config_content = format!(
        r#"# DC-Agent Configuration
# Generated by: dc-agent setup token

[api]
endpoint = "{api_url}"
provider_pubkey = "{provider_pubkey}"
agent_secret_key = "{agent_key}"
pool_id = "{pool_id}"

[polling]
interval_seconds = 30
health_check_interval_seconds = 300

[provisioner]
type = "{provisioner_type}"
{provisioner_template}"#,
        api_url = api_url,
        provider_pubkey = response.provider_pubkey,
        agent_key = key_path.to_string_lossy(),
        pool_id = response.pool_id,
        provisioner_type = response.provisioner_type,
        provisioner_template = provisioner_template,
    );

    let mut file = std::fs::File::create(output)?;
    file.write_all(config_content.as_bytes())?;

    println!();
    println!("Configuration written to: {}", output.display());

    // Step 7: Run gateway setup if parameters provided
    let gateway_configured = run_gateway_setup_if_requested(
        gateway_datacenter,
        gateway_public_ip,
        gateway_domain,
        gateway_port_start,
        gateway_port_end,
        gateway_ports_per_vm,
        gateway_ssh_host.or(proxmox_host_for_gateway),
        proxmox_ssh_port,
        proxmox_ssh_user,
        non_interactive,
        output,
    )
    .await?;

    println!();

    // Provide type-specific next steps
    match response.provisioner_type.as_str() {
        "proxmox" => {
            if proxmox_config.is_some() && gateway_configured {
                println!("✓ Proxmox and Gateway configured successfully!");
                println!();
                println!("Configuration is ready to use. Next steps:");
                println!("  1. Verify: dc-agent --config {} doctor", output.display());
                println!("  2. Start: dc-agent --config {} run", output.display());
            } else if proxmox_config.is_some() {
                println!("✓ Proxmox configured successfully!");
                println!();
                println!("Configuration is ready to use. Next steps:");
                println!("  1. Verify: dc-agent --config {} doctor", output.display());
                println!("  2. Start: dc-agent --config {} run", output.display());
                if !gateway_configured {
                    println!();
                    println!("Note: Gateway not configured. VMs will need public IPs.");
                    println!("  To enable gateway, run setup again with:");
                    println!("    --gateway-datacenter <DC> --gateway-public-ip <IP>");
                }
            } else {
                println!(
                    "IMPORTANT: You must configure Proxmox settings before running the agent!"
                );
                println!();
                println!("Next steps:");
                println!("  1. Edit {} and fill in:", output.display());
                println!("     - api_url: Your Proxmox host URL");
                println!("     - api_token_id and api_token_secret: Create in Proxmox UI");
                println!("     - node: Your Proxmox node name");
                println!("     - template_vmid: Create a template VM (e.g., Ubuntu 24.04)");
                println!();
                println!("  Alternative: Run setup again with --proxmox-host flag");
                println!(
                    "     dc-agent setup token --token {} --proxmox-host YOUR-HOST",
                    token
                );
                println!();
                println!("  2. Verify: dc-agent --config {} doctor", output.display());
                println!("  3. Start: dc-agent --config {} run", output.display());
            }
        }
        "script" => {
            println!("IMPORTANT: You must configure script paths before running the agent!");
            println!();
            println!("Next steps:");
            println!("  1. Edit {} and configure:", output.display());
            println!("     - provision: Path to provisioning script");
            println!("     - terminate: Path to termination script");
            println!("     - health_check: Path to health check script");
            println!();
            println!("  2. Verify: dc-agent --config {} doctor", output.display());
            println!("  3. Start: dc-agent --config {} run", output.display());
        }
        "manual" => {
            println!("Manual provisioner configured - no additional setup required!");
            println!();
            println!("Next steps:");
            println!(
                "  1. Optional: Edit {} to add notification webhook",
                output.display()
            );
            println!("  2. Verify: dc-agent --config {} doctor", output.display());
            println!("  3. Start: dc-agent --config {} run", output.display());
        }
        _ => {
            println!("Next steps:");
            println!(
                "  1. Edit {} and configure provisioner settings",
                output.display()
            );
            println!("  2. Run: dc-agent --config {} doctor", output.display());
            println!("  3. Run: dc-agent --config {} run", output.display());
        }
    }

    Ok(())
}

/// Optionally run Proxmox setup based on CLI args or interactive prompt.
/// Returns Some(ProxmoxConfig) if setup was completed, None otherwise.
async fn run_proxmox_setup_if_requested(
    proxmox_host: Option<String>,
    proxmox_ssh_port: u16,
    proxmox_ssh_user: &str,
    proxmox_user: &str,
    proxmox_storage: &str,
    proxmox_templates: &str,
    non_interactive: bool,
) -> Result<Option<dc_agent::config::ProxmoxConfig>> {
    use dc_agent::config::ProxmoxConfig;
    use dc_agent::setup::proxmox::{OsTemplate, ProxmoxSetup};

    // Determine if we should run Proxmox setup
    let host = if let Some(h) = proxmox_host {
        // Host provided via CLI - run setup
        h
    } else if non_interactive {
        // Non-interactive mode without host - skip setup
        return Ok(None);
    } else {
        // Interactive mode - ask user
        println!();
        println!("This pool uses Proxmox VE provisioner.");
        println!("Would you like to configure Proxmox automatically now?");
        println!("(This will SSH into Proxmox, create API tokens, and download templates)");
        print!("Configure Proxmox now? (y/n): ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Skipping Proxmox setup. You'll need to configure it manually.");
            return Ok(None);
        }

        // Prompt for Proxmox host
        print!("Proxmox host (IP or hostname): ");
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut host_input = String::new();
        std::io::stdin().read_line(&mut host_input)?;
        let host = host_input.trim().to_string();

        if host.is_empty() {
            println!("No host provided. Skipping Proxmox setup.");
            return Ok(None);
        }

        host
    };

    // Parse templates
    let template_list: Vec<OsTemplate> = proxmox_templates
        .split(',')
        .filter_map(|s: &str| OsTemplate::parse(s.trim()))
        .collect();

    if template_list.is_empty() {
        anyhow::bail!(
            "No valid templates specified. Available: ubuntu-24.04, ubuntu-22.04, debian-12, rocky-9"
        );
    }

    println!();
    println!("Proxmox Auto-Configuration");
    println!("==========================");
    println!("  Host: {}", host);
    println!("  SSH User: {}", proxmox_ssh_user);
    println!("  Proxmox User: {}", proxmox_user);
    println!("  Storage: {}", proxmox_storage);
    println!(
        "  Templates: {}",
        template_list
            .iter()
            .map(|t: &OsTemplate| t.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!();

    // Prompt for passwords
    let ssh_password =
        rpassword::prompt_password(format!("SSH password for {}@{}: ", proxmox_ssh_user, host))?;

    // Extract user part from proxmox_user (e.g., "root" from "root@pam")
    let proxmox_user_part = proxmox_user
        .split_once('@')
        .map(|(user, _)| user)
        .unwrap_or(proxmox_user);

    let proxmox_password = if proxmox_ssh_user == proxmox_user_part {
        ssh_password.clone()
    } else {
        rpassword::prompt_password(format!("Proxmox password for {}: ", proxmox_user))?
    };

    println!();

    let setup = ProxmoxSetup {
        host: host.clone(),
        port: proxmox_ssh_port,
        ssh_user: proxmox_ssh_user.to_string(),
        ssh_password,
        proxmox_user: proxmox_user.to_string(),
        proxmox_password,
        storage: proxmox_storage.to_string(),
        templates: template_list,
    };

    println!("Running Proxmox setup...");
    let result = setup.run().await?;

    println!();
    println!("✓ Proxmox setup complete!");
    println!();

    // Convert SetupResult to ProxmoxConfig
    let primary_vmid = result
        .template_vmids
        .get(&OsTemplate::Ubuntu2404)
        .or_else(|| result.template_vmids.values().next())
        .copied()
        .unwrap_or(9000);

    let config = ProxmoxConfig {
        api_url: result.api_url,
        api_token_id: result.api_token_id,
        api_token_secret: result.api_token_secret,
        node: result.node,
        template_vmid: primary_vmid,
        storage: result.storage,
        pool: None,
        verify_ssl: false,
        ip_wait_attempts: 12,
        ip_wait_interval_secs: 10,
    };

    Ok(Some(config))
}

/// Optionally run gateway setup based on CLI args.
/// Returns true if gateway was configured, false otherwise.
#[allow(clippy::too_many_arguments)]
async fn run_gateway_setup_if_requested(
    datacenter: Option<String>,
    public_ip: Option<String>,
    domain: &str,
    port_start: u16,
    port_end: u16,
    ports_per_vm: u16,
    ssh_host: Option<String>,
    ssh_port: u16,
    ssh_user: &str,
    non_interactive: bool,
    config_path: &std::path::Path,
) -> Result<bool> {
    use std::io::Write;

    // Check if gateway setup was requested
    let datacenter = match datacenter {
        Some(dc) => dc,
        None => {
            // Gateway not requested
            return Ok(false);
        }
    };

    // Validate required parameters
    let public_ip = public_ip.ok_or_else(|| {
        anyhow::anyhow!("--gateway-public-ip is required when --gateway-datacenter is set")
    })?;

    let host = ssh_host.ok_or_else(|| {
        anyhow::anyhow!("--proxmox-host is required for gateway setup (used as SSH target)")
    })?;

    println!();
    println!("Setting up Gateway (Caddy reverse proxy)...");
    println!("  Host: {}", host);
    println!("  Datacenter: {}", datacenter);
    println!("  Domain: {}", domain);
    println!("  Public IP: {}", public_ip);
    println!(
        "  Port range: {}-{} ({} per VM)",
        port_start, port_end, ports_per_vm
    );
    println!("  TLS: Automatic via Let's Encrypt HTTP-01 challenge");
    println!();

    // In non-interactive mode, we can't prompt for password
    let ssh_password = if non_interactive {
        anyhow::bail!("Gateway setup requires SSH password. Cannot run in non-interactive mode.");
    } else {
        rpassword::prompt_password(format!("SSH password for {}@{}: ", ssh_user, host))?
    };

    println!();

    let setup = GatewaySetup {
        host: host.clone(),
        ssh_port,
        ssh_user: ssh_user.to_string(),
        ssh_password,
        datacenter: datacenter.clone(),
        domain: domain.to_string(),
        public_ip: public_ip.clone(),
        port_range_start: port_start,
        port_range_end: port_end,
        ports_per_vm,
    };

    let _result = setup.run().await?;

    // Generate and append gateway config
    let gateway_config = setup.generate_gateway_config();

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(config_path)
        .context("Failed to open config file for appending gateway config")?;
    file.write_all(gateway_config.as_bytes())?;

    println!();
    println!("✓ Gateway configured successfully!");
    println!("  Gateway config appended to: {}", config_path.display());

    Ok(true)
}

async fn run_setup(provisioner: SetupProvisioner) -> Result<()> {
    match provisioner {
        SetupProvisioner::Token {
            token,
            api_url,
            output,
            force,
            proxmox_host,
            proxmox_ssh_port,
            proxmox_ssh_user,
            proxmox_user,
            proxmox_storage,
            proxmox_templates,
            non_interactive,
            gateway_datacenter,
            gateway_public_ip,
            gateway_domain,
            gateway_port_start,
            gateway_port_end,
            gateway_ports_per_vm,
        } => {
            run_setup_token(
                &token,
                &api_url,
                &output,
                force,
                proxmox_host.clone(),
                proxmox_ssh_port,
                &proxmox_ssh_user,
                &proxmox_user,
                &proxmox_storage,
                &proxmox_templates,
                non_interactive,
                // Gateway parameters
                gateway_datacenter,
                gateway_public_ip,
                &gateway_domain,
                gateway_port_start,
                gateway_port_end,
                gateway_ports_per_vm,
                proxmox_host, // Reuse as SSH host for gateway
            )
            .await
        }
        SetupProvisioner::Proxmox {
            host,
            ssh_port,
            ssh_user,
            proxmox_user,
            storage,
            templates,
            output,
        } => {
            println!("Decent Cloud Agent - Proxmox Setup");
            println!("===================================\n");

            // Parse templates
            let template_list: Vec<OsTemplate> = templates
                .split(',')
                .filter_map(|s| {
                    let t = OsTemplate::parse(s.trim());
                    if t.is_none() {
                        warn!(template = %s.trim(), "Unknown template, skipping");
                    }
                    t
                })
                .collect();

            if template_list.is_empty() {
                anyhow::bail!("No valid templates specified. Available: ubuntu-24.04, ubuntu-22.04, debian-12, rocky-9");
            }

            println!();
            println!("Proxmox Setup");
            println!("  Host: {}", host);
            println!("  SSH User: {}", ssh_user);
            println!("  Proxmox User: {}", proxmox_user);
            println!("  Storage: {}", storage);
            println!(
                "  Templates: {}",
                template_list
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            println!();

            // Prompt for passwords
            let ssh_password =
                rpassword::prompt_password(format!("SSH password for {}@{}: ", ssh_user, host))?;
            // Extract user part from proxmox_user (e.g., "root" from "root@pam")
            let proxmox_user_part = proxmox_user
                .split_once('@')
                .map(|(user, _)| user)
                .unwrap_or(&proxmox_user);
            let proxmox_password = if ssh_user == proxmox_user_part {
                ssh_password.clone()
            } else {
                rpassword::prompt_password(format!("Proxmox password for {}: ", proxmox_user))?
            };

            println!();

            let setup = ProxmoxSetup {
                host: host.clone(),
                port: ssh_port,
                ssh_user,
                ssh_password,
                proxmox_user,
                proxmox_password,
                storage,
                templates: template_list,
            };

            let result = setup.run().await?;

            // Write partial config file (without API credentials - those come from token setup)
            result.write_proxmox_config(&output)?;

            println!();
            println!("===================================");
            println!("Proxmox setup complete!");
            println!();
            println!("Proxmox configuration written to: {}", output.display());
            println!();
            println!("Next steps:");
            println!("  1. Create an agent pool in the provider dashboard");
            println!("  2. Generate a setup token for the pool");
            println!("  3. Run: dc-agent setup token --token <YOUR_TOKEN>");
            println!("     This will register the agent and add API credentials to the config.");
            println!("  4. Run: dc-agent --config {} doctor", output.display());
            println!("  5. Run: dc-agent --config {} run", output.display());

            Ok(())
        }
    }
}

async fn run_test_provision(
    config: Config,
    ssh_pubkey: Option<String>,
    keep: bool,
    contract_id: Option<String>,
) -> Result<()> {
    println!("dc-agent test-provision");
    println!("=======================\n");

    // For test-provision, use the default provisioner
    let provisioner = create_provisioner_from_config(&config.provisioner)?;

    // Generate contract ID if not provided
    let contract_id = match contract_id {
        Some(id) => id,
        None => {
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .context("System clock is before Unix epoch - check system time")?
                .as_secs();
            format!("test-{}", secs)
        }
    };

    // Use provided SSH key or a placeholder
    let ssh_key = ssh_pubkey.unwrap_or_else(|| {
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAITestKeyNotForRealUse test@dc-agent".to_string()
    });

    println!("Contract ID: {}", contract_id);
    println!("SSH Public Key: {}...", &ssh_key[..ssh_key.len().min(50)]);
    println!();

    let request = ProvisionRequest {
        contract_id: contract_id.clone(),
        offering_id: "test-offering".to_string(),
        cpu_cores: Some(1),
        memory_mb: Some(1024),
        storage_gb: Some(10),
        requester_ssh_pubkey: Some(ssh_key),
        instance_config: None,
    };

    println!("Provisioning test VM...");
    let start = std::time::Instant::now();
    let instance = provisioner.provision(&request).await?;
    let provision_time = start.elapsed();

    println!(
        "\n✓ VM provisioned successfully in {:.1}s",
        provision_time.as_secs_f64()
    );
    println!();
    println!("Instance details:");
    println!("  External ID: {}", instance.external_id);
    if let Some(ipv4) = &instance.ip_address {
        println!("  IPv4: {}", ipv4);
    }
    if let Some(ipv6) = &instance.ipv6_address {
        println!("  IPv6: {}", ipv6);
    }

    // Health check
    println!("\nRunning health check...");
    let health = provisioner.health_check(&instance.external_id).await?;
    println!("  Status: {:?}", health);

    if keep {
        println!("\n--keep specified, VM will remain running.");
        println!("To terminate later, use the Proxmox web UI or API.");
        if let Some(ipv4) = &instance.ip_address {
            println!("\nYou can SSH into the VM:");
            println!("  ssh root@{}", ipv4);
        }
    } else {
        println!("\nTerminating test VM...");
        provisioner.terminate(&instance.external_id).await?;
        println!("✓ VM terminated successfully");
    }

    println!("\n=======================");
    println!("Test complete!");

    Ok(())
}

async fn run_agent(config: Config) -> Result<()> {
    info!("Starting dc-agent");

    let api_client = std::sync::Arc::new(ApiClient::new(&config.api)?);
    let (provisioners, default_provisioner_type) = create_provisioner_map(&config)?;

    // Initialize gateway manager if configured
    let gateway_manager = match &config.gateway {
        Some(gw_config) => match GatewayManager::new(gw_config.clone(), api_client.clone()) {
            Ok(gm) => {
                info!(
                    datacenter = %gw_config.datacenter,
                    domain = %gw_config.domain,
                    public_ip = %gw_config.public_ip,
                    port_range = %format!("{}-{}", gw_config.port_range_start, gw_config.port_range_end),
                    "Gateway manager initialized"
                );
                Some(std::sync::Arc::new(tokio::sync::Mutex::new(gm)))
            }
            Err(e) => {
                warn!(
                    error = ?e,
                    "Gateway configured but failed to initialize - gateway features disabled"
                );
                None
            }
        },
        None => {
            info!("Gateway not configured - VMs will not get public subdomains");
            None
        }
    };

    info!(
        available_provisioners = ?provisioners.keys().collect::<Vec<_>>(),
        default = %default_provisioner_type,
        "Provisioner inventory loaded"
    );

    let poll_interval = Duration::from_secs(config.polling.interval_seconds);
    let mut poll_ticker = interval(poll_interval);

    // Start with a 60s default heartbeat interval, will be updated from server response
    let mut heartbeat_interval_secs: u64 = 60;
    let mut heartbeat_ticker = interval(Duration::from_secs(heartbeat_interval_secs));

    // Track active contracts for heartbeat reporting
    let mut active_contracts: i64 = 0;

    // Track orphan VMs for automatic cleanup
    let mut orphan_tracker = OrphanTracker::default();

    // Track consecutive failures for escalating log levels
    let mut heartbeat_failures: u32 = 0;
    let mut poll_failures: u32 = 0;

    info!(
        poll_interval_seconds = config.polling.interval_seconds,
        heartbeat_interval_seconds = heartbeat_interval_secs,
        orphan_grace_period_seconds = config.polling.orphan_grace_period_seconds,
        "Agent started"
    );

    // Send initial heartbeat immediately
    send_heartbeat(
        &api_client,
        &default_provisioner_type,
        active_contracts,
        &mut heartbeat_interval_secs,
        &mut heartbeat_ticker,
        &mut heartbeat_failures,
        gateway_manager.clone(),
    )
    .await;

    loop {
        tokio::select! {
            _ = poll_ticker.tick() => {
                active_contracts = poll_and_provision(&api_client, &provisioners, &default_provisioner_type, config.polling.orphan_grace_period_seconds, &mut orphan_tracker, &mut poll_failures, gateway_manager.clone()).await;
            }
            _ = heartbeat_ticker.tick() => {
                send_heartbeat(&api_client, &default_provisioner_type, active_contracts, &mut heartbeat_interval_secs, &mut heartbeat_ticker, &mut heartbeat_failures, gateway_manager.clone()).await;
            }
        }
    }
}

async fn send_heartbeat(
    api_client: &ApiClient,
    provisioner_type: &str,
    active_contracts: i64,
    heartbeat_interval_secs: &mut u64,
    heartbeat_ticker: &mut tokio::time::Interval,
    consecutive_failures: &mut u32,
    gateway_manager: OptionalGatewayManager,
) {
    // Collect bandwidth stats from gateway manager if available
    let bandwidth_stats = if let Some(ref gw) = gateway_manager {
        let gw_lock = gw.lock().await;
        let stats = gw_lock.get_bandwidth_stats();
        let allocations = gw_lock.port_allocations();

        if stats.is_empty() {
            None
        } else {
            // Map slug -> contract_id from allocations
            let reports: Vec<_> = stats
                .into_iter()
                .filter_map(|(slug, bw)| {
                    allocations.allocations.get(&slug).map(|alloc| {
                        dc_agent::api_client::VmBandwidthReport {
                            gateway_slug: slug,
                            contract_id: alloc.contract_id.clone(),
                            bytes_in: bw.bytes_in,
                            bytes_out: bw.bytes_out,
                        }
                    })
                })
                .collect();

            if reports.is_empty() {
                None
            } else {
                Some(reports)
            }
        }
    } else {
        None
    };

    match api_client
        .send_heartbeat(
            Some(env!("CARGO_PKG_VERSION")),
            Some(provisioner_type),
            None,
            active_contracts,
            bandwidth_stats,
        )
        .await
    {
        Ok(response) => {
            if *consecutive_failures > 0 {
                info!(
                    previous_failures = *consecutive_failures,
                    "Heartbeat connection restored"
                );
                *consecutive_failures = 0;
            }
            info!(
                active_contracts = active_contracts,
                next_heartbeat_seconds = response.next_heartbeat_seconds,
                "Heartbeat sent"
            );
            // Update heartbeat interval if server suggests a different one
            let suggested = response.next_heartbeat_seconds as u64;
            if suggested > 0 && suggested != *heartbeat_interval_secs {
                *heartbeat_interval_secs = suggested;
                *heartbeat_ticker = interval(Duration::from_secs(suggested));
                info!(interval_seconds = suggested, "Heartbeat interval updated");
            }
        }
        Err(e) => {
            *consecutive_failures += 1;
            // Escalate to error level after 3 consecutive failures
            if *consecutive_failures >= 3 {
                error!(
                    error = ?e,
                    consecutive_failures = *consecutive_failures,
                    "HEARTBEAT FAILURE: Agent cannot reach API server! Check network connectivity."
                );
            } else {
                warn!(
                    error = ?e,
                    consecutive_failures = *consecutive_failures,
                    "Failed to send heartbeat"
                );
            }
        }
    }
}

async fn poll_and_provision(
    api_client: &ApiClient,
    provisioners: &ProvisionerMap,
    default_provisioner_type: &str,
    orphan_grace_period_seconds: u64,
    orphan_tracker: &mut OrphanTracker,
    consecutive_failures: &mut u32,
    gateway_manager: OptionalGatewayManager,
) -> i64 {
    let mut active_count: i64 = 0;

    // Fetch pending contracts for provisioning
    match api_client.get_pending_contracts().await {
        Ok(contracts) => {
            if *consecutive_failures > 0 {
                info!(
                    previous_failures = *consecutive_failures,
                    "API connection restored"
                );
                *consecutive_failures = 0;
            }

            if !contracts.is_empty() {
                info!(count = contracts.len(), "Found pending contracts");

                for contract in &contracts {
                    info!(contract_id = %contract.contract_id, "Processing contract");

                    // Determine which provisioner to use (per-offering override or default)
                    let provisioner_type = contract
                        .provisioner_type
                        .as_deref()
                        .unwrap_or(default_provisioner_type);

                    let provisioner = match provisioners.get(provisioner_type) {
                        Some(p) => p.as_ref(),
                        None => {
                            error!(
                                contract_id = %contract.contract_id,
                                required_type = %provisioner_type,
                                available = ?provisioners.keys().collect::<Vec<_>>(),
                                "Offering requires provisioner type '{}' but agent only has: {:?}",
                                provisioner_type,
                                provisioners.keys().collect::<Vec<_>>()
                            );

                            // Report failure to API
                            if let Err(e) = api_client
                                .report_failed(
                                    &contract.contract_id,
                                    &format!(
                                        "Agent lacks required provisioner type '{}'. Available: {:?}",
                                        provisioner_type,
                                        provisioners.keys().collect::<Vec<_>>()
                                    ),
                                )
                                .await
                            {
                                error!(
                                    contract_id = %contract.contract_id,
                                    error = ?e,
                                    "Failed to report provisioner mismatch to API"
                                );
                            }
                            continue;
                        }
                    };

                    if contract.provisioner_type.is_some() {
                        info!(
                            contract_id = %contract.contract_id,
                            provisioner_type = %provisioner_type,
                            "Using offering-specific provisioner"
                        );
                    }

                    // Parse provisioner_config from offering
                    let provisioner_config: Option<serde_json::Value> =
                        match &contract.provisioner_config {
                            Some(s) => match serde_json::from_str(s) {
                                Ok(v) => Some(v),
                                Err(e) => {
                                    warn!(
                                        contract_id = %contract.contract_id,
                                        error = ?e,
                                        raw_config = %s,
                                        "Invalid provisioner_config JSON, ignoring"
                                    );
                                    None
                                }
                            },
                            None => None,
                        };

                    // Parse instance_config if present - log warning if malformed
                    let contract_instance_config: Option<serde_json::Value> =
                        match &contract.instance_config {
                            Some(s) => match serde_json::from_str(s) {
                                Ok(v) => Some(v),
                                Err(e) => {
                                    warn!(
                                        contract_id = %contract.contract_id,
                                        error = ?e,
                                        raw_config = %s,
                                        "Invalid instance_config JSON, ignoring"
                                    );
                                    None
                                }
                            },
                            None => None,
                        };

                    // Merge configs: contract instance_config overrides provisioner_config
                    let instance_config = match (provisioner_config, contract_instance_config) {
                        (Some(prov), Some(inst)) => {
                            // Merge: instance config takes precedence
                            let mut merged = prov;
                            if let serde_json::Value::Object(ref mut prov_map) = merged {
                                if let serde_json::Value::Object(inst_map) = inst {
                                    for (k, v) in inst_map {
                                        prov_map.insert(k, v);
                                    }
                                }
                            }
                            Some(merged)
                        }
                        (Some(prov), None) => Some(prov),
                        (None, Some(inst)) => Some(inst),
                        (None, None) => None,
                    };

                    // Extract specs from offering (returned by API)
                    let cpu_cores = contract.cpu_cores.map(|c| c as u32);
                    let memory_mb = contract.memory_mb();
                    let storage_gb = contract.storage_gb();

                    if cpu_cores.is_some() || memory_mb.is_some() || storage_gb.is_some() {
                        info!(
                            contract_id = %contract.contract_id,
                            cpu_cores = ?cpu_cores,
                            memory_mb = ?memory_mb,
                            storage_gb = ?storage_gb,
                            "Using offering specs for VM"
                        );
                    }

                    // Try to acquire provisioning lock (prevents race conditions with multiple agents)
                    match api_client.acquire_lock(&contract.contract_id).await {
                        Ok(true) => {
                            info!(
                                contract_id = %contract.contract_id,
                                "Acquired provisioning lock"
                            );
                        }
                        Ok(false) => {
                            info!(
                                contract_id = %contract.contract_id,
                                "Contract locked by another agent, skipping"
                            );
                            continue;
                        }
                        Err(e) => {
                            warn!(
                                contract_id = %contract.contract_id,
                                error = ?e,
                                "Failed to acquire lock, skipping contract"
                            );
                            continue;
                        }
                    }

                    let request = ProvisionRequest {
                        contract_id: contract.contract_id.clone(),
                        offering_id: contract.offering_id.clone(),
                        cpu_cores,
                        memory_mb,
                        storage_gb,
                        requester_ssh_pubkey: Some(contract.requester_ssh_pubkey.clone()),
                        instance_config,
                    };

                    // Mark contract as provisioning before starting (for UI feedback)
                    if let Err(e) = api_client
                        .report_provisioning_started(&contract.contract_id)
                        .await
                    {
                        warn!(
                            contract_id = %contract.contract_id,
                            error = ?e,
                            "Failed to report provisioning started, continuing anyway"
                        );
                    }

                    match provisioner.provision(&request).await {
                        Ok(mut instance) => {
                            info!(
                                contract_id = %contract.contract_id,
                                external_id = %instance.external_id,
                                ip_address = ?instance.ip_address,
                                "Provisioned successfully"
                            );

                            // Setup gateway (subdomain, ports, DNS) if configured
                            if let Some(ref gw) = gateway_manager {
                                let mut gw_lock = gw.lock().await;
                                match gw_lock
                                    .setup_gateway(instance.clone(), &contract.contract_id)
                                    .await
                                {
                                    Ok(updated_instance) => {
                                        instance = updated_instance;
                                        info!(
                                            contract_id = %contract.contract_id,
                                            gateway_subdomain = ?instance.gateway_subdomain,
                                            gateway_ssh_port = ?instance.gateway_ssh_port,
                                            "Gateway setup complete"
                                        );
                                    }
                                    Err(e) => {
                                        // Gateway setup failed - log but continue
                                        // VM is usable via internal IP even without gateway
                                        warn!(
                                            contract_id = %contract.contract_id,
                                            error = ?e,
                                            "Gateway setup failed - VM accessible via internal IP only"
                                        );
                                    }
                                }
                            }

                            if let Err(e) = api_client
                                .report_provisioned(&contract.contract_id, &instance)
                                .await
                            {
                                // This is a critical error - VM was created but API doesn't know
                                error!(
                                    contract_id = %contract.contract_id,
                                    external_id = %instance.external_id,
                                    ip_address = ?instance.ip_address,
                                    error = ?e,
                                    "CRITICAL: VM provisioned but failed to report to API! \
                                     Contract may be stuck. Manual intervention may be required."
                                );
                            }
                        }
                        Err(e) => {
                            // Use {:?} to show full error chain including underlying cause
                            error!(
                                contract_id = %contract.contract_id,
                                error = ?e,
                                "Provisioning failed"
                            );
                            // Include full chain in failure report
                            if let Err(report_err) = api_client
                                .report_failed(&contract.contract_id, &format!("{:?}", e))
                                .await
                            {
                                // Less critical than report_provisioned failure, but still serious
                                error!(
                                    contract_id = %contract.contract_id,
                                    original_error = ?e,
                                    report_error = %report_err,
                                    "Failed to report provisioning failure to API. \
                                     Contract may remain stuck in pending state."
                                );
                            }
                        }
                    }
                }
                active_count = contracts.len() as i64;
            }
        }
        Err(e) => {
            *consecutive_failures += 1;
            // Escalate to error level after 3 consecutive failures
            if *consecutive_failures >= 3 {
                error!(
                    error = ?e,
                    consecutive_failures = *consecutive_failures,
                    "POLL FAILURE: Cannot fetch contracts from API server! Check network connectivity."
                );
            } else {
                warn!(
                    error = ?e,
                    consecutive_failures = *consecutive_failures,
                    "Failed to fetch pending contracts"
                );
            }
            return 0;
        }
    }

    // Reconcile running instances - handles expired, cancelled, and orphan VMs
    reconcile_instances(
        api_client,
        provisioners,
        orphan_grace_period_seconds,
        orphan_tracker,
        gateway_manager,
    )
    .await;

    active_count
}

/// Reconcile running instances with the API.
/// Reports running VMs, terminates expired/cancelled contracts, and prunes orphans after grace period.
/// Collects instances from ALL provisioners and tries to terminate via the appropriate one.
async fn reconcile_instances(
    api_client: &ApiClient,
    provisioners: &ProvisionerMap,
    orphan_grace_period_seconds: u64,
    orphan_tracker: &mut OrphanTracker,
    gateway_manager: OptionalGatewayManager,
) {
    // Collect running instances from ALL provisioners
    let mut all_running_instances = Vec::new();
    for (ptype, provisioner) in provisioners {
        match provisioner.list_running_instances().await {
            Ok(instances) => {
                if !instances.is_empty() {
                    info!(
                        provisioner_type = %ptype,
                        count = instances.len(),
                        "Found running instances"
                    );
                }
                all_running_instances.extend(instances);
            }
            Err(e) => {
                warn!(
                    provisioner_type = %ptype,
                    error = ?e,
                    "Failed to list running instances from this provisioner"
                );
            }
        }
    }

    if all_running_instances.is_empty() {
        return;
    }

    // Call reconcile API
    let response: ReconcileResponse = match api_client.reconcile(&all_running_instances).await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = ?e, "Failed to reconcile with API");
            return;
        }
    };

    // Process terminations - try each provisioner until one succeeds
    for vm in &response.terminate {
        info!(
            external_id = %vm.external_id,
            contract_id = %vm.contract_id,
            reason = %vm.reason,
            "Terminating VM"
        );

        let mut terminated = false;
        for (ptype, provisioner) in provisioners {
            match provisioner.terminate(&vm.external_id).await {
                Ok(()) => {
                    info!(
                        external_id = %vm.external_id,
                        contract_id = %vm.contract_id,
                        provisioner_type = %ptype,
                        "VM terminated successfully"
                    );

                    // Cleanup gateway (DNS, Traefik config, port allocation) if configured
                    if let Some(ref gw) = gateway_manager {
                        let mut gw_lock = gw.lock().await;
                        if let Some(slug) = gw_lock.find_slug_by_contract(&vm.contract_id) {
                            if let Err(e) = gw_lock.cleanup_gateway(&slug).await {
                                warn!(
                                    contract_id = %vm.contract_id,
                                    slug = %slug,
                                    error = ?e,
                                    "Gateway cleanup failed"
                                );
                            }
                        }
                    }

                    if let Err(e) = api_client.report_terminated(&vm.contract_id).await {
                        error!(
                            contract_id = %vm.contract_id,
                            error = ?e,
                            "Failed to report termination to API. May retry on next poll."
                        );
                    }
                    terminated = true;
                    break;
                }
                Err(_) => {
                    // Try next provisioner
                    continue;
                }
            }
        }

        if !terminated {
            error!(
                external_id = %vm.external_id,
                contract_id = %vm.contract_id,
                "Termination failed - no provisioner could terminate this instance"
            );
        }
    }

    // Track and prune orphan VMs after grace period
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Collect current orphan external IDs
    let current_orphans: HashSet<String> = response
        .unknown
        .iter()
        .map(|vm| vm.external_id.clone())
        .collect();

    // Track new orphans and check grace period for existing ones
    let mut to_prune = Vec::new();
    for vm in &response.unknown {
        let first_seen = *orphan_tracker
            .first_seen
            .entry(vm.external_id.clone())
            .or_insert(now);

        let age_seconds = now.saturating_sub(first_seen);

        if age_seconds >= orphan_grace_period_seconds {
            // Grace period exceeded - prune this orphan
            to_prune.push(vm);
        } else if age_seconds == 0 || first_seen == now {
            // Newly detected orphan
            info!(
                external_id = %vm.external_id,
                message = %vm.message,
                grace_period_seconds = orphan_grace_period_seconds,
                "Orphan VM detected - will auto-prune after grace period if not resolved"
            );
        } else {
            // Existing orphan still in grace period
            warn!(
                external_id = %vm.external_id,
                message = %vm.message,
                age_seconds = age_seconds,
                remaining_seconds = orphan_grace_period_seconds.saturating_sub(age_seconds),
                "Orphan VM still present - will auto-prune if not resolved"
            );
        }
    }

    // Prune orphans that exceeded grace period
    for vm in &to_prune {
        warn!(
            external_id = %vm.external_id,
            message = %vm.message,
            grace_period_seconds = orphan_grace_period_seconds,
            "Pruning orphan VM - grace period exceeded"
        );

        let mut pruned = false;
        for (ptype, provisioner) in provisioners {
            match provisioner.terminate(&vm.external_id).await {
                Ok(()) => {
                    info!(
                        external_id = %vm.external_id,
                        provisioner_type = %ptype,
                        "Orphan VM pruned successfully"
                    );
                    pruned = true;
                    break;
                }
                Err(_) => {
                    // Try next provisioner
                    continue;
                }
            }
        }

        if !pruned {
            error!(
                external_id = %vm.external_id,
                "Orphan pruning failed - no provisioner could terminate this instance"
            );
        } else {
            // Remove from tracker after successful pruning
            orphan_tracker.first_seen.remove(&vm.external_id);
        }
    }

    // Clean up tracker - remove orphans that are no longer present (resolved)
    orphan_tracker
        .first_seen
        .retain(|external_id, first_seen_ts| {
            if current_orphans.contains(external_id) {
                true // Still an orphan, keep tracking
            } else {
                // Orphan resolved (contract fixed or VM removed manually)
                let age_seconds = now.saturating_sub(*first_seen_ts);
                info!(
                    external_id = %external_id,
                    was_tracked_for_seconds = age_seconds,
                    "Orphan VM resolved - no longer present"
                );
                false // Remove from tracker
            }
        });
}

/// Create a single provisioner from config
fn create_provisioner_from_config(prov_config: &ProvisionerConfig) -> Result<Box<dyn Provisioner>> {
    match prov_config {
        ProvisionerConfig::Proxmox(proxmox) => {
            info!("Creating Proxmox provisioner");
            Ok(Box::new(ProxmoxProvisioner::new(proxmox.clone())?))
        }
        ProvisionerConfig::Script(script) => {
            info!("Creating Script provisioner");
            Ok(Box::new(ScriptProvisioner::new(script.clone())))
        }
        ProvisionerConfig::Manual(manual) => {
            info!("Creating Manual provisioner");
            Ok(Box::new(ManualProvisioner::new(manual.clone())))
        }
    }
}

/// Create a map of all configured provisioners and return the default type
fn create_provisioner_map(config: &Config) -> Result<(ProvisionerMap, String)> {
    let mut map: ProvisionerMap = HashMap::new();

    // Add the default (required) provisioner
    let default_type = config.provisioner.type_name().to_string();
    let default_prov = create_provisioner_from_config(&config.provisioner)?;
    map.insert(default_type.clone(), default_prov);

    // Add any additional provisioners
    for additional in &config.additional_provisioners {
        let ptype = additional.type_name().to_string();
        if map.contains_key(&ptype) {
            warn!(
                provisioner_type = %ptype,
                "Duplicate provisioner type in additional_provisioners, skipping"
            );
            continue;
        }
        let prov = create_provisioner_from_config(additional)?;
        map.insert(ptype, prov);
    }

    Ok((map, default_type))
}

/// Format bytes into human-readable string (KB, MB, GB)
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

async fn run_doctor(config: Config, verify_api: bool, test_provision: bool) -> Result<()> {
    println!("dc-agent doctor");
    println!("================");
    println!();

    // Check configuration file
    println!("Configuration:");
    println!("  API endpoint: {}", config.api.endpoint);
    println!("  Provider pubkey: {}", config.api.provider_pubkey);
    println!("  Polling interval: {}s", config.polling.interval_seconds);
    println!(
        "  Health check interval: {}s",
        config.polling.health_check_interval_seconds
    );

    // Determine auth mode
    let auth_mode = if config.api.agent_secret_key.is_some() {
        "delegated agent key"
    } else if config.api.provider_secret_key.is_some() {
        "provider key (legacy)"
    } else {
        "no key configured"
    };
    println!("  Auth mode: {}", auth_mode);
    println!();

    // Try to create API client early (needed for gateway manager)
    let api_client = ApiClient::new(&config.api).ok().map(std::sync::Arc::new);

    // Check provisioner configuration and verify setup
    let provisioner = create_provisioner_from_config(&config.provisioner)?;

    // Show provisioner inventory
    println!("Provisioner Inventory:");
    println!("  Default: {} (required)", config.provisioner.type_name());
    if config.additional_provisioners.is_empty() {
        println!("  Additional: none");
    } else {
        for ap in &config.additional_provisioners {
            println!("  Additional: {}", ap.type_name());
        }
    }
    println!();

    match &config.provisioner {
        ProvisionerConfig::Proxmox(proxmox) => {
            println!("Provisioner: Proxmox");
            println!("  API URL: {}", proxmox.api_url);
            println!("  Node: {}", proxmox.node);
            println!("  Template VMID: {}", proxmox.template_vmid);
            println!("  Storage: {}", proxmox.storage);
            println!("  Verify SSL: {}", proxmox.verify_ssl);
            if let Some(pool) = &proxmox.pool {
                println!("  Resource pool: {}", pool);
            }
            println!();
            println!("Verifying Proxmox setup...");
            let verification = provisioner.verify_setup().await;
            if verification.api_reachable == Some(true) {
                println!("  [ok] Proxmox API reachable");
            }
            if verification.template_exists == Some(true) {
                println!("  [ok] Template VM {} exists", proxmox.template_vmid);
            }
            if verification.storage_accessible == Some(true) {
                println!("  [ok] Storage '{}' accessible", proxmox.storage);
            }
            if let Some(pool) = &proxmox.pool {
                if verification.pool_exists == Some(true) {
                    println!("  [ok] Pool '{}' exists", pool);
                }
            }
            if !verification.errors.is_empty() {
                println!();
                for error in &verification.errors {
                    println!("  [FAILED] {}", error);
                }
                return Err(anyhow::anyhow!(
                    "Proxmox setup verification failed with {} error(s)",
                    verification.errors.len()
                ));
            }
        }
        ProvisionerConfig::Script(script) => {
            println!("Provisioner: Script");
            println!("  Provision script: {}", script.provision);
            println!("  Terminate script: {}", script.terminate);
            println!("  Health check script: {}", script.health_check);
            println!("  Timeout: {}s", script.timeout_seconds);

            // Check if scripts exist
            for (name, path) in [
                ("provision", &script.provision),
                ("terminate", &script.terminate),
                ("health_check", &script.health_check),
            ] {
                if std::path::Path::new(path).exists() {
                    println!("  [ok] {} script exists", name);
                } else {
                    println!("  [MISSING] {} script: {}", name, path);
                }
            }
        }
        ProvisionerConfig::Manual(manual) => {
            println!("Provisioner: Manual");
            if let Some(webhook) = &manual.notification_webhook {
                println!("  Notification webhook: {}", webhook);
            } else {
                println!("  No notification webhook configured");
            }
        }
    }
    println!();

    // Check gateway configuration
    match &config.gateway {
        Some(gw) => {
            println!("Gateway:");
            println!("  Datacenter: {}", gw.datacenter);
            println!("  Domain: {}", gw.domain);
            println!("  Public IP: {}", gw.public_ip);
            println!(
                "  Port range: {}-{} ({} ports/VM)",
                gw.port_range_start, gw.port_range_end, gw.ports_per_vm
            );
            println!("  Caddy sites dir: {}", gw.caddy_sites_dir);
            println!("  Port allocations: {}", gw.port_allocations_path);
            println!("  DNS management: via central API");
            println!("  TLS: Automatic via Let's Encrypt HTTP-01");

            // Verify paths exist
            if std::path::Path::new(&gw.caddy_sites_dir).exists() {
                println!("  [ok] Caddy sites directory exists");
            } else {
                println!(
                    "  [WARN] Caddy sites directory does not exist: {}",
                    gw.caddy_sites_dir
                );
                println!("       Re-run setup with --gateway-datacenter to configure gateway");
            }

            // Check if Caddy is running
            match std::process::Command::new("systemctl")
                .args(["is-active", "caddy"])
                .output()
            {
                Ok(output) => {
                    let status = String::from_utf8_lossy(&output.stdout);
                    if status.trim() == "active" {
                        println!("  [ok] Caddy service is running");

                        // Check if Caddy is listening on expected ports
                        if let Ok(ss_output) =
                            std::process::Command::new("ss").args(["-tlnp"]).output()
                        {
                            let ss = String::from_utf8_lossy(&ss_output.stdout);
                            if ss.contains(":443") && ss.contains("caddy") {
                                println!("  [ok] Caddy listening on port 443");
                            } else if ss.contains(":443") {
                                println!("  [ok] Port 443 in use (Caddy or other)");
                            } else {
                                println!("  [WARN] Caddy not listening on port 443");
                            }
                        }
                    } else {
                        println!(
                            "  [WARN] Caddy service not running (status: {})",
                            status.trim()
                        );
                        println!("       Run: systemctl start caddy");
                    }
                }
                Err(_) => {
                    println!("  [info] Cannot check Caddy status (systemctl not available)");
                }
            }

            // Verify GatewayManager can be initialized (requires API client)
            match api_client.clone() {
                Some(client) => match GatewayManager::new(gw.clone(), client) {
                    Ok(gw_manager) => {
                        println!("  [ok] Gateway manager initialized");

                        // Show current port allocations count
                        let allocations = gw_manager.port_allocations();
                        let count = allocations.allocations.len();
                        if count > 0 {
                            println!("  [info] {} active VM(s) with gateway routing", count);
                        }

                        // Show bandwidth stats if available
                        let stats = gw_manager.get_bandwidth_stats();
                        if !stats.is_empty() {
                            println!("  Bandwidth stats:");
                            for (slug, bw) in &stats {
                                println!(
                                    "    {}: in={} out={}",
                                    slug,
                                    format_bytes(bw.bytes_in),
                                    format_bytes(bw.bytes_out)
                                );
                            }
                        }
                    }
                    Err(e) => println!("  [FAILED] Gateway initialization: {}", e),
                },
                None => {
                    println!("  [WARN] Cannot verify gateway manager (API client not available)");
                }
            }
        }
        None => {
            println!("Gateway: Not configured");
            println!("  VMs will not get public subdomains");
            println!("  To enable: re-run setup with --gateway-datacenter <DC> --gateway-public-ip <IP> ...");
        }
    }
    println!();

    let provisioner_type = config.provisioner.type_name();

    // Create API client for verification (separate from the one used for gateway check)
    let api_client = ApiClient::new(&config.api)?;
    println!("[ok] API client initialized");

    if verify_api {
        println!();
        println!("Verifying API connectivity...");

        match api_client
            .send_heartbeat(
                Some(env!("CARGO_PKG_VERSION")),
                Some(provisioner_type),
                None,
                0,
                None, // No bandwidth stats in doctor mode
            )
            .await
        {
            Ok(response) => {
                println!("[ok] API authentication successful");
                println!("  Heartbeat acknowledged: {}", response.acknowledged);
                println!("  Next heartbeat in: {}s", response.next_heartbeat_seconds);
            }
            Err(e) => {
                println!("[FAILED] API verification failed: {}", e);
                println!();
                println!("Possible causes:");
                println!("  - Agent not registered (run: dc-agent register)");
                println!("  - Agent delegation expired or revoked");
                println!("  - Invalid agent key");
                println!("  - Network connectivity issue");
                return Err(anyhow::anyhow!("API verification failed: {}", e));
            }
        }
    }

    // Test provisioning if requested (only for Proxmox)
    if test_provision {
        println!();
        println!("Testing provisioning...");

        match &config.provisioner {
            ProvisionerConfig::Proxmox(_) => {
                let test_contract_id = format!("doctor-test-{}", std::process::id());

                let request = ProvisionRequest {
                    contract_id: test_contract_id.clone(),
                    offering_id: "doctor-test".to_string(),
                    cpu_cores: Some(1),
                    memory_mb: Some(512),
                    storage_gb: None, // Use template default
                    requester_ssh_pubkey: None,
                    instance_config: None,
                };

                println!("  Cloning test VM from template...");
                match provisioner.provision(&request).await {
                    Ok(instance) => {
                        println!("[ok] Test VM created: VMID {}", instance.external_id);

                        // Check if we got an IP address (indicates QEMU guest agent is working)
                        let ip_warning = match &instance.ip_address {
                            None => {
                                println!(
                                    "[WARN] No IP address obtained - QEMU guest agent not running"
                                );
                                println!(
                                    "       Template may be missing qemu-guest-agent package."
                                );
                                println!("       Re-run setup to fix: dc-agent setup proxmox ...");
                                true
                            }
                            Some(ip) => {
                                println!("  IP address: {}", ip);
                                false
                            }
                        };

                        println!("  Terminating test VM...");
                        match provisioner.terminate(&instance.external_id).await {
                            Ok(()) => {
                                println!("[ok] Test VM terminated successfully");
                                println!();
                                if ip_warning {
                                    println!("Provisioning works but IP detection is broken!");
                                    println!("VMs will start but won't report their IP addresses.");
                                    println!();
                                    println!("To fix, install qemu-guest-agent in template:");
                                    println!("  1. SSH to Proxmox: ssh root@<proxmox-host>");
                                    println!("  2. Install libguestfs-tools: apt install libguestfs-tools");
                                    println!("  3. Customize image:");
                                    println!("     virt-customize -a /var/lib/vz/images/<vmid>/vm-<vmid>-disk-0 \\");
                                    println!("       --install qemu-guest-agent \\");
                                    println!(
                                        "       --run-command 'systemctl enable qemu-guest-agent'"
                                    );
                                } else {
                                    println!("Provisioning is working correctly!");
                                }
                            }
                            Err(e) => {
                                println!("[WARN] Test VM created but termination failed: {}", e);
                                println!(
                                    "  Manual cleanup may be required for VMID {}",
                                    instance.external_id
                                );
                            }
                        }
                    }
                    Err(e) => {
                        println!("[FAILED] Provisioning test failed: {:#}", e);
                        println!();
                        println!("Possible causes:");
                        println!("  - Template VM does not exist or is locked");
                        println!("  - Storage pool is full or inaccessible");
                        println!("  - API token lacks required permissions");
                        println!("  - Resource pool does not exist");
                        return Err(anyhow::anyhow!("Provisioning test failed: {:#}", e));
                    }
                }
            }
            _ => {
                println!("  [skip] --test-provision only supported for Proxmox provisioner");
            }
        }
    }
    println!();

    println!("Doctor check complete!");
    Ok(())
}
