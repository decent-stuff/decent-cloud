use crate::canister_backend::icrc2::{_icrc2_allowance, _icrc2_approve, _icrc2_transfer_from};
use candid::Nat;
use ic_cdk::{query, update};
use icrc_ledger_types::icrc2::allowance::{Allowance, AllowanceArgs};
use icrc_ledger_types::icrc2::approve::{ApproveArgs, ApproveError};
use icrc_ledger_types::icrc2::transfer_from::{TransferFromArgs, TransferFromError};

#[update]
fn icrc2_approve(args: ApproveArgs) -> Result<Nat, ApproveError> {
    _icrc2_approve(args)
}

#[update]
fn icrc2_transfer_from(args: TransferFromArgs) -> Result<Nat, TransferFromError> {
    _icrc2_transfer_from(args)
}

#[query]
fn icrc2_allowance(args: AllowanceArgs) -> Allowance {
    _icrc2_allowance(args)
}
