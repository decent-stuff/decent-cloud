#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use ic_cdk::println;
pub mod account_transfer_approvals;
pub mod account_transfers;
pub mod account_transfers_errors;
pub mod cache_balances;
pub mod cache_reputation;
pub mod cache_transactions;
pub mod contract_refund_request;
pub mod contract_sign_reply;
pub mod contract_sign_request;
pub mod dcc_identity;
pub mod ledger_cursor;
pub mod ledger_refresh;
pub mod offerings;
pub mod profiles;
pub mod registration;
pub mod rewards;

pub use account_transfer_approvals::*;
pub use account_transfers::*;
pub use account_transfers_errors::TransferError;
pub use cache_balances::*;
pub use cache_reputation::*;
pub use cache_transactions::*;
use candid::{Nat, Principal};
pub use contract_refund_request::*;
pub use contract_sign_reply::*;
pub use contract_sign_request::*;
pub use dcc_identity::{slice_to_32_bytes_array, slice_to_64_bytes_array};
use icrc_ledger_types::icrc1::account::Account as Icrc1Account;
pub use ledger_cursor::*;
pub use ledger_refresh::*;
use num_traits::cast::ToPrimitive;
pub use offerings::*;
pub use profiles::*;
pub use registration::*;
pub use rewards::*;

#[cfg(not(target_arch = "wasm32"))]
pub mod platform_specific_x86_64;
#[cfg(not(target_arch = "wasm32"))]
pub use platform_specific_x86_64 as platform_specific;

#[cfg(target_arch = "wasm32")]
pub mod platform_specific_wasm32;
#[cfg(target_arch = "wasm32")]
pub use platform_specific_wasm32 as platform_specific;

pub use platform_specific::{get_timestamp_ns, is_test_config, set_test_config, set_timestamp_ns};

pub use dcc_identity::DccIdentity;
#[allow(unused_imports)]
use ledger_map::{debug, error, info, warn};

#[macro_export]
macro_rules! fn_info {
    ($($arg:tt)*) => {
        $crate::info!(
            "[{}]: {}",
            function_name!(),
            format_args!($($arg)*)
        )
    };
}

pub const MINTING_ACCOUNT: IcrcCompatibleAccount = IcrcCompatibleAccount::new_minting();
pub const MINTING_ACCOUNT_PRINCIPAL: Principal = Principal::from_slice(b"MINTING");
pub const MINTING_ACCOUNT_ICRC1: Icrc1Account = Icrc1Account {
    owner: MINTING_ACCOUNT_PRINCIPAL,
    subaccount: None,
};
use std::{collections::HashMap, hash::BuildHasherDefault};
pub type AHashMap<K, V> = HashMap<K, V, BuildHasherDefault<ahash::AHasher>>;

pub const BLOCK_INTERVAL_SECS: u64 = 600;
pub const DC_TOKEN_DECIMALS_DIV: TokenAmountE9s =
    (10u32 as TokenAmountE9s).pow(DC_TOKEN_DECIMALS as u32);
