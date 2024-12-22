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
                .help("Which IC network to use, e.g., ic, local"),
        )
        .arg(
            Arg::new("identity")
                .long("identity")
                .global(true)
                .help("Identity name for the account")
                .action(ArgAction::Set),
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
                ),
        )
        .subcommand(
            Command::new("account")
                .arg_required_else_help(true)
                .about("Account management commands.")
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
                        .value_name("another-account-principal")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("amount-e9s")
                        .long("amount-e9s")
                        .help("Amount to transfer, in e9s")
                        .value_name("amount-in-token-en9s")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("amount-dct")
                        .long("amount-dct")
                        .help("Amount to transfer, in DC tokens")
                        .conflicts_with("amount-e9s")
                        .value_name("amount-in-tokens")
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
                    Arg::new("balances")
                        .long("balances")
                        .help("Get balances of all node provider identities")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("register")
                        .long("register")
                        .help("Register a node provider in the Decent Cloud Ledger, making it part of the network")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("check-in")
                        .long("check-in")
                        .help("Check-in Node Provider at the Decent Cloud Ledger, marking that a NP is available and accepting new requests")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("check-in-memo")
                        .long("check-in-memo")
                        .help("Provide the given memo value for the check-in")
                        .action(ArgAction::Set)
                )
                .arg(
                    Arg::new("check-in-nonce")
                        .long("check-in-nonce")
                        .help("Get the Node Provider check-in nonce at the Decent Cloud Ledger")
                        .action(ArgAction::SetTrue)
                )
                .arg(
                    Arg::new("update-profile")
                        .long("update-profile")
                        .help("Update Node Provider profile, from the provided profile description file")
                        .action(ArgAction::Set)
                        .value_name("file-path")
                        .num_args(1),
                )
                .arg(
                    Arg::new("update-offering")
                        .long("update-offering")
                        .help("Update Node Provider offering, from the provided offering description file")
                        .action(ArgAction::Set)
                        .num_args(1)
                        .value_name("file-path")
                        .conflicts_with("update-profile"),
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
                    Arg::new("balances")
                        .long("balances")
                        .help("Get balances of all user identities")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("register")
                        .long("register")
                        .help("Register user at the Decent Cloud Ledger")
                        .action(ArgAction::SetTrue),
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
                    Arg::new("data-fetch")
                        .long("data-fetch")
                        .visible_aliases(["fetch", "pull"])
                        .action(ArgAction::SetTrue)
                        .help("Sync data from the ledger"),
                )
                .arg(
                    Arg::new("data-push-authorize")
                        .long("data-push-authorize")
                        .visible_aliases(["push-authorize", "push-auth"])
                        .help("Authorize push to the Decent Cloud Ledger")
                        .action(ArgAction::SetTrue),
                    )
                .arg(
                    Arg::new("data-push")
                        .long("data-push")
                        .visible_aliases(["push"])
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
                        .default_value("127.0.0.1")
                        .help("Which IC network to use"),
                )
        )
        .subcommand(
            Command::new("offering")
                .arg_required_else_help(true)
                .about("Offering management commands.")
                .arg(
                    Arg::new("list")
                        .long("list")
                        .help("List all offerings")
                        .action(ArgAction::SetTrue),
                    )
                .arg(
                    Arg::new("query")
                        .long("query")
                        .help("Search for offerings that match the provided query")
                        .action(ArgAction::Set)
                        .num_args(1),
                )
                .arg(
                    Arg::new("contract-request")
                        .long("contract-request")
                        .help("Request to sign a contract with the given offering ID")
                        .action(ArgAction::Set)
                        .value_name("offering-id")
                        .num_args(1),
                )
                .arg(
                    Arg::new("contracts-list-open")
                        .long("contracts-list-open")
                        .help("List all open contracts")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("contract-reply")
                        .long("contract-reply")
                        .help("Reply to a contract request, accept or reject")
                        .action(ArgAction::Set)
                        .num_args(1)
                        .value_name("is-accepted")
                        .requires("contract-id"),
                )
                .arg(
                    Arg::new("contract-id")
                        .long("contract-id")
                        .help("Contract ID to use")
                        .action(ArgAction::Set)
                        .num_args(1),
                )
        )
        .get_matches()
}
