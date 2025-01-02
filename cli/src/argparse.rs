use clap::{Args, Parser, Subcommand};
use dcc_common::PaymentEntries;

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
    Np(NpCommands),
    /// User management commands
    #[command(subcommand)]
    User(UserCommands),
    /// Work with the local Decent Cloud Ledger
    #[command(arg_required_else_help = true)]
    LedgerLocal(LedgerLocalArgs),
    /// Work with the remote Decent Cloud Ledger
    #[command(subcommand)]
    LedgerRemote(LedgerRemoteCommands),
    /// Offering management commands
    #[command(subcommand)]
    Offering(OfferingCommands),
    /// Contract management commands
    #[command(subcommand)]
    Contract(ContractCommands),
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
pub enum NpCommands {
    /// List all node provider identities
    List(ListArgs),

    /// Register a node provider in the ledger
    Register,

    /// Check-in Node Provider
    CheckIn(CheckInArgs),

    /// Update Node Provider profile
    UpdateProfile(UpdateProfileArgs),

    /// Update Node Provider offering
    UpdateOffering(UpdateOfferingArgs),
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
pub struct UpdateProfileArgs {
    /// Update Node Provider profile with data from the provided file
    #[arg(long, requires = "identity")]
    pub profile_file: String,
}

#[derive(Args)]
pub struct UpdateOfferingArgs {
    /// Update Node Provider offering with data from the provided file
    #[arg(long, requires = "identity")]
    pub offering_file: String,
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

#[derive(Subcommand)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub enum OfferingCommands {
    /// List all open offerings
    List,

    /// Search for offerings that match the provided query
    Query(OfferingQueryArgs),
}

#[derive(Args)]
pub struct OfferingQueryArgs {
    /// Search for offerings that match the provided query
    pub query: String,
}

#[derive(Subcommand)]
#[command(subcommand_required = true, arg_required_else_help = true)]
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

    /// Payment entries (JSON array), e.g.:
    ///   '[["on_demand","hour",1], ["reserved","year",10]]'
    #[arg(long, required_unless_present_any(["interactive"]), value_parser = parse_payment_entries)]
    pub payment_entries_json: Option<PaymentEntries>,

    /// Interactive mode
    #[arg(long, short = 'i', default_value_t = false)]
    pub interactive: bool,
}

fn parse_payment_entries(s: &str) -> Result<PaymentEntries, String> {
    serde_json::from_str(s)
        .map_err(|e| format!("Failed to parse JSON for --payment-entries-json: {}", e))
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
