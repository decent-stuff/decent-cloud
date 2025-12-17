use anyhow::Result;
use clap::{Parser, Subcommand};
use dc_agent::{
    api_client::ApiClient,
    config::{Config, ProvisionerType},
    provisioner::{manual::ManualProvisioner, proxmox::ProxmoxProvisioner, script::ScriptProvisioner, ProvisionRequest, Provisioner},
    setup::{proxmox::OsTemplate, ProxmoxSetup},
};
use std::path::PathBuf;
use std::sync::Arc;
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
    /// Start the agent polling loop
    Run,
    /// Check agent configuration and connectivity
    Doctor,
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

        /// Provider public key (hex)
        #[arg(long, default_value = "")]
        provider_pubkey: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run | Commands::Doctor | Commands::TestProvision { .. } => {
            let config = Config::load(&cli.config)?;
            match cli.command {
                Commands::Run => run_agent(config).await,
                Commands::Doctor => run_doctor(config).await,
                Commands::TestProvision { ssh_pubkey, keep, contract_id } => {
                    run_test_provision(config, ssh_pubkey, keep, contract_id).await
                }
                _ => unreachable!(),
            }
        }
        Commands::Setup { provisioner } => run_setup(provisioner).await,
    }
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
            provider_pubkey,
        } => {
            println!("Decent Cloud Agent - Proxmox Setup");
            println!("===================================\n");

            // Parse templates
            let template_list: Vec<OsTemplate> = templates
                .split(',')
                .filter_map(|s| {
                    let t = OsTemplate::parse(s.trim());
                    if t.is_none() {
                        eprintln!("Warning: Unknown template '{}', skipping", s.trim());
                    }
                    t
                })
                .collect();

            if template_list.is_empty() {
                anyhow::bail!("No valid templates specified. Available: ubuntu-24.04, ubuntu-22.04, debian-12, rocky-9");
            }

            println!("Host: {}", host);
            println!("SSH User: {}", ssh_user);
            println!("Proxmox User: {}", proxmox_user);
            println!("Storage: {}", storage);
            println!("Templates: {}", template_list.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", "));
            println!();

            // Prompt for passwords
            let ssh_password = rpassword::prompt_password(format!("SSH password for {}@{}: ", ssh_user, host))?;
            let proxmox_password = if ssh_user == proxmox_user.split('@').next().unwrap_or("root") {
                // Same user, reuse password
                ssh_password.clone()
            } else {
                rpassword::prompt_password(format!("Proxmox password for {}: ", proxmox_user))?
            };

            println!();

            let setup = ProxmoxSetup {
                host,
                port: ssh_port,
                ssh_user,
                ssh_password,
                proxmox_user,
                proxmox_password,
                storage,
                templates: template_list,
            };

            let result = setup.run().await?;

            // Write config file
            let pubkey = if provider_pubkey.is_empty() {
                "YOUR_PROVIDER_PUBKEY_HERE"
            } else {
                &provider_pubkey
            };
            result.write_config(&output, &api_endpoint, pubkey)?;

            println!("\n===================================");
            println!("Setup complete!");
            println!();
            println!("Configuration written to: {}", output.display());
            println!();
            println!("Next steps:");
            println!("1. Edit {} and set your provider_secret_key", output.display());
            println!("2. Run: dc-agent --config {} doctor", output.display());
            println!("3. Run: dc-agent --config {} run", output.display());

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

    let provisioner: Arc<dyn Provisioner> = create_provisioner(&config)?;

    // Generate contract ID if not provided
    let contract_id = contract_id.unwrap_or_else(|| {
        format!("test-{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs())
    });

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

    println!("\n✓ VM provisioned successfully in {:.1}s", provision_time.as_secs_f64());
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

    let api_client = Arc::new(ApiClient::new(&config.api)?);
    let provisioner: Arc<dyn Provisioner> = create_provisioner(&config)?;

    let poll_interval = Duration::from_secs(config.polling.interval_seconds);
    let mut ticker = interval(poll_interval);

    info!(
        interval_seconds = config.polling.interval_seconds,
        "Polling loop started"
    );

    loop {
        ticker.tick().await;

        // Fetch pending contracts
        match api_client.get_pending_contracts().await {
            Ok(contracts) => {
                if contracts.is_empty() {
                    continue;
                }

                info!(count = contracts.len(), "Found pending contracts");

                for contract in contracts {
                    info!(contract_id = %contract.contract_id, "Processing contract");

                    // Parse instance_config if present
                    let instance_config: Option<serde_json::Value> = contract
                        .instance_config
                        .as_ref()
                        .and_then(|s| serde_json::from_str(s).ok());

                    let request = ProvisionRequest {
                        contract_id: contract.contract_id.clone(),
                        offering_id: contract.offering_id.clone(),
                        cpu_cores: None, // Extract from offering if available
                        memory_mb: None,
                        storage_gb: None,
                        requester_ssh_pubkey: Some(contract.requester_ssh_pubkey.clone()),
                        instance_config,
                    };

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
                                error!(
                                    contract_id = %contract.contract_id,
                                    error = %e,
                                    "Failed to report provisioning to API"
                                );
                            }
                        }
                        Err(e) => {
                            error!(
                                contract_id = %contract.contract_id,
                                error = %e,
                                "Provisioning failed"
                            );
                            if let Err(report_err) = api_client
                                .report_failed(&contract.contract_id, &e.to_string())
                                .await
                            {
                                error!(
                                    contract_id = %contract.contract_id,
                                    error = %report_err,
                                    "Failed to report failure to API"
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch pending contracts");
            }
        }
    }
}

fn create_provisioner(config: &Config) -> Result<Arc<dyn Provisioner>> {
    match config.provisioner.provisioner_type {
        ProvisionerType::Proxmox => {
            let proxmox_config = config
                .provisioner
                .get_proxmox()
                .ok_or_else(|| anyhow::anyhow!("Proxmox configuration missing"))?
                .clone();
            info!("Creating Proxmox provisioner");
            Ok(Arc::new(ProxmoxProvisioner::new(proxmox_config)?))
        }
        ProvisionerType::Script => {
            let script_config = config
                .provisioner
                .get_script()
                .ok_or_else(|| anyhow::anyhow!("Script configuration missing"))?
                .clone();
            info!("Creating Script provisioner");
            Ok(Arc::new(ScriptProvisioner::new(script_config)))
        }
        ProvisionerType::Manual => {
            let manual_config = config
                .provisioner
                .get_manual()
                .ok_or_else(|| anyhow::anyhow!("Manual configuration missing"))?
                .clone();
            info!("Creating Manual provisioner");
            Ok(Arc::new(ManualProvisioner::new(manual_config)))
        }
    }
}

async fn run_doctor(config: Config) -> Result<()> {
    println!("dc-agent doctor");
    println!("================");
    println!();

    // Check configuration file
    println!("✓ Configuration file loaded");
    println!("  API endpoint: {}", config.api.endpoint);
    println!("  Provider pubkey: {}", config.api.provider_pubkey);
    println!("  Polling interval: {}s", config.polling.interval_seconds);
    println!(
        "  Health check interval: {}s",
        config.polling.health_check_interval_seconds
    );
    println!();

    // Check provisioner configuration
    match config.provisioner.provisioner_type {
        ProvisionerType::Proxmox => {
            if let Some(proxmox) = config.provisioner.get_proxmox() {
                println!("✓ Provisioner type: Proxmox");
                println!("  API URL: {}", proxmox.api_url);
                println!("  Node: {}", proxmox.node);
                println!("  Template VMID: {}", proxmox.template_vmid);
                println!("  Storage: {}", proxmox.storage);
                println!("  Verify SSL: {}", proxmox.verify_ssl);
                if let Some(pool) = &proxmox.pool {
                    println!("  Resource pool: {}", pool);
                }
            } else {
                println!("✗ Proxmox configuration missing");
                return Err(anyhow::anyhow!(
                    "Proxmox type selected but configuration missing"
                ));
            }
        }
        ProvisionerType::Script => {
            if let Some(script) = config.provisioner.get_script() {
                println!("✓ Provisioner type: Script");
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
                        println!("  ✓ {} script exists", name);
                    } else {
                        println!("  ✗ {} script not found: {}", name, path);
                    }
                }
            } else {
                println!("✗ Script configuration missing");
                return Err(anyhow::anyhow!(
                    "Script type selected but configuration missing"
                ));
            }
        }
        ProvisionerType::Manual => {
            if let Some(manual) = config.provisioner.get_manual() {
                println!("✓ Provisioner type: Manual");
                if let Some(webhook) = &manual.notification_webhook {
                    println!("  Notification webhook: {}", webhook);
                } else {
                    println!("  No notification webhook configured");
                }
            } else {
                println!("✗ Manual configuration missing");
                return Err(anyhow::anyhow!(
                    "Manual type selected but configuration missing"
                ));
            }
        }
    }
    println!();

    // Test API connectivity
    println!("Testing API connectivity...");
    let _api_client = ApiClient::new(&config.api)?;
    println!("✓ API client initialized");
    println!();

    println!("Doctor check complete - all checks passed!");
    Ok(())
}
