/// This file contains the implementation of a command-line interface (CLI) for interacting with the LedgerMap library.
///
/// The CLI allows various ledger operations, such as listing, inserting/updating (upserting), and deleting entries.
///
#[cfg(not(target_arch = "wasm32"))]
use clap::{arg, Arg, Command};
use ledger_map::LedgerMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Once;

/// Struct to hold the parsed command-line arguments
struct ParsedArgs {
    list: bool,
    upsert: Option<(String, String)>,
    delete: Option<String>,
    path: Option<String>,
}

/// Parse the command-line arguments using clap library
#[cfg(not(target_arch = "wasm32"))]
fn parse_args() -> ParsedArgs {
    let matches = Command::new("LedgerMap CLI")
        .about("LedgerMap CLI")
        .arg(arg!(--list "List entries").required(false))
        .arg(
            Arg::new("upsert")
                .long("upsert")
                .help("Upsert key-value pair")
                .num_args(2),
        )
        .arg(arg!(--delete <KEY> "Delete key").required(false))
        .arg(arg!(--path <VALUE> "Specify file path for the ledger").required(false))
        .get_matches();

    let list = *matches.get_one::<bool>("list").unwrap_or(&false);

    let upsert = matches.get_many::<String>("upsert").map(|mut values| {
        (
            values.next().unwrap().to_string(),
            values.next().unwrap().to_string(),
        )
    });

    let delete = matches.get_one::<String>("delete").map(|s| s.to_string());

    let path = matches.get_one::<String>("path").map(|s| s.to_string());

    ParsedArgs {
        list,
        upsert,
        delete,
        path,
    }
}

#[cfg(target_arch = "wasm32")]
fn parse_args() -> ParsedArgs {
    ParsedArgs {
        list: false,
        upsert: None,
        delete: None,
        path: None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn logs_init() {
    // Set log level to info by default
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init_from_env("RUST_LOG");
}

#[cfg(target_arch = "wasm32")]
fn logs_init() {}

static INIT: Once = Once::new();

pub fn initialize() {
    INIT.call_once(|| {
        logs_init();
    });
}

fn main() -> anyhow::Result<()> {
    logs_init();

    // Parse the command-line arguments
    let args = parse_args();

    // Extract the file path from the parsed arguments
    let ledger_path = args.path.as_ref().map(|p| PathBuf::from_str(p).unwrap());

    let mut ledger_map =
        LedgerMap::new_with_path(None, ledger_path).expect("Failed to create ledger");

    if args.list {
        println!("Listing entries:");
        // Iterate over the entries in the ledger and print them
        for entry in ledger_map.iter(None) {
            println!("{}", entry);
        }
    }

    if let Some((key, value)) = args.upsert {
        // Upsert (insert/update) an entry in the ledger
        ledger_map.upsert("Unspecified", key.as_bytes(), value.as_bytes())?;
        println!("Upsert entry with KEY: {}, VALUE: {}", key, value);
        ledger_map.commit_block()?;
    }

    if let Some(key) = args.delete {
        // Delete an entry from the ledger
        ledger_map.delete("Unspecified", key.as_bytes())?;
        println!("Delete entry with KEY: {}", key);
    }

    Ok(())
}
