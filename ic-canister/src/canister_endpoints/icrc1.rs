use crate::canister_backend::icrc1::*;
use candid::Nat;
#[allow(unused_imports)]
use ic_cdk::println;
use icrc_ledger_types::icrc::generic_metadata_value::MetadataValue;
use icrc_ledger_types::icrc1::account::Account as Icrc1Account;
use icrc_ledger_types::icrc1::transfer::{Memo as Icrc1Memo, TransferError as Icrc1TransferError};

#[ic_cdk::query]
fn icrc1_metadata() -> Vec<(String, MetadataValue)> {
    _icrc1_metadata()
}

#[ic_cdk::query]
fn icrc1_balance_of(account: Icrc1Account) -> Nat {
    _icrc1_balance_of(account)
}

#[ic_cdk::query]
fn icrc1_total_supply() -> Nat {
    _icrc1_total_supply()
}

#[ic_cdk::query]
fn icrc1_name() -> String {
    _icrc1_name()
}

#[ic_cdk::query]
fn icrc1_symbol() -> String {
    _icrc1_symbol()
}

#[ic_cdk::query]
fn icrc1_decimals() -> u8 {
    _icrc1_decimals()
}

#[ic_cdk::query]
fn icrc1_fee() -> Nat {
    _icrc1_fee()
}

#[ic_cdk::query]
fn icrc1_supported_standards() -> Vec<Icrc1StandardRecord> {
    _icrc1_supported_standards()
}

#[ic_cdk::query]
fn icrc1_minting_account() -> Option<Icrc1Account> {
    _icrc1_minting_account()
}

#[ic_cdk::update]
fn icrc1_transfer(arg: TransferArg) -> Result<Nat, Icrc1TransferError> {
    _icrc1_transfer(arg)
}

// test only
#[ic_cdk::update]
fn mint_tokens_for_test(account: Icrc1Account, amount: Nat, memo: Option<Icrc1Memo>) -> Nat {
    _mint_tokens_for_test(account, amount, memo)
}
