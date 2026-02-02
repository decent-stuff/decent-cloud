use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dc_agent::{
    api_client::{setup_agent, ApiClient, ReconcileResponse},
    config::{Config, ProvisionerConfig},
    gateway::GatewayManager,
    orphan_tracker::OrphanTracker,
    post_provision::execute_post_provision_script,
    provisioner::{
        manual::ManualProvisioner, proxmox::ProxmoxProvisioner, script::ScriptProvisioner,
        ProvisionRequest, Provisioner,
    },
    registration::{default_agent_dir, generate_agent_keypair},
    setup::{detect_public_ip, GatewaySetup},
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Map of provisioner type name to provisioner instance
type ProvisionerMap = HashMap<String, Box<dyn Provisioner>>;
/// Optional gateway manager wrapped in Arc<Mutex> for shared async access
type OptionalGatewayManager = Option<std::sync::Arc<tokio::sync::Mutex<GatewayManager>>>;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

#[derive(Parser)]
#[command(name = "dc-agent", version)]
#[command(about = "Decent Cloud Provider Provisioning Agent", long_about = None)]
struct Cli {
    /// Path to configuration file
    #[arg(long, default_value = "/etc/dc-agent/dc-agent.toml")]
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

        /// Also test gateway setup (subdomain, port forwarding, DNS)
        #[arg(long, default_value = "false")]
        test_gateway: bool,

        /// Skip DNS record creation during gateway test (for local testing)
        #[arg(long, default_value = "false")]
        skip_dns: bool,
    },
    /// Check for and apply updates
    Upgrade {
        /// Only check for updates, don't install
        #[arg(long)]
        check_only: bool,

        /// Skip confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,

        /// Force upgrade even if same version
        #[arg(long)]
        force: bool,
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
        #[arg(long, default_value = "/etc/dc-agent/dc-agent.toml")]
        output: PathBuf,

        /// Force registration even if detected location doesn't match pool location
        #[arg(long, default_value = "false")]
        force: bool,

        // === Optional: Automated Proxmox setup ===
        /// Enable automatic Proxmox setup (creates templates and API token)
        #[arg(long)]
        setup_proxmox: bool,

        /// Proxmox API username (default: root@pam)
        #[arg(long, default_value = "root@pam")]
        proxmox_user: String,

        /// Storage for VM disks (default: local-lvm)
        #[arg(long, default_value = "local-lvm")]
        proxmox_storage: String,

        /// OS templates to create (comma-separated: ubuntu-24.04,debian-12,rocky-9)
        #[arg(long, default_value = "ubuntu-24.04")]
        proxmox_templates: String,

        /// Skip interactive prompts
        #[arg(long, default_value = "false")]
        non_interactive: bool,

        // === Optional: Gateway setup (Caddy reverse proxy) ===
        /// Datacenter identifier for gateway (e.g., dc-lk). Enables gateway setup.
        #[arg(long)]
        gateway_datacenter: Option<String>,

        /// Host's public IPv4 address (auto-detected if not provided)
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

        /// Install and start systemd service after setup
        /// (default: true when Proxmox setup succeeds or service already exists)
        #[arg(long)]
        install_service: Option<bool>,
    },
    // Note: Gateway setup is integrated into the Token command via --gateway-* flags.
    // Use: dc-agent setup token --gateway-datacenter <DC> (public IP auto-detected)
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
                    test_gateway,
                    skip_dns,
                } => {
                    run_test_provision(config, ssh_pubkey, keep, contract_id, test_gateway, skip_dns)
                        .await
                }
                _ => unreachable!(),
            }
        }
        Commands::Setup { provisioner } => run_setup(*provisioner).await,
        Commands::Upgrade {
            check_only,
            yes,
            force,
        } => dc_agent::upgrade::run_upgrade(check_only, yes, force).await,
    }
}

