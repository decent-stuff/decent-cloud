use anyhow::Result;
use clap::{Parser, Subcommand};
use dc_agent::{
    api_client::ApiClient,
    config::{Config, ProvisionerType},
    provisioner::{
        manual::ManualProvisioner, proxmox::ProxmoxProvisioner, script::ScriptProvisioner,
        ProvisionRequest, Provisioner,
    },
    setup::{proxmox::OsTemplate, ProxmoxSetup},
};
use dcc_common::DccIdentity;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
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
    /// Initialize a new agent keypair
    Init {
        /// Output directory for keys (default: ~/.dc-agent)
        #[arg(long)]
        output: Option<PathBuf>,

        /// Force overwrite existing keys
        #[arg(long, default_value = "false")]
        force: bool,
    },
    /// Register agent with the Decent Cloud API
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
        /// Actually call the API to verify authentication works
        #[arg(long, default_value = "false")]
        verify_api: bool,
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
                Commands::Doctor { verify_api } => run_doctor(config, verify_api).await,
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

/// Get the default agent keys directory (~/.dc-agent)
fn default_agent_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Failed to find home directory")
        .join(".dc-agent")
}

/// Initialize a new agent keypair
fn run_init(output: Option<PathBuf>, force: bool) -> Result<()> {
    println!("dc-agent init");
    println!("=============\n");

    let agent_dir = output.unwrap_or_else(default_agent_dir);
    let private_key_path = agent_dir.join("agent.key");
    let public_key_path = agent_dir.join("agent.pub");

    // Check if keys already exist
    if private_key_path.exists() && !force {
        anyhow::bail!(
            "Agent key already exists at {}. Use --force to overwrite.",
            private_key_path.display()
        );
    }

    // Create directory if needed
    std::fs::create_dir_all(&agent_dir)?;

    // Generate new Ed25519 keypair
    println!("Generating new Ed25519 keypair...");
    let signing_key = SigningKey::generate(&mut OsRng);
    let identity = DccIdentity::new_signing(&signing_key)?;

    // Get the public key bytes and hex
    let pubkey_bytes = identity.to_bytes_verifying();
    let pubkey_hex = hex::encode(&pubkey_bytes);
    let secret_bytes = signing_key.to_bytes();
    let secret_hex = hex::encode(secret_bytes);

    // Write private key (secret + public = 64 bytes for ed25519)
    std::fs::write(&private_key_path, &secret_hex)?;
    // Set restrictive permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&private_key_path, std::fs::Permissions::from_mode(0o600))?;
    }

    // Write public key
    std::fs::write(&public_key_path, &pubkey_hex)?;

    println!("✓ Agent keypair generated successfully\n");
    println!("Keys directory: {}", agent_dir.display());
    println!("Private key: {}", private_key_path.display());
    println!("Public key:  {}", public_key_path.display());
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

