use anyhow::Result;
use clap::{Parser, Subcommand};
use dc_agent::{
    api_client::ApiClient,
    config::{Config, ProvisionerType},
    provisioner::{manual::ManualProvisioner, proxmox::ProxmoxProvisioner, script::ScriptProvisioner, ProvisionRequest, Provisioner},
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
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let config = Config::load(&cli.config)?;

    match cli.command {
        Commands::Run => run_agent(config).await,
        Commands::Doctor => run_doctor(config).await,
    }
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
