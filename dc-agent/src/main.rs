use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dc_agent::{
    api_client::{ApiClient, ReconcileResponse},
    config::{Config, ProvisionerConfig},
    provisioner::{
        manual::ManualProvisioner, proxmox::ProxmoxProvisioner, script::ScriptProvisioner,
        ProvisionRequest, Provisioner,
    },
    registration::{
        default_agent_dir, generate_agent_keypair, load_agent_pubkey, load_provider_identity,
        register_agent_with_api, DEFAULT_PERMISSIONS,
    },
    setup::{proxmox::OsTemplate, ProxmoxSetup},
};
use dcc_common::DccIdentity;
use std::collections::HashMap;
use std::path::PathBuf;

/// Map of provisioner type name to provisioner instance
type ProvisionerMap = HashMap<String, Box<dyn Provisioner>>;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

#[derive(Parser)]
#[command(name = "dc-agent")]
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
    /// Initialize a new agent keypair (use 'setup' for automatic registration)
    Init {
        /// Output directory for keys (default: ~/.dc-agent)
        #[arg(long)]
        output: Option<PathBuf>,

        /// Force overwrite existing keys
        #[arg(long, default_value = "false")]
        force: bool,
    },
    /// Register agent with API (use 'setup' for automatic registration)
    Register {
        /// Provider's main public key (hex)
        #[arg(long)]
        provider_pubkey: String,

        /// Provider's main secret key (hex) - for signing the delegation
        #[arg(long)]
        provider_secret_key: Option<String>,

        /// API endpoint (default: https://api.decent-cloud.org)
        #[arg(long, default_value = "https://api.decent-cloud.org")]
        api_endpoint: String,

        /// Human-readable label for this agent
        #[arg(long)]
        label: Option<String>,

        /// Agent keys directory (default: ~/.dc-agent)
        #[arg(long)]
        keys_dir: Option<PathBuf>,
    },
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
        provisioner: SetupProvisioner,
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
    /// Set up Proxmox VE provisioner (creates templates and API token)
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

        /// Decent Cloud API endpoint
        #[arg(long, default_value = "https://api.decent-cloud.org")]
        api_endpoint: String,

        /// Provider identity name or path (default: auto-detect from ~/.dcc/identity/)
        #[arg(long)]
        identity: Option<String>,

        /// Skip agent registration (for offline setup)
        #[arg(long, default_value = "false")]
        skip_registration: bool,
    },
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
        Commands::Init { output, force } => run_init(output, force),
        Commands::Register {
            provider_pubkey,
            provider_secret_key,
            api_endpoint,
            label,
            keys_dir,
        } => {
            run_register(
                provider_pubkey,
                provider_secret_key,
                api_endpoint,
                label,
                keys_dir,
            )
            .await
        }
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
        Commands::Setup { provisioner } => run_setup(provisioner).await,
    }
}

/// Initialize a new agent keypair.
fn run_init(output: Option<PathBuf>, force: bool) -> Result<()> {
    println!("dc-agent init");
    println!("=============\n");

    let agent_dir = match output {
        Some(path) => path,
        None => default_agent_dir()?,
    };
    let private_key_path = agent_dir.join("agent.key");

    // Check if key exists without force flag
    if private_key_path.exists() && !force {
        anyhow::bail!(
            "Agent key already exists at {}. Use --force to overwrite.",
            private_key_path.display()
        );
    }

    println!("Generating new Ed25519 keypair...");
    let (key_path, pubkey_hex) = generate_agent_keypair(&agent_dir, force)?;

    println!("Agent keypair generated successfully\n");
    println!("Keys directory: {}", agent_dir.display());
    println!("Private key: {}", key_path.display());
    println!("Public key:  {}", agent_dir.join("agent.pub").display());
    println!();
    println!("Agent public key (hex): {}", pubkey_hex);
    println!();
    println!("Next steps:");
    println!("1. Register this agent with your provider identity:");
    println!("   dc-agent register --provider-pubkey <YOUR_PROVIDER_PUBKEY> --provider-secret-key <YOUR_PROVIDER_SECRET>");
    println!();
    println!("Or provide the provider secret key interactively for security.");

    Ok(())
}