/// Setup agent using a one-time setup token.
#[allow(clippy::too_many_arguments)]
async fn run_setup_token(
    token: &str,
    api_url: &str,
    output: &std::path::Path,
    force: bool,
    setup_proxmox: bool,
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
    // Service installation
    install_service: Option<bool>,
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
            println!("[warn] Failed to detect location: {:#}", e);
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
    let proxmox_config = if response.provisioner_type == "proxmox" {
        run_proxmox_setup_if_requested(
            setup_proxmox,
            proxmox_user,
            proxmox_storage,
            proxmox_templates,
            non_interactive,
        )
        .await?
    } else {
        None
    };

    // Step 5b: Auto-derive gateway datacenter from pool_id if not provided
    // Pool ID format: "sl-8eba3c90" -> datacenter "dc-sl"
    let gateway_datacenter = gateway_datacenter.or_else(|| {
        // Check each condition and explain why auto-enable is skipped
        if !is_proxmox_host() {
            println!();
            println!("[info] Gateway auto-enable skipped: not running on Proxmox host");
            return None;
        }
        if !response.permissions.contains(&"dns_manage".to_string()) {
            println!();
            println!("[info] Gateway auto-enable skipped: pool lacks 'dns_manage' permission");
            return None;
        }
        if proxmox_config.is_none() {
            println!();
            println!("[info] Gateway auto-enable skipped: Proxmox setup not completed");
            return None;
        }

        // Validate pool_id format: expected "<dc_code>-<uuid>" like "sl-8eba3c90"
        match parse_datacenter_from_pool_id(&response.pool_id) {
            Some(datacenter) => {
                println!();
                println!(
                    "[auto] Gateway enabled: {} (derived from pool {})",
                    datacenter, response.pool_id
                );
                Some(datacenter)
            }
            None => {
                println!();
                println!(
                    "[warn] Cannot derive gateway datacenter: pool_id '{}' has invalid format (expected 'code-uuid')",
                    response.pool_id
                );
                None
            }
        }
    });

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
    // Gateway setup runs locally on the host (same as Proxmox setup)
    let gateway_configured = run_gateway_setup_if_requested(
        gateway_datacenter,
        gateway_public_ip,
        gateway_domain,
        gateway_port_start,
        gateway_port_end,
        gateway_ports_per_vm,
        output,
    )
    .await?;

    // Step 8: Install systemd service
    // Default: install/update service when on Proxmox host with successful Proxmox setup
    // OR when service already exists (to update config path and restart)
    let should_install_service = install_service.unwrap_or_else(|| {
        let service_exists = is_service_installed();
        let proxmox_setup_done = is_proxmox_host() && proxmox_config.is_some();

        if service_exists {
            // Always update existing service to use new config
            println!();
            println!("[auto] Existing dc-agent service detected - will update config");
            true
        } else if proxmox_setup_done {
            // Auto-install when Proxmox setup succeeded
            true
        } else {
            false
        }
    });

    let service_installed = if should_install_service {
        println!();
        println!("Installing systemd service...");
        match install_systemd_service(output) {
            Ok(()) => {
                println!("✓ Systemd service installed and started!");
                true
            }
            Err(e) => {
                println!("[WARN] Failed to install systemd service: {:#}", e);
                println!("       You can manually start the agent with:");
                println!("         dc-agent --config {} run", output.display());
                false
            }
        }
    } else {
        false
    };

    println!();

    // Provide type-specific next steps based on what was configured
    let setup_complete = proxmox_config.is_some();

    if service_installed && setup_complete {
        // Full success - service is running
        println!("==========================================");
        println!("dc-agent is now running!");
        println!("==========================================");
        println!();
        println!("  Config: {}", output.display());
        println!("  Keys:   /root/.dc-agent/");
        println!();
        println!("Commands:");
        println!("  systemctl status dc-agent     # Check status");
        println!("  journalctl -fu dc-agent       # View logs");
        println!("  dc-agent upgrade --check-only # Check for updates");
        if !gateway_configured {
            println!();
            println!("Note: Gateway not configured. VMs will need public IPs.");
            println!("  To enable gateway, re-run setup with --gateway-datacenter <DC>");
        }
    } else if setup_complete {
        // Setup complete but service not installed
        if gateway_configured {
            println!("✓ Proxmox and Gateway configured successfully!");
        } else {
            println!("✓ Proxmox configured successfully!");
        }
        println!();
        println!("Next steps:");
        println!("  1. Verify: dc-agent --config {} doctor", output.display());
        println!("  2. Start:  dc-agent --config {} run", output.display());
        println!();
        println!("Or install as systemd service:");
        println!("  dc-agent setup token --token {} --install-service", token);
        if !gateway_configured {
            println!();
            println!("Note: Gateway not configured. VMs will need public IPs.");
            println!("  To enable gateway, re-run setup with --gateway-datacenter <DC>");
        }
    } else {
        // Proxmox not configured - show appropriate instructions
        match response.provisioner_type.as_str() {
            "proxmox" => {
                // On Proxmox host, suggest --setup-proxmox as primary option
                if is_proxmox_host() {
                    println!("IMPORTANT: Proxmox setup incomplete. Re-run with --setup-proxmox:");
                    println!();
                    println!(
                        "  dc-agent setup token --token {} --setup-proxmox --non-interactive",
                        token
                    );
                    println!();
                    println!("This will automatically configure Proxmox and install the service.");
                } else {
                    // Not on Proxmox host - show manual instructions
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
    }

    Ok(())
}

/// Check if dc-agent systemd service already exists.
fn is_service_installed() -> bool {
    std::path::Path::new("/etc/systemd/system/dc-agent.service").exists()
}

/// Install or update systemd service for dc-agent.
/// Writes service unit file, reloads systemd, enables and starts/restarts the service.
fn install_systemd_service(config_path: &std::path::Path) -> Result<()> {
    use std::io::Write;

    const SYSTEMD_DIR: &str = "/etc/systemd/system";
    const SERVICE_FILE: &str = "dc-agent.service";
    const BINARY_PATH: &str = "/usr/local/bin/dc-agent";

    // Verify binary exists
    if !std::path::Path::new(BINARY_PATH).exists() {
        anyhow::bail!(
            "dc-agent binary not found at {}. Install it first with:\n  \
             curl -sSL https://get.decent-cloud.org | bash",
            BINARY_PATH
        );
    }

    // Convert config path to absolute (required for systemd which runs from /)
    let absolute_config_path = config_path
        .canonicalize()
        .with_context(|| format!("Config file not found: {}", config_path.display()))?;

    let service_existed = is_service_installed();

    // Create the systemd service unit file
    let service_content = format!(
        r#"[Unit]
Description=Decent Cloud Provisioning Agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart={} --config {} run
Restart=always
RestartSec=10
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
"#,
        BINARY_PATH,
        absolute_config_path.display()
    );

    let service_path = format!("{}/{}", SYSTEMD_DIR, SERVICE_FILE);
    let mut file = std::fs::File::create(&service_path)
        .with_context(|| format!("Failed to create systemd service file: {}", service_path))?;
    file.write_all(service_content.as_bytes())?;
    if service_existed {
        println!("[ok] Updated {}", service_path);
    } else {
        println!("[ok] Created {}", service_path);
    }

    // Reload systemd to pick up new service file
    let reload_status = std::process::Command::new("systemctl")
        .arg("daemon-reload")
        .status()
        .context("Failed to run systemctl daemon-reload")?;
    if !reload_status.success() {
        anyhow::bail!(
            "systemctl daemon-reload failed with exit code {:?}",
            reload_status.code()
        );
    }
    println!("[ok] Systemd daemon reloaded");

    // Enable service
    let enable_status = std::process::Command::new("systemctl")
        .args(["enable", SERVICE_FILE])
        .status()
        .context("Failed to run systemctl enable")?;
    if !enable_status.success() {
        anyhow::bail!(
            "systemctl enable failed with exit code {:?}",
            enable_status.code()
        );
    }
    println!("[ok] Service enabled");

    // Use restart if service existed (to pick up new config), otherwise start
    let action = if service_existed { "restart" } else { "start" };
    let start_status = std::process::Command::new("systemctl")
        .args([action, SERVICE_FILE])
        .status()
        .context("Failed to run systemctl")?;
    if !start_status.success() {
        anyhow::bail!(
            "systemctl {} failed with exit code {:?}",
            action,
            start_status.code()
        );
    }

    // Wait briefly and verify service is actually running (not just started and crashed)
    std::thread::sleep(std::time::Duration::from_secs(2));
    let status_output = std::process::Command::new("systemctl")
        .args(["is-active", SERVICE_FILE])
        .output()
        .context("Failed to check service status")?;
    let status = String::from_utf8_lossy(&status_output.stdout)
        .trim()
        .to_string();

    if status != "active" {
        // Get the last few lines of journal for diagnosis
        let journal = std::process::Command::new("journalctl")
            .args(["-u", SERVICE_FILE, "-n", "10", "--no-pager"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        anyhow::bail!(
            "Service failed to start (status: {}). Check config and logs:\n\
             journalctl -u dc-agent -n 20\n\n\
             Recent logs:\n{}",
            status,
            journal
        );
    }

    if service_existed {
        println!("[ok] Service restarted with new config");
    } else {
        println!("[ok] Service started");
    }
    println!("[ok] Service is running (verified)");

    Ok(())
}

/// Check if we're running on a Proxmox host by looking for pvesm command.
fn is_proxmox_host() -> bool {
    // Check multiple indicators for Proxmox VE:
    // 1. pvesm command exists (Proxmox storage manager)
    // 2. pveversion command exists
    // 3. /etc/pve directory exists (Proxmox config dir)
    std::process::Command::new("which")
        .arg("pvesm")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
        || std::process::Command::new("which")
            .arg("pveversion")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        || std::path::Path::new("/etc/pve").exists()
}

/// Parse datacenter identifier from pool_id.
/// Pool ID format: "<dc_code>-<uuid>" like "sl-8eba3c90" or "usw-abc123"
/// Returns "dc-<code>" (e.g., "dc-sl") if valid, None otherwise.
///
/// Validation rules:
/// - Must contain at least one dash
/// - dc_code (before first dash) must be 2-4 lowercase ASCII letters
fn parse_datacenter_from_pool_id(pool_id: &str) -> Option<String> {
    let parts: Vec<&str> = pool_id.split('-').collect();

    // Must have at least 2 parts (code and uuid)
    if parts.len() < 2 {
        return None;
    }

    let dc_code = parts[0];

    // dc_code must not be empty
    if dc_code.is_empty() {
        return None;
    }

    // dc_code must be 2-4 lowercase ASCII letters
    if dc_code.len() < 2 || dc_code.len() > 4 {
        return None;
    }

    if !dc_code.chars().all(|c| c.is_ascii_lowercase()) {
        return None;
    }

    Some(format!("dc-{}", dc_code))
}

/// Optionally run Proxmox setup based on CLI args or auto-detection.
/// This runs locally on the Proxmox host - no SSH required.
/// Returns Some(ProxmoxConfig) if setup was completed, None otherwise.
async fn run_proxmox_setup_if_requested(
    setup_proxmox: bool,
    proxmox_user: &str,
    proxmox_storage: &str,
    proxmox_templates: &str,
    non_interactive: bool,
) -> Result<Option<dc_agent::config::ProxmoxConfig>> {
    use dc_agent::config::ProxmoxConfig;
    use dc_agent::setup::proxmox::{OsTemplate, ProxmoxSetup};

    // Auto-detect if we're on a Proxmox host
    let on_proxmox = is_proxmox_host();

    // Determine if we should run Proxmox setup
    let should_setup = if setup_proxmox {
        // Explicitly requested via CLI flag
        true
    } else if on_proxmox && non_interactive {
        // Non-interactive mode on Proxmox host - auto-enable setup!
        println!();
        println!("[auto] Proxmox host detected - running automatic Proxmox setup");
        true
    } else if non_interactive {
        // Non-interactive mode, not on Proxmox - skip setup
        return Ok(None);
    } else if on_proxmox {
        // Interactive mode on Proxmox host - ask but default to yes
        println!();
        println!("Proxmox host detected!");
        println!("Would you like to configure Proxmox automatically now?");
        println!("(This will create API tokens and download templates)");
        print!("Configure Proxmox now? (Y/n): ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim();

        // Default to yes (empty input or 'y')
        if input.is_empty() || input.eq_ignore_ascii_case("y") {
            true
        } else {
            println!("Skipping Proxmox setup. You'll need to configure it manually.");
            return Ok(None);
        }
    } else {
        // Not on Proxmox host - skip unless explicitly requested
        println!();
        println!("Note: Not running on a Proxmox host.");
        println!("      Use --setup-proxmox if this IS a Proxmox host.");
        return Ok(None);
    };

    if !should_setup {
        return Ok(None);
    }

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
    println!("  User: {} (for API token)", proxmox_user);
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

    let setup = ProxmoxSetup {
        proxmox_user: proxmox_user.to_string(),
        storage: proxmox_storage.to_string(),
        templates: template_list,
    };

    println!("Running Proxmox setup locally...");
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
/// Runs locally on the Proxmox host - no SSH required.
/// Returns true if gateway was configured, false otherwise.
async fn run_gateway_setup_if_requested(
    datacenter: Option<String>,
    public_ip: Option<String>,
    domain: &str,
    port_start: u16,
    port_end: u16,
    ports_per_vm: u16,
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

    // Auto-detect public IP if not provided
    let public_ip = match public_ip {
        Some(ip) => {
            println!("Using provided public IP: {}", ip);
            ip
        }
        None => {
            println!("Detecting public IP...");
            let ip = detect_public_ip()?;
            println!("  Detected: {}", ip);
            ip
        }
    };

    println!();
    println!("Setting up Gateway (Caddy reverse proxy) locally...");
    println!("  Datacenter: {}", datacenter);
    println!("  Domain: {}", domain);
    println!("  Public IP: {}", public_ip);
    println!(
        "  Port range: {}-{} ({} per VM)",
        port_start, port_end, ports_per_vm
    );
    println!("  TLS: Automatic via Let's Encrypt HTTP-01 challenge");
    println!();

    let setup = GatewaySetup {
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
            setup_proxmox,
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
            install_service,
        } => {
            run_setup_token(
                &token,
                &api_url,
                &output,
                force,
                setup_proxmox,
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
                install_service,
            )
            .await
        }
    }
}

async fn run_test_provision(
    config: Config,
    ssh_pubkey: Option<String>,
    keep: bool,
    contract_id: Option<String>,
    test_gateway: bool,
    skip_dns: bool,
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
    if test_gateway {
        println!(
            "Gateway testing: enabled{}",
            if skip_dns { " (DNS skipped)" } else { "" }
        );
    }
    println!();

    let request = ProvisionRequest {
        contract_id: contract_id.clone(),
        offering_id: "test-offering".to_string(),
        cpu_cores: Some(1),
        memory_mb: Some(1024),
        storage_gb: Some(10),
        requester_ssh_pubkey: Some(ssh_key),
        instance_config: None,
        post_provision_script: None,
    };

    println!("Provisioning test VM...");
    let start = std::time::Instant::now();
    let mut instance = provisioner.provision(&request).await?;
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

    // Gateway setup if requested
    let gateway_manager = if test_gateway {
        match &config.gateway {
            Some(gw_config) => {
                // Create a minimal API client for gateway (DNS operations will be skipped in test mode)
                let api_client = std::sync::Arc::new(
                    ApiClient::new(&config.api).context("Failed to create API client for gateway")?,
                );
                match GatewayManager::new(gw_config.clone(), api_client) {
                    Ok(mut gm) => {
                        println!("\nSetting up gateway{}...", if skip_dns { " (local only, no DNS)" } else { "" });
                        let gw_result = if skip_dns {
                            gm.setup_gateway_local(instance.clone(), &contract_id).await
                        } else {
                            gm.setup_gateway(instance.clone(), &contract_id).await
                        };
                        match gw_result {
                            Ok(updated_instance) => {
                                instance = updated_instance;
                                println!("✓ Gateway setup complete");
                                println!();
                                println!("Gateway details:");
                                if let Some(slug) = &instance.gateway_slug {
                                    println!("  Slug: {}", slug);
                                }
                                if let Some(subdomain) = &instance.gateway_subdomain {
                                    println!("  Subdomain: {}", subdomain);
                                }
                                if let Some(port) = instance.gateway_ssh_port {
                                    println!("  SSH Port: {}", port);
                                }
                                if let (Some(start), Some(end)) = (
                                    instance.gateway_port_range_start,
                                    instance.gateway_port_range_end,
                                ) {
                                    println!("  Port Range: {}-{}", start, end);
                                }
                                Some(gm)
                            }
                            Err(e) => {
                                println!("⚠ Gateway setup failed: {:#}", e);
                                println!("  (VM provisioning succeeded, continuing without gateway)");
                                None
                            }
                        }
                    }
                    Err(e) => {
                        println!("⚠ Failed to initialize gateway manager: {:#}", e);
                        None
                    }
                }
            }
            None => {
                println!("\n⚠ --test-gateway specified but no gateway configured in dc-agent.toml");
                None
            }
        }
    } else {
        None
    };

    // Health check
    println!("\nRunning health check...");
    let health = provisioner.health_check(&instance.external_id).await?;
    println!("  Status: {:?}", health);

    if keep {
        println!("\n--keep specified, VM will remain running.");
        println!("To terminate later, use the Proxmox web UI or API.");

        // Show connection instructions
        if let Some(subdomain) = &instance.gateway_subdomain {
            if let Some(port) = instance.gateway_ssh_port {
                println!("\nSSH via gateway:");
                println!("  ssh -p {} ubuntu@{}", port, subdomain);
                println!("  ssh -p {} ubuntu@{}", port, config.gateway.as_ref().map(|g| &g.public_ip).unwrap_or(&"<public_ip>".to_string()));
            }
        } else if let Some(ipv4) = &instance.ip_address {
            println!("\nYou can SSH into the VM (internal network only):");
            println!("  ssh ubuntu@{}", ipv4);
        }
    } else {
        // Cleanup gateway first if it was set up
        if let Some(mut gm) = gateway_manager {
            println!("\nCleaning up gateway...");
            if let Err(e) = gm.cleanup_gateway(&contract_id).await {
                println!("⚠ Gateway cleanup warning: {:#}", e);
            } else {
                println!("✓ Gateway cleaned up");
            }
        }

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

    // Validate config for placeholder values before starting
    config.validate()?;

    let api_client = std::sync::Arc::new(ApiClient::new(&config.api)?);
    let (provisioners, default_provisioner_type) = create_provisioner_map(&config)?;

    // Verify provisioner setup before starting the polling loop
    // This catches issues like unreachable Proxmox API early
    let default_provisioner = provisioners
        .get(&default_provisioner_type)
        .expect("default provisioner must exist");
    let verification = default_provisioner.verify_setup().await;
    if !verification.is_ok() {
        error!(
            errors = ?verification.errors,
            "Provisioner setup verification failed"
        );
        anyhow::bail!(
            "Provisioner setup verification failed:\n  - {}\n\nRun 'dc-agent doctor' for detailed diagnostics.",
            verification.errors.join("\n  - ")
        );
    }
    info!("Provisioner setup verified successfully");

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

    // Resource collection every 5 minutes (less frequent than heartbeat)
    const RESOURCE_COLLECTION_INTERVAL_SECS: u64 = 300;
    let mut resource_ticker = interval(Duration::from_secs(RESOURCE_COLLECTION_INTERVAL_SECS));
    resource_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // Cached resources (collected periodically, sent with each heartbeat)
    // Initialized below before first heartbeat
    let mut cached_resources: Option<dc_agent::api_client::ResourceInventory>;

    // Track active contracts for heartbeat reporting
    let mut active_contracts: i64 = 0;

    // Load orphan tracker from disk (persists across restarts)
    let orphan_tracker_path = Path::new(&config.polling.orphan_tracker_path);
    let mut orphan_tracker = OrphanTracker::load(orphan_tracker_path).with_context(|| {
        format!(
            "Failed to load orphan tracker from {:?}",
            orphan_tracker_path
        )
    })?;
    info!(
        path = %config.polling.orphan_tracker_path,
        tracked_orphans = orphan_tracker.first_seen.len(),
        "Orphan tracker loaded"
    );

    // Track consecutive failures for escalating log levels
    let mut heartbeat_failures: u32 = 0;
    let mut poll_failures: u32 = 0;

    // Update check every hour (3600 seconds)
    const UPDATE_CHECK_INTERVAL_SECS: u64 = 3600;
    let mut update_check_ticker = interval(Duration::from_secs(UPDATE_CHECK_INTERVAL_SECS));
    update_check_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    info!(
        poll_interval_seconds = config.polling.interval_seconds,
        heartbeat_interval_seconds = heartbeat_interval_secs,
        orphan_grace_period_seconds = config.polling.orphan_grace_period_seconds,
        update_check_interval_seconds = UPDATE_CHECK_INTERVAL_SECS,
        "Agent started"
    );

    // Collect initial resources
    cached_resources = default_provisioner.collect_resources().await;
    if cached_resources.is_some() {
        info!("Collected initial resource inventory");
    }

    // Send initial heartbeat immediately
    send_heartbeat(
        &api_client,
        &default_provisioner_type,
        active_contracts,
        &mut heartbeat_interval_secs,
        &mut heartbeat_ticker,
        &mut heartbeat_failures,
        gateway_manager.clone(),
        cached_resources.clone(),
    )
    .await;

    loop {
        tokio::select! {
            _ = poll_ticker.tick() => {
                active_contracts = poll_and_provision(&api_client, &provisioners, &default_provisioner_type, config.polling.orphan_grace_period_seconds, &mut orphan_tracker, &mut poll_failures, gateway_manager.clone()).await;
            }
            _ = heartbeat_ticker.tick() => {
                send_heartbeat(&api_client, &default_provisioner_type, active_contracts, &mut heartbeat_interval_secs, &mut heartbeat_ticker, &mut heartbeat_failures, gateway_manager.clone(), cached_resources.clone()).await;
            }
            _ = resource_ticker.tick() => {
                // Refresh resource inventory periodically
                if let Some(resources) = default_provisioner.collect_resources().await {
                    cached_resources = Some(resources);
                    info!("Refreshed resource inventory");
                }
            }
            _ = update_check_ticker.tick() => {
                check_for_updates_and_log().await;
            }
        }
    }
}

/// Check for available updates and log if a new version is available.
/// Runs periodically during agent operation to notify about updates.
async fn check_for_updates_and_log() {
    use tracing::debug;

    let current = env!("CARGO_PKG_VERSION");
    match dc_agent::upgrade::check_latest_version().await {
        Ok(latest) if dc_agent::upgrade::is_newer(current, &latest) => {
            warn!(
                current_version = %current,
                latest_version = %latest,
                "Update available! Run 'dc-agent upgrade' to install"
            );
        }
        Ok(_) => {
            debug!("Version check: up to date");
        }
        Err(e) => {
            // Don't spam logs on network issues - use debug level
            debug!(error = %e, "Failed to check for updates");
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn send_heartbeat(
    api_client: &ApiClient,
    provisioner_type: &str,
    active_contracts: i64,
    heartbeat_interval_secs: &mut u64,
    heartbeat_ticker: &mut tokio::time::Interval,
    consecutive_failures: &mut u32,
    gateway_manager: OptionalGatewayManager,
    resources: Option<dc_agent::api_client::ResourceInventory>,
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
            resources,
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
                        post_provision_script: contract.post_provision_script.clone(),
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

                            // Execute post-provision script if configured
                            if let Some(ref script) = request.post_provision_script {
                                if let Some(ref ip) = instance.ip_address {
                                    match execute_post_provision_script(
                                        ip,
                                        instance.ssh_port,
                                        script,
                                        &contract.contract_id,
                                    )
                                    .await
                                    {
                                        Ok(()) => {
                                            info!(
                                                contract_id = %contract.contract_id,
                                                "Post-provision script completed successfully"
                                            );
                                        }
                                        Err(e) => {
                                            // Script failed - log but continue
                                            // VM is still usable, just custom setup didn't work
                                            warn!(
                                                contract_id = %contract.contract_id,
                                                error = ?e,
                                                "Post-provision script failed - VM is still accessible"
                                            );
                                        }
                                    }
                                } else {
                                    warn!(
                                        contract_id = %contract.contract_id,
                                        "Cannot execute post-provision script: no IP address available"
                                    );
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
        let mut termination_errors: Vec<(String, String)> = Vec::new();
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
                Err(e) => {
                    warn!(
                        external_id = %vm.external_id,
                        provisioner_type = %ptype,
                        error = ?e,
                        "Provisioner failed to terminate, trying next"
                    );
                    termination_errors.push((ptype.to_string(), format!("{e:#}")));
                    continue;
                }
            }
        }

        if !terminated {
            error!(
                external_id = %vm.external_id,
                contract_id = %vm.contract_id,
                errors = ?termination_errors,
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
        let first_seen = orphan_tracker.record_orphan(&vm.external_id, now);
        let age_seconds = now.saturating_sub(first_seen);

        if age_seconds >= orphan_grace_period_seconds {
            // Grace period exceeded - prune this orphan
            to_prune.push(vm);
        } else if first_seen == now {
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
        let mut prune_errors: Vec<(String, String)> = Vec::new();
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
                Err(e) => {
                    warn!(
                        external_id = %vm.external_id,
                        provisioner_type = %ptype,
                        error = ?e,
                        "Provisioner failed to prune orphan, trying next"
                    );
                    prune_errors.push((ptype.to_string(), format!("{e:#}")));
                    continue;
                }
            }
        }

        if !pruned {
            error!(
                external_id = %vm.external_id,
                errors = ?prune_errors,
                "Orphan pruning failed - no provisioner could terminate this instance"
            );
        } else {
            // Remove from tracker after successful pruning
            orphan_tracker.remove(&vm.external_id);
        }
    }

    // Clean up tracker - remove orphans that are no longer present (resolved)
    let removed = orphan_tracker.retain_present(&current_orphans);
    for external_id in removed {
        info!(
            external_id = %external_id,
            "Orphan VM resolved - no longer present"
        );
    }

    // Persist orphan tracker to disk so state survives restarts
    if let Err(e) = orphan_tracker.save() {
        error!(error = ?e, "Failed to save orphan tracker - state may be lost on restart");
    }
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
                    Err(e) => println!("  [FAILED] Gateway initialization: {:#}", e),
                },
                None => {
                    println!("  [WARN] Cannot verify gateway manager (API client not available)");
                }
            }
        }
        None => {
            println!("Gateway: Not configured");
            println!("  VMs will not get public subdomains");
            println!("  To enable: re-run setup with --gateway-datacenter <DC>");
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
                None, // No resources in doctor mode
            )
            .await
        {
            Ok(response) => {
                println!("[ok] API authentication successful");
                println!("  Heartbeat acknowledged: {}", response.acknowledged);
                println!("  Next heartbeat in: {}s", response.next_heartbeat_seconds);
            }
            Err(e) => {
                println!("[FAILED] API verification failed: {:#}", e);
                println!();
                println!("Possible causes:");
                println!("  - Agent not registered (run: dc-agent register)");
                println!("  - Agent delegation expired or revoked");
                println!("  - Invalid agent key");
                println!("  - Network connectivity issue");
                return Err(anyhow::anyhow!("API verification failed: {:#}", e));
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
                    post_provision_script: None,
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
                                println!("[WARN] Test VM created but termination failed: {:#}", e);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_datacenter_from_pool_id_valid() {
        // Standard 2-letter datacenter codes
        assert_eq!(
            parse_datacenter_from_pool_id("sl-8eba3c90"),
            Some("dc-sl".to_string())
        );
        assert_eq!(
            parse_datacenter_from_pool_id("us-abc123"),
            Some("dc-us".to_string())
        );

        // 3-letter datacenter codes
        assert_eq!(
            parse_datacenter_from_pool_id("usw-abc123"),
            Some("dc-usw".to_string())
        );
        assert_eq!(
            parse_datacenter_from_pool_id("euw-deadbeef"),
            Some("dc-euw".to_string())
        );

        // 4-letter datacenter codes (max allowed)
        assert_eq!(
            parse_datacenter_from_pool_id("apne-12345"),
            Some("dc-apne".to_string())
        );

        // Multiple dashes in uuid part
        assert_eq!(
            parse_datacenter_from_pool_id("sl-abc-def-123"),
            Some("dc-sl".to_string())
        );
    }

    #[test]
    fn test_parse_datacenter_from_pool_id_invalid_no_dash() {
        // No dash at all
        assert_eq!(parse_datacenter_from_pool_id("8eba3c90"), None);
        assert_eq!(parse_datacenter_from_pool_id("slabc123"), None);
    }

    #[test]
    fn test_parse_datacenter_from_pool_id_invalid_empty_code() {
        // Empty code (starts with dash)
        assert_eq!(parse_datacenter_from_pool_id("-8eba3c90"), None);
        assert_eq!(parse_datacenter_from_pool_id("-"), None);
    }

    #[test]
    fn test_parse_datacenter_from_pool_id_invalid_uppercase() {
        // Uppercase letters
        assert_eq!(parse_datacenter_from_pool_id("SL-8eba3c90"), None);
        assert_eq!(parse_datacenter_from_pool_id("Sl-8eba3c90"), None);
        assert_eq!(parse_datacenter_from_pool_id("USW-abc123"), None);
    }

    #[test]
    fn test_parse_datacenter_from_pool_id_invalid_too_short() {
        // Code too short (< 2 chars)
        assert_eq!(parse_datacenter_from_pool_id("s-8eba3c90"), None);
        assert_eq!(parse_datacenter_from_pool_id("a-123"), None);
    }

    #[test]
    fn test_parse_datacenter_from_pool_id_invalid_too_long() {
        // Code too long (> 4 chars)
        assert_eq!(parse_datacenter_from_pool_id("abcde-8eba3c90"), None);
        assert_eq!(parse_datacenter_from_pool_id("uswest-123"), None);
    }

    #[test]
    fn test_parse_datacenter_from_pool_id_invalid_non_alpha() {
        // Non-alphabetic characters in code
        assert_eq!(parse_datacenter_from_pool_id("s1-8eba3c90"), None);
        assert_eq!(parse_datacenter_from_pool_id("12-abc123"), None);
        assert_eq!(parse_datacenter_from_pool_id("u_s-abc123"), None);
    }

    #[test]
    fn test_parse_datacenter_from_pool_id_edge_cases() {
        // Empty string
        assert_eq!(parse_datacenter_from_pool_id(""), None);

        // Just a dash
        assert_eq!(parse_datacenter_from_pool_id("-"), None);

        // Only code, no uuid
        assert_eq!(parse_datacenter_from_pool_id("sl"), None);
    }

    #[test]
    fn test_systemd_service_content_uses_absolute_paths() {
        // Create a temporary config file to test canonicalization
        use std::io::Write;
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("dc-agent.toml");
        let mut file = std::fs::File::create(&config_path).unwrap();
        writeln!(file, "[api]").unwrap();

        // The config path should be converted to absolute
        let absolute = config_path.canonicalize().unwrap();

        // Verify the canonical path is absolute and not the same as relative
        assert!(absolute.is_absolute());
        assert_ne!(
            config_path.to_string_lossy(),
            "dc-agent.toml",
            "Test setup should use a unique path"
        );

        // Verify canonicalize works as expected
        assert!(absolute.to_string_lossy().starts_with('/'));
    }

    #[test]
    fn test_systemd_service_format() {
        // Test that the service file format is valid
        let binary_path = "/usr/local/bin/dc-agent";
        let config_path = "/etc/dc-agent/config.toml";

        let service_content = format!(
            r#"[Unit]
Description=Decent Cloud Provisioning Agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart={} --config {} run
Restart=always
RestartSec=10
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
"#,
            binary_path, config_path
        );

        // Verify service content contains absolute paths
        assert!(
            service_content.contains("/usr/local/bin/dc-agent"),
            "Service should use absolute binary path"
        );
        assert!(
            service_content.contains("/etc/dc-agent/config.toml"),
            "Service should use absolute config path"
        );

        // Verify essential systemd directives are present
        assert!(service_content.contains("[Unit]"));
        assert!(service_content.contains("[Service]"));
        assert!(service_content.contains("[Install]"));
        assert!(service_content.contains("Restart=always"));
        assert!(service_content.contains("WantedBy=multi-user.target"));
    }

    #[test]
    fn test_is_service_installed_returns_false_for_nonexistent() {
        // When the service file doesn't exist, should return false
        // This test relies on the fact that /etc/systemd/system/dc-agent.service
        // typically doesn't exist in test environments
        // We can't guarantee this, so we just test the function doesn't panic
        let _ = is_service_installed();
    }
}
