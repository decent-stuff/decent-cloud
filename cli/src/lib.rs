pub mod identity;
pub mod ledger_canister_client;

pub use ledger_map::*;

/// Error type for CLI initialization failures
#[derive(Debug)]
pub enum CliError {
    HomeDirNotFound,
    LedgerLoad(anyhow::Error),
    CacheRefresh(anyhow::Error),
    InvalidNetwork(String),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::HomeDirNotFound => {
                write!(
                    f,
                    "Could not determine home directory. Please set HOME environment variable or use --local-ledger-dir to specify ledger location."
                )
            }
            CliError::LedgerLoad(e) => {
                write!(
                    f,
                    "Failed to load local ledger: {e}\n\nPossible fixes:\n  - Ensure the ledger directory exists and is readable\n  - Check file permissions\n  - Use --local-ledger-dir to specify a different location\n  - Try running: mkdir -p ~/.dcc/ledger"
                )
            }
            CliError::CacheRefresh(e) => {
                write!(
                    f,
                    "Failed to refresh caches from ledger: {e}\n\nThis may indicate a corrupted ledger. Try:\n  - Delete the ledger file and let it recreate: rm ~/.dcc/ledger/main.bin\n  - Use --local-ledger-dir to point to a backup ledger"
                )
            }
            CliError::InvalidNetwork(network) => {
                write!(
                    f,
                    "Invalid network: '{}'\n\nValid networks are: local, mainnet-eu, mainnet-01, mainnet-02, ic\n\nUse --network <name> to specify the network.",
                    network
                )
            }
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CliError::HomeDirNotFound => None,
            CliError::LedgerLoad(e) => Some(e.as_ref()),
            CliError::CacheRefresh(e) => Some(e.as_ref()),
            CliError::InvalidNetwork(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_error_home_dir_not_found_display() {
        let error = CliError::HomeDirNotFound;
        let display_msg = format!("{}", error);

        assert!(display_msg.contains("Could not determine home directory"));
        assert!(display_msg.contains("HOME environment variable"));
        assert!(display_msg.contains("--local-ledger-dir"));
        assert!(display_msg.contains("specify ledger location"));
    }

    #[test]
    fn test_cli_error_ledger_load_display() {
        let underlying_error = anyhow::anyhow!("Failed to open file");
        let error = CliError::LedgerLoad(underlying_error);
        let display_msg = format!("{}", error);

        assert!(display_msg.contains("Failed to load local ledger"));
        assert!(display_msg.contains("Failed to open file"));
        assert!(display_msg.contains("Possible fixes"));
        assert!(display_msg.contains("ledger directory exists"));
        assert!(display_msg.contains("file permissions"));
        assert!(display_msg.contains("--local-ledger-dir"));
        assert!(display_msg.contains("mkdir -p ~/.dcc/ledger"));
    }

    #[test]
    fn test_cli_error_cache_refresh_display() {
        let underlying_error = anyhow::anyhow!("Corrupted data");
        let error = CliError::CacheRefresh(underlying_error);
        let display_msg = format!("{}", error);

        assert!(display_msg.contains("Failed to refresh caches from ledger"));
        assert!(display_msg.contains("Corrupted data"));
        assert!(display_msg.contains("corrupted ledger"));
        assert!(display_msg.contains("rm ~/.dcc/ledger/main.bin"));
        assert!(display_msg.contains("--local-ledger-dir"));
        assert!(display_msg.contains("backup ledger"));
    }

    #[test]
    fn test_cli_error_home_dir_not_found_debug() {
        let error = CliError::HomeDirNotFound;
        let debug_msg = format!("{:?}", error);

        assert!(debug_msg.contains("HomeDirNotFound"));
    }

    #[test]
    fn test_cli_error_ledger_load_debug() {
        let underlying_error = anyhow::anyhow!("Test error");
        let error = CliError::LedgerLoad(underlying_error);
        let debug_msg = format!("{:?}", error);

        assert!(debug_msg.contains("LedgerLoad"));
        assert!(debug_msg.contains("Test error"));
    }

    #[test]
    fn test_cli_error_cache_refresh_debug() {
        let underlying_error = anyhow::anyhow!("Cache error");
        let error = CliError::CacheRefresh(underlying_error);
        let debug_msg = format!("{:?}", error);

        assert!(debug_msg.contains("CacheRefresh"));
        assert!(debug_msg.contains("Cache error"));
    }

    #[test]
    fn test_cli_error_implements_std_error() {
        // Verify that CliError can be used as a standard error
        let error: Box<dyn std::error::Error> = CliError::HomeDirNotFound.into();
        assert!(error.source().is_none());

        let underlying = anyhow::anyhow!("underlying error");
        let error_with_source: Box<dyn std::error::Error> = CliError::LedgerLoad(underlying).into();
        // LedgerLoad and CacheRefresh wrap anyhow::Error which has source()
        assert!(error_with_source.source().is_some());
    }

    #[test]
    fn test_cli_error_ledger_load_preserves_original_error() {
        let original_msg = "Permission denied";
        let underlying_error = anyhow::anyhow!(original_msg);
        let error = CliError::LedgerLoad(underlying_error);
        let display_msg = format!("{}", error);

        assert!(display_msg.contains(original_msg));
    }

    #[test]
    fn test_cli_error_cache_refresh_preserves_original_error() {
        let original_msg = "Invalid ledger format";
        let underlying_error = anyhow::anyhow!(original_msg);
        let error = CliError::CacheRefresh(underlying_error);
        let display_msg = format!("{}", error);

        assert!(display_msg.contains(original_msg));
    }

    #[test]
    fn test_cli_error_invalid_network_display() {
        let network = "invalid-network";
        let error = CliError::InvalidNetwork(network.to_string());
        let display_msg = format!("{}", error);

        assert!(display_msg.contains("Invalid network"));
        assert!(display_msg.contains(network));
        assert!(display_msg.contains("local"));
        assert!(display_msg.contains("mainnet-eu"));
        assert!(display_msg.contains("mainnet-01"));
        assert!(display_msg.contains("mainnet-02"));
        assert!(display_msg.contains("ic"));
        assert!(display_msg.contains("--network"));
    }

    #[test]
    fn test_cli_error_invalid_network_debug() {
        let network = "testnet";
        let error = CliError::InvalidNetwork(network.to_string());
        let debug_msg = format!("{:?}", error);

        assert!(debug_msg.contains("InvalidNetwork"));
        assert!(debug_msg.contains(network));
    }

    #[test]
    fn test_cli_error_messages_are_actionable() {
        // HomeDirNotFound should provide actionable steps
        let home_error = CliError::HomeDirNotFound;
        let home_msg = format!("{}", home_error);
        assert!(home_msg.contains("set HOME") || home_msg.contains("--local-ledger-dir"));

        // LedgerLoad should provide multiple fix suggestions
        let ledger_error = CliError::LedgerLoad(anyhow::anyhow!("test"));
        let ledger_msg = format!("{}", ledger_error);
        assert!(ledger_msg.contains("Possible fixes") || ledger_msg.contains("Try:"));

        // CacheRefresh should provide recovery steps
        let cache_error = CliError::CacheRefresh(anyhow::anyhow!("test"));
        let cache_msg = format!("{}", cache_error);
        assert!(cache_msg.contains("rm ") || cache_msg.contains("--local-ledger-dir"));

        // InvalidNetwork should list valid options
        let network_error = CliError::InvalidNetwork("foo".to_string());
        let network_msg = format!("{}", network_error);
        assert!(network_msg.contains("Valid networks are"));
        assert!(network_msg.contains("--network"));
    }
}

