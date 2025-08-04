mod account;
mod contract;
mod keygen;
mod ledger;
mod offering;
mod provider;
mod user;

use crate::argparse::{Cli, Commands};
pub use account::handle_account_command;
use candid::Principal as IcPrincipal;
use clap::Parser;
pub use contract::handle_contract_command;
pub use keygen::handle_keygen_command;
pub use ledger::{handle_ledger_local_command, handle_ledger_remote_command};
use ledger_map::LedgerMap;
pub use offering::handle_offering_command;
pub use provider::handle_provider_command;
use std::error::Error;
pub use user::handle_user_command;

pub async fn handle_command(
    command: Commands,
    ledger_local: LedgerMap,
) -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let network: String = cli.network.unwrap_or_else(|| "ic".to_string());

    let network_url = match network.as_str() {
        "local" => "http://127.0.0.1:8000",
        "mainnet-eu" | "mainnet-01" | "mainnet-02" | "ic" => "https://icp-api.io",
        _ => panic!("unknown network: {}", network),
    };

    let ledger_canister_id = match network.as_str() {
        "local" => IcPrincipal::from_text("bkyz2-fmaaa-aaaaa-qaaaq-cai")?,
        "mainnet-eu" => IcPrincipal::from_text("tlvs5-oqaaa-aaaas-aaabq-cai")?,
        "mainnet-01" | "ic" => IcPrincipal::from_text("ggi4a-wyaaa-aaaai-actqq-cai")?,
        "mainnet-02" => IcPrincipal::from_text("gplx4-aqaaa-aaaai-actra-cai")?,
        _ => panic!("unknown network: {}", network),
    };

    let identity = cli.identity;

    match command {
        Commands::Keygen(args) => handle_keygen_command(args, identity).await,
        Commands::Account(args) => {
            handle_account_command(
                args,
                network_url,
                ledger_canister_id,
                identity,
                &ledger_local,
            )
            .await
        }
        Commands::Provider(args) => {
            handle_provider_command(
                args,
                network_url,
                ledger_canister_id,
                identity,
                ledger_local,
            )
            .await
        }
        Commands::User(args) => {
            handle_user_command(
                args,
                network_url,
                ledger_canister_id,
                identity,
                ledger_local,
            )
            .await
        }
        Commands::LedgerLocal(args) => handle_ledger_local_command(args, ledger_local).await,
        Commands::LedgerRemote(args) => {
            handle_ledger_remote_command(
                args,
                network_url,
                ledger_canister_id,
                identity,
                ledger_local,
            )
            .await
        }
        Commands::Offering(args) => handle_offering_command(args, ledger_local).await,
        Commands::Contract(args) => {
            handle_contract_command(
                args,
                network_url,
                ledger_canister_id,
                identity,
                ledger_local,
            )
            .await
        }
    }
}
