use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "dcc",
    about = "Decent Cloud CLI",
    version = env!("CARGO_PKG_VERSION"),
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

    /// Verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Pick which subcommand to use
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub enum Commands {
    /// Generate key pairs
    Keygen(KeygenArgs),
    /// Account management commands
    Account(AccountArgs),
    /// Node Provider management commands
    #[command(subcommand)]
    Provider(ProviderCommands),
    /// User management commands
    #[command(subcommand)]
    User(UserCommands),
    /// Work with the local Decent Cloud Ledger
    #[command(arg_required_else_help = true)]
    LedgerLocal(LedgerLocalArgs),
    /// Work with the remote Decent Cloud Ledger
    #[command(subcommand)]
    LedgerRemote(LedgerRemoteCommands),
}

#[derive(Args)]
pub struct KeygenArgs {
    /// BIP39 compatible mnemonic, 12 to 24 words
    #[arg(long, conflicts_with = "generate", requires = "identity")]
    pub mnemonic: Option<String>,

    /// Generate a random mnemonic
    #[arg(long, requires = "identity")]
    pub generate: bool,
}

#[derive(Args)]
pub struct AccountArgs {
    /// Balance of the account
    #[arg(long, requires = "identity")]
    pub balance: bool,

    /// List all accounts in the local ledger
    #[arg(long, visible_aliases = ["list-accounts"])]
    pub list_all: bool,

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

#[derive(Subcommand)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub enum ProviderCommands {
    /// List all node provider identities
    List(ListArgs),

    /// Register a node provider in the ledger
    Register,

    /// Check-in Node Provider
    CheckIn(CheckInArgs),

    /// Get offering suggestions for a pool based on hardware capabilities
    PoolSuggestOfferings(PoolSuggestOfferingsArgs),

    /// Generate offerings for a pool with provided pricing
    PoolGenerateOfferings(PoolGenerateOfferingsArgs),
}

#[derive(Args)]
pub struct ListArgs {
    /// Include balances in the listing of node identities
    #[arg(long)]
    pub balances: bool,

    /// Only local identities
    #[arg(long, visible_aliases = ["local"])]
    pub only_local: bool,
}

#[derive(Args)]
pub struct CheckInArgs {
    /// Only print the Node Provider check-in nonce
    #[arg(long, visible_aliases = ["nonce"])]
    pub only_nonce: bool,

    /// Provide a memo value for check-in
    #[arg(long)]
    pub memo: Option<String>,
}

#[derive(Args)]
pub struct PoolSuggestOfferingsArgs {
    /// Pool ID to get suggestions for
    #[arg(long, required = true)]
    pub pool_id: String,

    /// API server URL (defaults to production)
    #[arg(long, default_value = "https://api.decentcloud.net")]
    pub api_url: String,
}

#[derive(Args)]
pub struct PoolGenerateOfferingsArgs {
    /// Pool ID to generate offerings for
    #[arg(long, required = true)]
    pub pool_id: String,

    /// Tier names to generate (comma-separated, e.g., "small,medium")
    #[arg(long)]
    pub tiers: Option<String>,

    /// Pricing file (JSON format with tier -> {monthlyPrice, currency})
    #[arg(long, required = true)]
    pub pricing_file: String,

    /// Visibility: "public" or "private"
    #[arg(long, default_value = "public")]
    pub visibility: String,

    /// Preview only - don't actually create offerings
    #[arg(long)]
    pub dry_run: bool,

    /// API server URL (defaults to production)
    #[arg(long, default_value = "https://api.decentcloud.net")]
    pub api_url: String,
}

#[derive(Subcommand)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub enum UserCommands {
    /// List all user identities
    List(ListArgs),

    /// Register a user identity in the ledger
    Register,
}

#[derive(Args)]
pub struct LedgerLocalArgs {
    /// List raw ledger entries
    #[arg(long)]
    pub list_entries_raw: bool,

    /// List ledger entries
    #[arg(long)]
    pub list_entries: bool,

    /// List all accounts in the local ledger
    #[arg(long)]
    pub list_accounts: bool,
}

#[derive(Subcommand, PartialEq)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub enum LedgerRemoteCommands {
    /// Sync data from the ledger
    #[command(visible_aliases = ["fetch", "pull"])]
    DataFetch,

    /// Authorize push to the ledger
    #[command(visible_aliases = ["push-authorize", "push-auth"])]
    DataPushAuthorize,

    /// Push the ledger entries to the ledger
    #[command(visible_aliases= ["push"])]
    DataPush,

    /// Show metadata
    Metadata,

    /// Get the registration fee
    GetRegistrationFee,

    /// Get nonce that is used as the seed for the check-in
    GetCheckInNonce,

    /// Get DEBUG logs from the ledger canister
    GetLogsDebug,

    /// Get INFO logs from the ledger canister
    GetLogsInfo,

    /// Get WARNING logs from the ledger canister
    GetLogsWarn,

    /// Get ERROR logs from the ledger canister
    GetLogsError,
}

pub fn parse_args() -> Cli {
    Cli::parse()
}
