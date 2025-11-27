mod argparse;
mod commands;
mod identity;
mod ledger;
mod utils;

use argparse::parse_args;
use ledger_map::LedgerMap;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = parse_args();
    utils::init_logger(cli.verbose);

    let ledger_path = cli
        .local_ledger_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .expect("Could not get home directory")
                .join(".dcc")
                .join("ledger")
        })
        .join("main.bin");

    let ledger_local =
        LedgerMap::new_with_path(None, Some(ledger_path)).expect("Failed to load the local ledger");

    dcc_common::refresh_caches_from_ledger(&ledger_local).expect("Failed to get balances");

    commands::handle_command(cli.command, ledger_local).await?;

    Ok(())
}
