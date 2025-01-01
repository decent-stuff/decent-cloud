use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "dcc",
    about = "Decent Cloud CLI",
    version = "0.1.0",
    author = "Decent Cloud Development Team",
    subcommand_required = true,
    arg_required_else_help = true
)]
pub struct Cli {
    /// Which IC network to use, e.g., ic, local
    #[arg(long, global = true)]
    pub network: Option<String>,

    /// Identity name for the account
    #[arg(long, global = true)]
    pub identity: Option<String>,

    /// Local Decent Cloud Ledger directory
    #[arg(long, global = true)]
    pub local_ledger_dir: Option<String>,

    /// Pick which subcommand to use
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate key pairs
    Keygen(KeygenArgs),
    /// Account management commands
    Account(AccountArgs),
    /// Node Provider management commands
    Np(NpArgs),
    /// User management commands
    User(UserArgs),
    /// Work with the local Decent Cloud Ledger
    LedgerLocal(LedgerLocalArgs),
    /// Work with the remote Decent Cloud Ledger
    LedgerRemote(LedgerRemoteArgs),
    /// Offering management commands
    Offering(OfferingArgs),
    /// Contract management commands
    #[command(subcommand)]
    Contract(ContractCommands),
}

#[derive(Args)]
pub struct KeygenArgs {
    /// Use the given mnemonic
    #[arg(
        long,
        conflicts_with = "generate",
        requires = "identity",
        num_args = 12..=24
    )]
    pub mnemonic: Vec<String>,

    /// Generate a random mnemonic
    #[arg(long, requires = "identity")]
    pub generate: bool,
}

#[derive(Args)]
pub struct AccountArgs {
    /// Balance of the account
    #[arg(long, requires = "identity")]
    pub balance: bool,

    /// Transfer funds to another account
    #[arg(long, requires = "identity")]
    pub transfer_to: Option<String>,

    /// Amount to transfer, in e9s
    #[arg(long, conflicts_with = "amount_dct")]
    pub amount_e9s: Option<String>,

    /// Amount to transfer, in DC tokens
    #[arg(long)]
    pub amount_dct: Option<String>,
}

#[derive(Args)]
pub struct NpArgs {
    /// List all node provider identities
    #[arg(long)]
    pub list: bool,

    /// Get balances of all node provider identities
    #[arg(long)]
    pub balances: bool,

    /// Register a node provider in the ledger
    #[arg(long, requires = "identity")]
    pub register: bool,

    /// Check-in Node Provider
    #[arg(long, requires = "identity")]
    pub check_in: bool,

    /// Provide a memo value for check-in
    #[arg(long)]
    pub check_in_memo: Option<String>,

    /// Get the Node Provider check-in nonce
    #[arg(long)]
    pub check_in_nonce: bool,

    /// Update Node Provider profile
    #[arg(long, requires = "identity")]
    pub update_profile: Option<String>,

    /// Update Node Provider offering
    #[arg(long, conflicts_with = "update_profile", requires = "identity")]
    pub update_offering: Option<String>,
}

#[derive(Args)]
pub struct UserArgs {
    /// List all user identities
    #[arg(long)]
    pub list: bool,

    /// Get balances of all user identities
    #[arg(long)]
    pub balances: bool,

    /// Register user at the ledger
    #[arg(long, requires = "identity")]
    pub register: bool,
}

#[derive(Args)]
pub struct LedgerLocalArgs {
    /// List raw ledger entries
    #[arg(long)]
    pub list_entries_raw: bool,

    /// List ledger entries
    #[arg(long)]
    pub list_entries: bool,
}

#[derive(Args)]
pub struct LedgerRemoteArgs {
    /// Sync data from the ledger
    #[arg(long, visible_aliases = ["fetch", "pull"])]
    pub data_fetch: bool,

    /// Authorize push to the ledger
    #[arg(long, visible_aliases = ["push-authorize", "push-auth"], requires = "identity")]
    pub data_push_authorize: bool,

    /// Push the ledger entries to the ledger
    #[arg(long, visible_aliases = ["push"], requires = "identity")]
    pub data_push: bool,

    /// Canister function to call
    #[arg(long)]
    pub canister_function: Option<String>,

    /// Prefix directory
    #[arg(long)]
    pub dir: Option<String>,

    /// Which IC network to use
    #[arg(long)]
    pub network: Option<String>,
}

#[derive(Args)]
pub struct OfferingArgs {
    /// List all offerings
    #[arg(long, conflicts_with = "query")]
    pub list: bool,

    /// Search for offerings with the provided query
    #[arg(long, conflicts_with = "list")]
    pub query: Option<String>,
}

#[derive(Subcommand)]
pub enum ContractCommands {
    /// List all open contracts
    ListOpen(ListOpenArgs),
    /// Request to sign a contract
    SignRequest(SignRequestArgs),
    /// Reply to a contract-sign request
    SignReply(SignReplyArgs),
}

#[derive(Args)]
pub struct ListOpenArgs {
    #[arg(long)]
    pub list_open: bool,
}

#[derive(Args)]
pub struct SignRequestArgs {
    /// Specify the offering ID for the contract sign request
    #[arg(long, required_unless_present_any(["interactive"]))]
    pub offering_id: Option<String>,

    /// Public key for the user, in SSH format
    #[arg(long, required_unless_present_any(["interactive"]))]
    pub requester_ssh_pubkey: Option<String>,

    /// Contact information for the user
    #[arg(long, required_unless_present_any(["interactive"]))]
    pub requester_contact: Option<String>,

    /// Public key of the provider, as a PEM string
    #[arg(long, required_unless_present_any(["interactive"]))]
    pub provider_pubkey_pem: Option<String>,

    /// Memo for the contract-sign request
    #[arg(long, required_unless_present_any(["interactive"]))]
    pub memo: Option<String>,

    /// Interactive mode
    #[arg(long, short = 'i', default_value_t = false)]
    pub interactive: bool,
}

#[derive(Args)]
pub struct SignReplyArgs {
    /// Public key of the original requester, as a PEM string
    #[arg(long, required_unless_present_any(["interactive"]))]
    pub provider_pubkey_pem: Option<String>,

    /// Contract ID of the request that we are replying to
    #[arg(long, required_unless_present_any(["interactive"]))]
    pub contract_id: Option<String>,

    /// True/False to mark whether the signing was accepted or rejected by the provider
    #[arg(long, requires = "identity", required_unless_present_any(["interactive"]), visible_alias = "accept")]
    pub sign_accept: Option<bool>,

    /// Thank you note, or similar on success. Reason the request failed on failure.
    #[arg(long, required_unless_present_any(["interactive"]))]
    pub response_text: Option<String>,

    /// Instructions or a link to the detailed instructions: describing next steps, further information, etc.
    #[arg(long, required_unless_present_any(["interactive"]))]
    pub response_details: Option<String>,

    /// Interactive mode
    #[arg(long, short = 'i', default_value_t = false)]
    pub interactive: bool,
}

pub fn parse_args() -> Cli {
    Cli::parse()
}
