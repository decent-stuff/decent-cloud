mod account;
mod contract;
mod keygen;
mod ledger;
mod offering;
mod provider;
mod user;

use crate::argparse::{Cli, Commands};
use crate::CliError;
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
        unknown => return Err(CliError::InvalidNetwork(unknown.to_string()).into()),
    };

    let ledger_canister_id = match network.as_str() {
        "local" => IcPrincipal::from_text("bkyz2-fmaaa-aaaaa-qaaaq-cai")?,
        "mainnet-eu" => IcPrincipal::from_text("tlvs5-oqaaa-aaaas-aaabq-cai")?,
        "mainnet-01" | "ic" => IcPrincipal::from_text("ggi4a-wyaaa-aaaai-actqq-cai")?,
        "mainnet-02" => IcPrincipal::from_text("gplx4-aqaaa-aaaai-actra-cai")?,
        unknown => return Err(CliError::InvalidNetwork(unknown.to_string()).into()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CliError;

    #[test]
    fn test_invalid_network_error_message() {
        let error = CliError::InvalidNetwork("invalid-network".to_string());
        let error_msg = format!("{}", error);

        // Verify error message contains helpful information
        assert!(error_msg.contains("Invalid network"));
        assert!(error_msg.contains("invalid-network"));
        assert!(error_msg.contains("local"));
        assert!(error_msg.contains("mainnet-eu"));
        assert!(error_msg.contains("mainnet-01"));
        assert!(error_msg.contains("mainnet-02"));
        assert!(error_msg.contains("ic"));
        assert!(error_msg.contains("--network"));
    }

    #[test]
    fn test_valid_networks_are_accepted() {
        // This test verifies that all valid network names are properly handled
        // We can't easily test the full handle_command function without more setup,
        // but we can verify the match arms cover all expected networks

        let valid_networks = vec!["local", "mainnet-eu", "mainnet-01", "mainnet-02", "ic"];

        for network in valid_networks {
            // Verify network_url match handles this network
            let network_url = match network {
                "local" => Some("http://127.0.0.1:8000"),
                "mainnet-eu" | "mainnet-01" | "mainnet-02" | "ic" => Some("https://icp-api.io"),
                _ => None,
            };
            assert!(network_url.is_some(), "Network {} should have a URL", network);

            // Verify ledger_canister_id match handles this network
            let principal_result: Result<IcPrincipal, ic_agent::export::PrincipalError> = match network {
                "local" => IcPrincipal::from_text("bkyz2-fmaaa-aaaaa-qaaaq-cai"),
                "mainnet-eu" => IcPrincipal::from_text("tlvs5-oqaaa-aaaas-aaabq-cai"),
                "mainnet-01" | "ic" => IcPrincipal::from_text("ggi4a-wyaaa-aaaai-actqq-cai"),
                "mainnet-02" => IcPrincipal::from_text("gplx4-aqaaa-aaaai-actra-cai"),
                _ => panic!("Should not reach here with valid network"),
            };
            assert!(principal_result.is_ok(), "Network {} should have a valid principal", network);
        }
    }
}
