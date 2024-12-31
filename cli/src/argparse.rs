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
                        .num_args(0..)
                        .requires("identity"),
                )
                .arg(
                    Arg::new("generate")
                        .long("generate")
                        .conflicts_with("mnemonic")
                        .help("Generate a random new mnemonic")
                        .action(ArgAction::SetTrue)
                        .requires("identity"),
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
                        .action(ArgAction::SetTrue)
                        .requires("identity"),
                )
                .arg(
                    Arg::new("transfer-to")
                        .long("transfer-to")
                        .help("Transfer funds to another account")
                        .value_name("another-account-principal")
                        .action(ArgAction::Set)
                        .requires("identity"),
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
                        .action(ArgAction::SetTrue)
                        .requires("identity"),
                )
                .arg(
                    Arg::new("check-in")
                        .long("check-in")
                        .help("Check-in Node Provider at the Decent Cloud Ledger, marking that a NP is available and accepting new requests")
                        .action(ArgAction::SetTrue)
                        .requires("identity"),
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
                        .num_args(1)
                        .requires("identity"),
                )
                .arg(
                    Arg::new("update-offering")
                        .long("update-offering")
                        .help("Update Node Provider offering, from the provided offering description file")
                        .action(ArgAction::Set)
                        .num_args(1)
                        .value_name("file-path")
                        .conflicts_with("update-profile")
                        .requires("identity"),
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
                        .action(ArgAction::SetTrue)
                        .requires("identity"),
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
                .arg_required_else_help(true)
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
                        .action(ArgAction::SetTrue)
                        .requires("identity"),
                    )
                .arg(
                    Arg::new("data-push")
                        .long("data-push")
                        .visible_aliases(["push"])
                        .help("Push the ledger entries to the Decent Cloud Ledger")
                        .action(ArgAction::SetTrue)
                        .requires("identity")
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
                    .action(ArgAction::SetTrue)
                    .conflicts_with("query"),
                )
            .arg(
                Arg::new("query")
                    .long("query")
                    .help("Search for offerings that match the provided query")
                    .action(ArgAction::Set)
                    .num_args(1)
                    .conflicts_with("list"),
            )
        )
        .subcommand(
            Command::new("contract")
            .visible_alias("contracts")
            .arg_required_else_help(true)
            .about("Contract management commands.")
            .subcommand(
                Command::new("list-open")
                .about("List all open contracts")
                .arg(
                    Arg::new("list-open")
                        .long("list-open")
                        .help("List all open contracts")
                        .action(ArgAction::SetTrue),
                ),
            )
            .subcommand(
                Command::new("sign-request")
                .about("Request to sign a contract")
                .arg(
                    Arg::new("offering-id")
                        .long("offering-id")
                        .help("Specify the offering ID for the contract sign request")
                        .required_unless_present_any(["interactive"])
                        .action(ArgAction::Set)
                        .num_args(1),
                )
                .arg(
                    Arg::new("requester-ssh-pubkey")
                        .long("requester-ssh-pubkey")
                        .help("Public key for the user, in SSH format")
                        .action(ArgAction::Set)
                        .num_args(1)
                        .required_unless_present_any(["interactive"]),
                )
                .arg(
                    Arg::new("requester-contact")
                        .long("requester-contact")
                        .help("Contact information for the user")
                        .action(ArgAction::Set)
                        .num_args(1)
                        .required_unless_present_any(["interactive"]),
                )
                .arg(
                    Arg::new("provider-pubkey-pem")
                        .long("provider-pubkey-pem")
                        .help("Public key of the provider, as a PEM string")
                        .action(ArgAction::Set)
                        .num_args(1)
                        .required_unless_present_any(["interactive"]),
                )
                .arg(
                    Arg::new("memo")
                        .long("memo")
                        .help("Memo for the contract-sign request")
                        .action(ArgAction::Set)
                        .num_args(1)
                        .required_unless_present_any(["interactive"]),
                )
                .arg(
                    Arg::new("interactive")
                    .long("interactive")
                    .short('i')
                    .help("Interactive mode")
                    .action(ArgAction::SetTrue),
                )
            )
            .subcommand(
                Command::new("sign-reply")
                .about("Reply to a contract-sign request")
                .arg(
                    Arg::new("provider-pubkey-pem")
                        .long("provider-pubkey-pem")
                        .help("Public key of the provider, as a PEM string")
                        .action(ArgAction::Set)
                        .num_args(1)
                        .required_unless_present_any(["interactive"]),
                )
                .arg(
                    Arg::new("contract-id")
                        .long("contract-id")
                        .help("Contract ID to use")
                        .action(ArgAction::Set)
                        .num_args(1),
                )
                .arg(
                    Arg::new("accept")
                        .long("accept")
                        .help("Reply to a contract-sign request, accept")
                        .action(ArgAction::SetTrue)
                        .requires("contract-id")
                        .requires("identity"),
                )
                .arg(
                    Arg::new("reject")
                        .long("reject")
                        .help("Reply to a contract-sign request, reject")
                        .action(ArgAction::SetTrue)
                        .requires("contract-id")
                        .requires("identity"),
                )
                .arg(
                    Arg::new("memo")
                        .long("memo")
                        .help("Memo for the contract-sign request")
                        .action(ArgAction::Set)
                        .num_args(1)
                        .required_unless_present_any(["interactive"]),
                )
            )
        )
        .get_matches()
}
