mod argparse;
mod commands;
mod identity;
mod ledger;
mod utils;

use argparse::parse_args;
use decent_cloud::CliError;
use ledger_map::LedgerMap;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = parse_args();
    utils::init_logger(cli.verbose);

    let ledger_path = match cli.local_ledger_dir {
        Some(dir) => PathBuf::from(dir),
        None => {
            let home = dirs::home_dir().ok_or(CliError::HomeDirNotFound)?;
            home.join(".dcc").join("ledger")
        }
    }
    .join("main.bin");

    let ledger_local = LedgerMap::new_with_path(None, Some(ledger_path))
        .map_err(CliError::LedgerLoad)?;

    dcc_common::refresh_caches_from_ledger(&ledger_local)
        .map_err(CliError::CacheRefresh)?;

    commands::handle_command(cli.command, ledger_local).await?;

    Ok(())
}
