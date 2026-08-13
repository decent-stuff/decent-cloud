use anyhow::Result;
use clap::{Parser, Subcommand};
use dc_agent::{config::Config, setup_cmd::SetupProvisioner};
use std::path::PathBuf;

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
    /// Reset the root password on a provisioned VM
    ResetPassword {
        /// Contract ID (hex-encoded)
        #[arg(long)]
        contract_id: String,

        /// New password (if not provided, a secure random password will be generated)
        #[arg(long)]
        password: Option<String>,
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
        Commands::Run | Commands::Doctor { .. } | Commands::TestProvision { .. } => {
            let config = Config::load(&cli.config)?;
            match cli.command {
                Commands::Run => dc_agent::runtime::run(config).await,
                Commands::Doctor {
                    no_verify_api,
                    no_test_provision,
                } => dc_agent::doctor::run(config, !no_verify_api, !no_test_provision).await,
                Commands::TestProvision {
                    ssh_pubkey,
                    keep,
                    contract_id,
                    test_gateway,
                    skip_dns,
                } => {
                    dc_agent::ops::test_provision(
                        config,
                        ssh_pubkey,
                        keep,
                        contract_id,
                        test_gateway,
                        skip_dns,
                    )
                    .await
                }
                _ => anyhow::bail!("Invalid command state - this is a bug"),
            }
        }
        Commands::Setup { provisioner } => dc_agent::setup_cmd::run(*provisioner).await,
        Commands::Upgrade {
            check_only,
            yes,
            force,
        } => dc_agent::upgrade::run_upgrade(check_only, yes, force, None).await,
        Commands::ResetPassword {
            contract_id,
            password,
        } => {
            let config = Config::load(&cli.config)?;
            dc_agent::ops::reset_password(config, &contract_id, password).await
        }
    }
}
