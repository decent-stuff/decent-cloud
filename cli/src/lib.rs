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
}