/// Register agent with the Decent Cloud API.
async fn run_register(
    provider_pubkey: String,
    provider_secret_key: Option<String>,
    api_endpoint: String,
    label: Option<String>,
    keys_dir: Option<PathBuf>,
) -> Result<()> {
    println!("dc-agent register");
    println!("=================\n");

    let agent_dir = match keys_dir.as_deref() {
        Some(path) => path.to_path_buf(),
        None => default_agent_dir()?,
    };
    let private_key_path = agent_dir.join("agent.key");
    let agent_pubkey_hex = load_agent_pubkey(keys_dir.as_deref())?;

    println!("Agent public key: {}", agent_pubkey_hex);
    println!("Provider public key: {}", provider_pubkey);
    println!("API endpoint: {}", api_endpoint);
    if let Some(lbl) = &label {
        println!("Label: {}", lbl);
    }
    println!();

    // Get provider secret key (prompt if not provided)
    let provider_secret = match provider_secret_key {
        Some(key) => key,
        None => rpassword::prompt_password("Provider secret key (hex): ")?,
    };

    let provider_secret_bytes = hex::decode(&provider_secret)
        .map_err(|e| anyhow::anyhow!("Invalid provider secret key: {}", e))?;

    if provider_secret_bytes.len() != 32 {
        anyhow::bail!("Provider secret key must be 32 bytes");
    }

    // Create provider identity and verify it matches the provided pubkey
    let provider_identity = DccIdentity::new_signing_from_bytes(&provider_secret_bytes)?;
    let derived_pubkey = hex::encode(provider_identity.to_bytes_verifying());

    if derived_pubkey != provider_pubkey {
        anyhow::bail!(
            "Provider secret key does not match the provided public key.\n\
             Expected: {}\n\
             Derived:  {}",
            provider_pubkey,
            derived_pubkey
        );
    }

    println!("Signing delegation and registering with API...");
    register_agent_with_api(
        &provider_identity,
        &agent_pubkey_hex,
        &api_endpoint,
        label.as_deref(),
    )
    .await?;

    println!("\nAgent registered successfully!\n");
    println!("Agent public key: {}", agent_pubkey_hex);
    println!("Permissions: {}", DEFAULT_PERMISSIONS.join(", "));
    println!();
    println!("Next steps:");
    println!("1. Update your dc-agent.toml with:");
    println!("   [api]");
    println!("   endpoint = \"{}\"", api_endpoint);
    println!("   provider_pubkey = \"{}\"", provider_pubkey);
    println!("   agent_secret_key = \"{}\"", private_key_path.display());
    println!();
    println!("2. Run: dc-agent doctor");
    println!("3. Run: dc-agent run");

    Ok(())
}

