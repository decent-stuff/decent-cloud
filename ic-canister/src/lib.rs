pub mod canister_backend;

#[cfg(target_arch = "wasm32")]
mod canister_endpoints;

pub use dcc_common::{
    DC_TOKEN_DECIMALS, DC_TOKEN_DECIMALS_DIV, DC_TOKEN_NAME, DC_TOKEN_SYMBOL,
    DC_TOKEN_TOTAL_SUPPLY, DC_TOKEN_TRANSFER_FEE_E9S, MEMO_BYTES_MAX, MINTING_ACCOUNT,
    MINTING_ACCOUNT_ICRC1,
};
