use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run => {
            println!("Agent starting...");
            println!("Config file: {}", cli.config.display());
            Ok(())
        }
        Commands::Doctor => {
            println!("Checking configuration...");
            println!("Config file: {}", cli.config.display());
            Ok(())
        }
    }
}
