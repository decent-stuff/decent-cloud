use clap::{Arg, ArgAction, Command};

pub fn parse_args() -> clap::ArgMatches {
    Command::new("dcc")
        .about("Decent Cloud Cli")
        .version("0.1.0")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .author("Decent Cloud Development Team")
        .arg(
            Arg::new("network")
                .long("network")
                .action(ArgAction::Set)
                .default_value("local")
                .help("Which IC network to use"),
        )
        .arg(
            Arg::new("local-ledger-dir")
                .long("local-ledger-dir")
                .action(ArgAction::Set)
                .global(true)
                .help("Local Decent Cloud Ledger directory"),
        )
        .subcommand(
            Command::new("keygen")
                .arg_required_else_help(true)
                .about("Generate a new key pair.")
                .arg(
                    Arg::new("mnemonic")
                        .long("mnemonic")
                        .help("Use the following mnemonic to generate a new key pair")
                        .conflicts_with("generate")
                        .action(ArgAction::Set)
                        .num_args(0..),
                )
                .arg(
                    Arg::new("generate")
                        .long("generate")
                        .conflicts_with("mnemonic")
                        .help("Generate a random new mnemonic")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("identity")
                        .long("identity")
                        .required(true)
                        .help("Identity for the new key pair")
                        .action(ArgAction::Set),
                ),
        )
        .subcommand(
            Command::new("account")
                .arg_required_else_help(true)
                .about("Account management commands.")
                .arg(
                    Arg::new("identity")
                        .long("identity")
                        .required(true)
                        .help("Identity for the account")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("balance")
                        .long("balance")
                        .help("Balance of the account")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("transfer-to")
                        .long("transfer-to")
                        .help("Transfer funds to another account")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("amount-e9s")
                        .long("amount-e9s")
                        .help("Amount to transfer, in e9s")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("amount-dct")
                        .long("amount-dct")
                        .help("Amount to transfer, in DC tokens")
                        .conflicts_with("amount-e9s")
                        .action(ArgAction::Set),
                ),
        )
        .subcommand(
            Command::new("np")
                .arg_required_else_help(true)
                .about("Node Provider management commands.")
                .arg(
                    Arg::new("list")
                        .long("list")
                        .help("List all node provider identities")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("register")
                        .long("register")
                        .help("Register node provider at the Decent Cloud Ledger")
                        .action(ArgAction::Set)
                        .num_args(1),
                )
                .arg(
                    Arg::new("check-in")
                        .long("check-in")
                        .help("Check-in Node Provider at the Decent Cloud Ledger, marking that a NP is available")
                        .action(ArgAction::Set)
                        .num_args(1),
                )
                .arg(
                    Arg::new("update-profile")
                        .long("update-profile")
                        .help("Update Node Provider profile, from the provided profile description file")
                        .action(ArgAction::Set)
                        .num_args(2),
                ),
        )
        .subcommand(
            Command::new("user")
                .arg_required_else_help(true)
                .about("User management commands.")
                .arg(
                    Arg::new("list")
                        .long("list")
                        .help("List all user identities")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("register")
                        .long("register")
                        .help("Register user at the Decent Cloud Ledger")
                        .action(ArgAction::Set)
                        .num_args(1),
                ),
        )
        .subcommand(
            Command::new("ledger_local")
                .about("Work with the local Decent Cloud Ledger.")
                .arg(
                    Arg::new("list_entries_raw")
                        .long("list_entries_raw")
                        .action(ArgAction::SetTrue)
                        .help("List raw ledger entries")
                    )
                .arg(Arg::new("list_entries")
                        .long("list_entries")
                        .action(ArgAction::SetTrue)
                        .help("List ledger entries")
                    ),
        )
        .subcommand(
            Command::new("ledger_remote")
                // .arg_required_else_help(true)
                .about("Work with the remote Decent Cloud Ledger.")
                .arg(
                    Arg::new("identity")
                        .long("identity")
                        .help("Identity for the remote ledger")
                        .action(ArgAction::Set)
                        .num_args(1),
                )
                .arg(
                    Arg::new("data-fetch")
                        .long("data-fetch")
                        .visible_aliases(&["fetch", "pull"])
                        .action(ArgAction::SetTrue)
                        .help("Sync data from the ledger"),
                )
                .arg(
                    Arg::new("data-push-authorize")
                        .long("data-push-authorize")
                        .visible_aliases(&["push-authorize", "push-auth"])
                        .help("Authorize push to the Decent Cloud Ledger")
                        .action(ArgAction::SetTrue),
                    )
                .arg(
                    Arg::new("data-push")
                        .long("data-push")
                        .visible_aliases(&["push"])
                        .help("Push the ledger entries to the Decent Cloud Ledger")
                        .action(ArgAction::SetTrue)
                    )
                .arg(
                    Arg::new("canister_function")
                        .action(ArgAction::Set)
                        .help("Canister function to call"),
                )
                .arg(
                    Arg::new("dir")
                        .long("dir")
                        .action(ArgAction::Set)
                        .help("Prefix directory"),
                )
                .arg(
                    Arg::new("network")
                        .long("network")
                        .action(ArgAction::Set)
                        // .default_value("mainnet")
                        .default_value("127.0.0.1")
                        .help("Which IC network to use"),
                )
        )
        .get_matches()
}