async fn run_setup(provisioner: SetupProvisioner) -> Result<()> {
    match provisioner {
        SetupProvisioner::Proxmox {
            host,
            ssh_port,
            ssh_user,
            proxmox_user,
            storage,
            templates,
            output,
            api_endpoint,
            identity,
            skip_registration,
        } => {
            println!("Decent Cloud Agent - Proxmox Setup");
            println!("===================================\n");

            // Step 1: Load provider identity (if not skipping registration)
            let provider_identity = if skip_registration {
                println!("[skip] Agent registration skipped (--skip-registration)");
                None
            } else {
                match load_provider_identity(identity.as_deref()) {
                    Ok(id) => {
                        println!(
                            "[ok] Provider identity loaded: {}",
                            hex::encode(id.to_bytes_verifying())
                        );
                        Some(id)
                    }
                    Err(e) => {
                        // Check if this is "no identities" vs "multiple identities" or other error
                        let err_str = e.to_string();
                        if err_str.contains("Multiple identities") {
                            // User has identities but we can't pick - this is a user error, fail
                            return Err(e);
                        } else if err_str.contains("No identities found") {
                            // No identity at all - proceed without registration
                            println!("[skip] {}", e);
                            println!("       Proxmox setup will continue, but agent won't be registered.");
                            println!();
                            None
                        } else {
                            // Other error (e.g., corrupted key) - fail
                            return Err(e);
                        }
                    }
                }
            };

            let provider_pubkey = provider_identity
                .as_ref()
                .map(|id| hex::encode(id.to_bytes_verifying()))
                .unwrap_or_else(|| "YOUR_PROVIDER_PUBKEY_HERE".to_string());

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

            // Step 2: Generate agent keypair and register (if identity available)
            let agent_key_path = if let Some(ref provider_id) = provider_identity {
                println!();
                println!("Agent Registration");
                println!("------------------");

                let agent_dir = default_agent_dir()?;
                let (key_path, agent_pubkey) = generate_agent_keypair(&agent_dir, false)?;
                println!("[ok] Agent keypair: {}", key_path.display());

                // Register with API
                let label = Some(format!("proxmox-{}", host));
                let registration_ok = match register_agent_with_api(
                    provider_id,
                    &agent_pubkey,
                    &api_endpoint,
                    label.as_deref(),
                )
                .await
                {
                    Ok(()) => {
                        println!("[ok] Agent registered with API");
                        true
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        println!("[FAILED] Agent registration failed: {}", e);
                        println!();
                        if err_str.contains("404") || err_str.contains("not found") {
                            println!("       Your identity may not be registered as a provider.");
                            println!("       Register as provider first: dc provider register");
                        } else if err_str.contains("401") || err_str.contains("403") {
                            println!("       Authentication failed. Check your identity key.");
                        } else {
                            println!("       Check your network connection and API endpoint.");
                        }
                        println!();
                        println!(
                            "       Config will be written, but you must register before running."
                        );
                        println!("       After fixing the issue, run: dc-agent register --provider-pubkey {}", hex::encode(provider_id.to_bytes_verifying()));
                        false
                    }
                };

                // Return key path and registration status
                Some((key_path.to_string_lossy().to_string(), registration_ok))
            } else {
                None
            };

            // Step 3: Write config file
            result.write_config(
                &output,
                &api_endpoint,
                &provider_pubkey,
                agent_key_path.as_ref().map(|(p, _)| p.as_str()),
            )?;

            let registration_failed = agent_key_path.as_ref().map(|(_, ok)| !ok).unwrap_or(false);

            println!();
            println!("===================================");
            if registration_failed {
                println!("Setup incomplete - agent registration failed!");
            } else {
                println!("Setup complete!");
            }
            println!();
            println!("Configuration: {}", output.display());

            if provider_identity.is_some() {
                println!();
                if registration_failed {
                    println!("After fixing registration, run:");
                    println!("  dc-agent register --provider-pubkey {}", provider_pubkey);
                    println!();
                    println!("Then verify with:");
                } else {
                    println!("Next steps:");
                }
                println!("  dc-agent --config {} doctor", output.display());
                println!("  dc-agent --config {} run", output.display());
            } else {
                println!();
                println!("Next steps:");
                println!("  1. Create provider identity: dc identity new provider");
                println!(
                    "  2. Re-run setup: dc-agent setup proxmox --host {} --identity provider",
                    host
                );
            }

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

    let api_client = ApiClient::new(&config.api)?;
    let (provisioners, default_provisioner_type) = create_provisioner_map(&config)?;

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

    // Track consecutive failures for escalating log levels
    let mut heartbeat_failures: u32 = 0;
    let mut poll_failures: u32 = 0;

    info!(
        poll_interval_seconds = config.polling.interval_seconds,
        heartbeat_interval_seconds = heartbeat_interval_secs,
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
    )
    .await;

    loop {
        tokio::select! {
            _ = poll_ticker.tick() => {
                active_contracts = poll_and_provision(&api_client, &provisioners, &default_provisioner_type, &mut poll_failures).await;
            }
            _ = heartbeat_ticker.tick() => {
                send_heartbeat(&api_client, &default_provisioner_type, active_contracts, &mut heartbeat_interval_secs, &mut heartbeat_ticker, &mut heartbeat_failures).await;
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
) {
    match api_client
        .send_heartbeat(
            Some(env!("CARGO_PKG_VERSION")),
            Some(provisioner_type),
            None,
            active_contracts,
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
                    error = %e,
                    consecutive_failures = *consecutive_failures,
                    "HEARTBEAT FAILURE: Agent cannot reach API server! Check network connectivity."
                );
            } else {
                warn!(
                    error = %e,
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
    consecutive_failures: &mut u32,
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
                                    error = %e,
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

                    // Log if offering has custom provisioner_config (not yet used)
                    if contract.provisioner_config.is_some() {
                        warn!(
                            contract_id = %contract.contract_id,
                            "Offering has provisioner_config but per-contract config override is not yet implemented"
                        );
                    }

                    // Parse instance_config if present - log warning if malformed
                    let instance_config: Option<serde_json::Value> = match &contract.instance_config
                    {
                        Some(s) => match serde_json::from_str(s) {
                            Ok(v) => Some(v),
                            Err(e) => {
                                warn!(
                                    contract_id = %contract.contract_id,
                                    error = %e,
                                    raw_config = %s,
                                    "Invalid instance_config JSON, ignoring"
                                );
                                None
                            }
                        },
                        None => None,
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
                            error = %e,
                            "Failed to report provisioning started, continuing anyway"
                        );
                    }

                    match provisioner.provision(&request).await {
                        Ok(instance) => {
                            info!(
                                contract_id = %contract.contract_id,
                                external_id = %instance.external_id,
                                ip_address = ?instance.ip_address,
                                "Provisioned successfully"
                            );
                            if let Err(e) = api_client
                                .report_provisioned(&contract.contract_id, &instance)
                                .await
                            {
                                // This is a critical error - VM was created but API doesn't know
                                error!(
                                    contract_id = %contract.contract_id,
                                    external_id = %instance.external_id,
                                    ip_address = ?instance.ip_address,
                                    error = %e,
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
                    error = %e,
                    consecutive_failures = *consecutive_failures,
                    "POLL FAILURE: Cannot fetch contracts from API server! Check network connectivity."
                );
            } else {
                warn!(
                    error = %e,
                    consecutive_failures = *consecutive_failures,
                    "Failed to fetch pending contracts"
                );
            }
            return 0;
        }
    }

    // Reconcile running instances - handles expired, cancelled, and orphan VMs
    reconcile_instances(api_client, provisioners).await;

    active_count
}

/// Reconcile running instances with the API.
/// Reports running VMs, terminates expired/cancelled contracts, warns about orphans.
/// Collects instances from ALL provisioners and tries to terminate via the appropriate one.
async fn reconcile_instances(
    api_client: &ApiClient,
    provisioners: &ProvisionerMap,
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
                    error = %e,
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
            warn!(error = %e, "Failed to reconcile with API");
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
                    if let Err(e) = api_client.report_terminated(&vm.contract_id).await {
                        error!(
                            contract_id = %vm.contract_id,
                            error = %e,
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

    // Warn about orphan VMs
    for vm in &response.unknown {
        warn!(
            external_id = %vm.external_id,
            message = %vm.message,
            "Orphan VM detected - no matching contract"
        );
    }
}

/// Create a single provisioner from config
fn create_provisioner_from_config(
    prov_config: &ProvisionerConfig,
) -> Result<Box<dyn Provisioner>> {
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
fn create_provisioner_map(
    config: &Config,
) -> Result<(ProvisionerMap, String)> {
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

    let provisioner_type = config.provisioner.type_name();

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