/// Register agent with the Decent Cloud API
async fn run_register(
    provider_pubkey: String,
    provider_secret_key: Option<String>,
    api_endpoint: String,
    label: Option<String>,
    keys_dir: Option<PathBuf>,
) -> Result<()> {
    println!("dc-agent register");
    println!("=================\n");

    let agent_dir = keys_dir.unwrap_or_else(default_agent_dir);
    let private_key_path = agent_dir.join("agent.key");
    let public_key_path = agent_dir.join("agent.pub");

    // Load agent public key
    if !public_key_path.exists() {
        anyhow::bail!(
            "Agent key not found at {}. Run 'dc-agent init' first.",
            public_key_path.display()
        );
    }

    let agent_pubkey_hex = std::fs::read_to_string(&public_key_path)?
        .trim()
        .to_string();
    let agent_pubkey = hex::decode(&agent_pubkey_hex)
        .map_err(|e| anyhow::anyhow!("Invalid agent public key: {}", e))?;

    if agent_pubkey.len() != 32 {
        anyhow::bail!("Agent public key must be 32 bytes");
    }

    println!("Agent public key: {}", agent_pubkey_hex);
    println!("Provider public key: {}", provider_pubkey);
    println!("API endpoint: {}", api_endpoint);
    if let Some(lbl) = &label {
        println!("Label: {}", lbl);
    }
    println!();

    // Get provider secret key
    let provider_secret = match provider_secret_key {
        Some(key) => key,
        None => rpassword::prompt_password("Provider secret key (hex): ")?,
    };

    let provider_secret_bytes = hex::decode(&provider_secret)
        .map_err(|e| anyhow::anyhow!("Invalid provider secret key: {}", e))?;

    if provider_secret_bytes.len() != 32 {
        anyhow::bail!("Provider secret key must be 32 bytes");
    }

    // Create provider identity for signing
    let provider_identity = DccIdentity::new_signing_from_bytes(&provider_secret_bytes)?;

    // Verify provider pubkey matches
    let derived_pubkey = provider_identity.to_bytes_verifying();
    let expected_pubkey = hex::decode(&provider_pubkey)
        .map_err(|e| anyhow::anyhow!("Invalid provider public key: {}", e))?;

    if derived_pubkey != expected_pubkey {
        anyhow::bail!(
            "Provider secret key does not match the provided public key.\n\
             Expected: {}\n\
             Derived:  {}",
            provider_pubkey,
            hex::encode(&derived_pubkey)
        );
    }

    // Build delegation message
    // Format: agent_pubkey || provider_pubkey || permissions_json || expires_at_ns || label
    let permissions = vec!["provision", "health_check", "heartbeat", "fetch_contracts"];
    let permissions_json = serde_json::to_string(&permissions)?;

    let mut message = Vec::new();
    message.extend_from_slice(&agent_pubkey);
    message.extend_from_slice(&derived_pubkey);
    message.extend_from_slice(permissions_json.as_bytes());
    // No expiry (None)
    if let Some(lbl) = &label {
        message.extend_from_slice(lbl.as_bytes());
    }

    // Sign the delegation
    println!("Signing delegation...");
    let signature = provider_identity.sign(&message)?;
    let signature_hex = hex::encode(signature.to_bytes());

    // Register with API
    println!("Registering with API...");

    let client = reqwest::Client::new();
    let url = format!(
        "{}/providers/{}/agent-delegations",
        api_endpoint, provider_pubkey
    );

    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct CreateDelegationRequest {
        agent_pubkey: String,
        permissions: Vec<String>,
        expires_at_ns: Option<i64>,
        label: Option<String>,
        signature: String,
    }

    let request_body = CreateDelegationRequest {
        agent_pubkey: agent_pubkey_hex.clone(),
        permissions: permissions.iter().map(|s| s.to_string()).collect(),
        expires_at_ns: None,
        label: label.clone(),
        signature: signature_hex,
    };

    // Sign the HTTP request with provider key
    let timestamp = chrono::Utc::now().timestamp();
    let body_json = serde_json::to_string(&request_body)?;
    let sign_message = format!("POST\n{}\n{}\n{}", url, timestamp, body_json);
    let http_signature = provider_identity.sign(sign_message.as_bytes())?;

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("X-Public-Key", &provider_pubkey)
        .header("X-Timestamp", timestamp.to_string())
        .header("X-Signature", hex::encode(http_signature.to_bytes()))
        .body(body_json)
        .send()
        .await?;

    let status = response.status();
    let response_text = response.text().await?;

    if !status.is_success() {
        anyhow::bail!("Registration failed ({}): {}", status, response_text);
    }

    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    struct ApiResponse<T> {
        success: bool,
        data: Option<T>,
        error: Option<String>,
    }

    let api_response: ApiResponse<serde_json::Value> = serde_json::from_str(&response_text)
        .map_err(|e| anyhow::anyhow!("Failed to parse API response: {}", e))?;

    if !api_response.success {
        anyhow::bail!(
            "Registration failed: {}",
            api_response
                .error
                .unwrap_or_else(|| "Unknown error".to_string())
        );
    }

    println!("\n✓ Agent registered successfully!\n");
    println!("Agent public key: {}", agent_pubkey_hex);
    println!("Permissions: {}", permissions.join(", "));
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
            println!(
                "Templates: {}",
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
            println!(
                "1. Edit {} and set your provider_secret_key",
                output.display()
            );
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
        format!(
            "test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        )
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

async fn run_doctor(config: Config, verify_api: bool) -> Result<()> {
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

    // Check provisioner configuration
    match config.provisioner.provisioner_type {
        ProvisionerType::Proxmox => {
            if let Some(proxmox) = config.provisioner.get_proxmox() {
                println!("Provisioner: Proxmox");
                println!("  API URL: {}", proxmox.api_url);
                println!("  Node: {}", proxmox.node);
                println!("  Template VMID: {}", proxmox.template_vmid);
                println!("  Storage: {}", proxmox.storage);
                println!("  Verify SSL: {}", proxmox.verify_ssl);
                if let Some(pool) = &proxmox.pool {
                    println!("  Resource pool: {}", pool);
                }
            } else {
                println!("X Proxmox configuration missing");
                return Err(anyhow::anyhow!(
                    "Proxmox type selected but configuration missing"
                ));
            }
        }
        ProvisionerType::Script => {
            if let Some(script) = config.provisioner.get_script() {
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
            } else {
                println!("X Script configuration missing");
                return Err(anyhow::anyhow!(
                    "Script type selected but configuration missing"
                ));
            }
        }
        ProvisionerType::Manual => {
            if let Some(manual) = config.provisioner.get_manual() {
                println!("Provisioner: Manual");
                if let Some(webhook) = &manual.notification_webhook {
                    println!("  Notification webhook: {}", webhook);
                } else {
                    println!("  No notification webhook configured");
                }
            } else {
                println!("X Manual configuration missing");
                return Err(anyhow::anyhow!(
                    "Manual type selected but configuration missing"
                ));
            }
        }
    }
    println!();

    // Test API connectivity
    let provisioner_type = match config.provisioner.provisioner_type {
        ProvisionerType::Proxmox => "proxmox",
        ProvisionerType::Script => "script",
        ProvisionerType::Manual => "manual",
    };

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
                println!(
                    "  Next heartbeat in: {}s",
                    response.next_heartbeat_seconds
                );
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
    } else {
        println!();
        println!("Tip: Use --verify-api to test API authentication");
    }
    println!();

    println!("Doctor check complete!");
    Ok(())
}
