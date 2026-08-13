use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::PathBuf;

use crate::api_client::{register_gateway, setup_agent};
use crate::host::{
    install_systemd_service, is_proxmox_host, is_service_installed, parse_datacenter_from_pool_id,
};
use crate::registration::{default_agent_dir, generate_agent_keypair};
use crate::setup::{detect_public_ip, GatewaySetup};

#[derive(Subcommand)]
pub enum SetupProvisioner {
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

        // === Optional: Gateway setup (Caddy reverse proxy with DNS-01 wildcard TLS) ===
        /// Datacenter ID for gateway (2-20 chars [a-z0-9-]). Enables gateway setup.
        /// Generate with: openssl rand -hex 4
        #[arg(long)]
        gateway_dc_id: Option<String>,

        /// Host's public IPv4 address (auto-detected if not provided)
        #[arg(long)]
        gateway_public_ip: Option<String>,

        /// Base domain for gateway subdomains (default: decent-cloud.org)
        #[arg(long, default_value = "decent-cloud.org")]
        gateway_domain: String,

        /// Gateway DNS prefix (default: gw, use dev-gw for dev)
        #[arg(long, default_value = "gw")]
        gateway_gw_prefix: String,

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
    // Use: dc-agent setup token --gateway-dc-id <DC_ID>
    // acme-dns credentials are obtained automatically from the central API.
}

/// Setup agent using a one-time setup token.
#[allow(clippy::too_many_arguments)]
pub async fn run_setup_token(
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
    gateway_dc_id: Option<String>,
    gateway_public_ip: Option<String>,
    gateway_domain: &str,
    gateway_gw_prefix: &str,
    gateway_port_start: u16,
    gateway_port_end: u16,
    gateway_ports_per_vm: u16,
    // Service installation
    install_service: Option<bool>,
) -> Result<()> {
    use crate::geolocation::{country_to_region, detect_country, region_display_name};
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

    // Step 5b: Auto-derive gateway dc_id from pool_id if not provided
    // Pool ID format: "sl-8eba3c90" -> dc_id "dc-sl"
    let gateway_dc_id = gateway_dc_id.or_else(|| {
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
            Some(dc_id) => {
                println!();
                println!(
                    "[auto] Gateway enabled: dc_id={} (derived from pool {})",
                    dc_id, response.pool_id
                );
                Some(dc_id)
            }
            None => {
                println!();
                println!(
                    "[warn] Cannot derive gateway dc_id: pool_id '{}' has invalid format (expected 'code-uuid')",
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

    // Step 7: Register gateway with central API for acme-dns credentials
    let gw_registration = if gateway_dc_id.is_some() {
        let dc_id = gateway_dc_id.as_deref().unwrap();
        println!();
        println!("Registering gateway with central API for TLS credentials...");
        match register_gateway(api_url, &key_path.to_string_lossy(), dc_id).await {
            Ok(reg) => {
                println!("[ok] Gateway registered with acme-dns");
                println!("  acme-dns subdomain: {}", reg.acme_dns_subdomain);
                Some(reg)
            }
            Err(e) => {
                anyhow::bail!(
                    "Gateway registration failed: {:#}\n\
                     Ensure the API server has ACME_DNS_SERVER_URL configured.",
                    e
                );
            }
        }
    } else {
        None
    };

    // Step 8: Run gateway setup locally on the host
    let gateway_configured = run_gateway_setup_if_requested(
        gateway_dc_id,
        gateway_public_ip,
        gw_registration,
        gateway_domain,
        gateway_gw_prefix,
        gateway_port_start,
        gateway_port_end,
        gateway_ports_per_vm,
        output,
    )
    .await?;

    // Step 9: Install systemd service
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
            println!("  To enable gateway, re-run setup with --gateway-dc-id <DC>");
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
            println!("  To enable gateway, re-run setup with --gateway-dc-id <DC>");
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

/// Optionally run Proxmox setup based on CLI args or auto-detection.
/// This runs locally on the Proxmox host - no SSH required.
/// Returns Some(ProxmoxConfig) if setup was completed, None otherwise.
async fn run_proxmox_setup_if_requested(
    setup_proxmox: bool,
    proxmox_user: &str,
    proxmox_storage: &str,
    proxmox_templates: &str,
    non_interactive: bool,
) -> Result<Option<crate::config::ProxmoxConfig>> {
    use crate::config::ProxmoxConfig;
    use crate::setup::proxmox::{OsTemplate, ProxmoxSetup};

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
#[allow(clippy::too_many_arguments)]
async fn run_gateway_setup_if_requested(
    dc_id: Option<String>,
    public_ip: Option<String>,
    acme_dns: Option<crate::api_client::GatewayRegistration>,
    domain: &str,
    gw_prefix: &str,
    port_start: u16,
    port_end: u16,
    ports_per_vm: u16,
    config_path: &std::path::Path,
) -> Result<bool> {
    use std::io::Write;

    // Check if gateway setup was requested
    let dc_id = match dc_id {
        Some(id) => id,
        None => {
            // Gateway not requested
            return Ok(false);
        }
    };

    let acme_dns = acme_dns
        .context("acme-dns credentials required for gateway setup but registration failed")?;

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
    println!("  DC ID: {}", dc_id);
    println!("  Public IP: {}", public_ip);
    println!("  Wildcard: *.{}.{}.{}", dc_id, gw_prefix, domain);
    println!(
        "  Port range: {}-{} ({} per VM)",
        port_start, port_end, ports_per_vm
    );
    println!("  TLS: Per-provider wildcard cert via DNS-01 (acme-dns)");
    println!();

    let setup = GatewaySetup {
        dc_id: dc_id.clone(),
        public_ip: public_ip.clone(),
        domain: domain.to_string(),
        gw_prefix: gw_prefix.to_string(),
        acme_dns_server_url: acme_dns.acme_dns_server_url,
        acme_dns_username: acme_dns.acme_dns_username,
        acme_dns_password: acme_dns.acme_dns_password,
        acme_dns_subdomain: acme_dns.acme_dns_subdomain,
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
    println!("Gateway configured successfully!");
    println!("  Gateway config appended to: {}", config_path.display());

    Ok(true)
}

pub async fn run(provisioner: SetupProvisioner) -> Result<()> {
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
            gateway_dc_id,
            gateway_public_ip,
            gateway_domain,
            gateway_gw_prefix,
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
                gateway_dc_id,
                gateway_public_ip,
                &gateway_domain,
                &gateway_gw_prefix,
                gateway_port_start,
                gateway_port_end,
                gateway_ports_per_vm,
                install_service,
            )
            .await
        }
    }
}