pub const DC_TOKEN_DECIMALS: u8 = 9;
pub const DC_TOKEN_NAME: &str = "Decent Cloud";
pub const DC_TOKEN_SYMBOL: &str = "DC";
pub const DC_TOKEN_TOTAL_SUPPLY: TokenAmountE9s = 21_000_000 * DC_TOKEN_DECIMALS_DIV;
pub const DC_TOKEN_TRANSFER_FEE_E9S: TokenAmountE9s = 1_000_000;
pub const ED25519_SIGNATURE_LENGTH: usize = 64;
pub const ED25519_SIGN_CONTEXT: &[u8] = b"decent-cloud";
pub const FETCH_SIZE_BYTES_DEFAULT: u64 = 1024 * 1024;
pub const KEY_LAST_REWARD_DISTRIBUTION_TS: &[u8] = b"LastRewardNs";
pub const LABEL_DC_TOKEN_APPROVAL: &str = "DCTokenApproval";
pub const LABEL_DC_TOKEN_TRANSFER: &str = "DCTokenTransfer";
pub const LABEL_NP_CHECK_IN: &str = "NPCheckIn";
pub const LABEL_NP_OFFERING: &str = "NPOffering";
pub const LABEL_NP_PROFILE: &str = "NPProfile";
pub const LABEL_NP_REGISTER: &str = "NPRegister";
pub const LABEL_REPUTATION_AGE: &str = "RepAge";
pub const LABEL_REPUTATION_CHANGE: &str = "RepChange";
pub const LABEL_REWARD_DISTRIBUTION: &str = "RewardDistr";
pub const LABEL_USER_REGISTER: &str = "UserRegister";
pub const LABEL_CONTRACT_SIGN_REQUEST: &str = "ContractSignReq";
pub const LABEL_CONTRACT_SIGN_REPLY: &str = "ContractSignReply";
pub const MAX_NP_PROFILE_BYTES: usize = 4 * 1024;
pub const MAX_NP_OFFERING_BYTES: usize = 32 * 1024;
// Maximum response size (replicated execution)	in bytes is 2 MiB
pub const MAX_RESPONSE_BYTES_REPLICATED: usize = 2 * 1024 * 1024;
// Maximum response size (non-replicated execution, i.e. in query calls) in bytes is 3 MiB
// https://internetcomputer.org/docs/current/developer-docs/smart-contracts/maintain/resource-limits#resource-constraints-and-limits
pub const MAX_RESPONSE_BYTES_NON_REPLICATED: usize = 3 * 1024 * 1024;
pub const MAX_PUBKEY_BYTES: usize = 32;
pub const MEMO_BYTES_MAX: usize = 32;
/// Reduction of reputations for all accounts, based on time (per block), in parts per million
pub const REPUTATION_AGING_PPM: u64 = 1_000;
pub const MAX_REPUTATION_INCREASE_PER_TX: i64 = DC_TOKEN_DECIMALS_DIV as i64 * 10; // Max 10 DC tokens per transaction
pub const REWARD_HALVING_AFTER_BLOCKS: u64 = 210_000; // halve the rewards every 210000 reward distributions
pub const DATA_PULL_BYTES_BEFORE_LEN: u16 = 16; // How many bytes before the pulled data should be compared as a quick sanity check

// Default first block's time
// Calculated with:
// python3 -c "from datetime import datetime; print(int(datetime.strptime('2024-01-01 00:00:00', '%Y-%m-%d %H:%M:%S').timestamp()), '* 1_000_000_000')"
pub const FIRST_BLOCK_TIMESTAMP_NS: u64 = 1704063600 * 1_000_000_000;

// Transaction deduplication window (24 hours in nanoseconds)
pub const TX_WINDOW: u64 = 24 * 60 * 60 * 1_000_000_000;
// Permitted time drift for transactions (2 minutes in nanoseconds)
pub const PERMITTED_DRIFT: u64 = 2 * 60 * 1_000_000_000;

pub type TokenAmountE9s = u64;

pub fn nat_to_balance(nat: &Nat) -> TokenAmountE9s {
    nat.0
        .to_u128()
        .map(|n| n.min(TokenAmountE9s::MAX as u128))
        .unwrap_or(0) as TokenAmountE9s
}

pub fn get_account_from_pubkey(pubkey_bytes: &[u8]) -> IcrcCompatibleAccount {
    let dcc_ident = DccIdentity::new_verifying_from_bytes(pubkey_bytes)
        .unwrap_or_else(|_| panic!("Failed to parse pubkey {}", hex::encode(pubkey_bytes)));
    dcc_ident.as_icrc_compatible_account()
}

pub fn common_global_vars_init() {
    account_balances_cache_init();
    account_transfers_cache_init();
    approvals_cache_init();
    contracts_open_cache_init();
    platform_specific::platform_specific_init();
    recent_cache_init();
    registrations_cache_init();
    reputations_cache_init();
    reward_e9s_per_block_cache_init();
}
