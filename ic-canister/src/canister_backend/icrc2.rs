use crate::{canister_backend::generic::LEDGER_MAP, DC_TOKEN_TRANSFER_FEE_E9S};
use candid::Nat;
use dcc_common::{
    account_balance_get, approval_get, approval_update, get_timestamp_ns, ledger_funds_transfer,
    nat_to_balance, FundsTransfer, FundsTransferApproval, IcrcCompatibleAccount, TokenAmountE9s,
    MEMO_BYTES_MAX,
};
use ic_cdk::caller;
use icrc_ledger_types::icrc1::account::Account;
use icrc_ledger_types::icrc2::allowance::{Allowance, AllowanceArgs};
use icrc_ledger_types::icrc2::approve::{ApproveArgs, ApproveError};
use icrc_ledger_types::icrc2::transfer_from::{TransferFromArgs, TransferFromError};

pub fn _icrc2_approve(args: ApproveArgs) -> Result<Nat, ApproveError> {
    // Validate fee
    let fee = nat_to_balance(&args.fee.unwrap_or_default());
    if fee != DC_TOKEN_TRANSFER_FEE_E9S {
        return Err(ApproveError::BadFee {
            expected_fee: DC_TOKEN_TRANSFER_FEE_E9S.into(),
        });
    }

    let caller_principal = caller();
    let from =
        IcrcCompatibleAccount::new(caller_principal, args.from_subaccount.map(|s| s.to_vec()));

    // Prevent self-approval
    if caller_principal == args.spender.owner {
        return Err(ApproveError::GenericError {
            error_code: 1u32.into(),
            message: "Cannot approve transfers to self".to_string(),
        });
    }

    // Check the size of the memo
    if let Some(memo) = &args.memo {
        if memo.0.len() > MEMO_BYTES_MAX {
            return Err(ApproveError::GenericError {
                error_code: 2u32.into(),
                message: "Memo too large".to_string(),
            });
        }
    }

    // Check if caller has sufficient balance for fee
    let balance = account_balance_get(&from);
    if balance < DC_TOKEN_TRANSFER_FEE_E9S {
        return Err(ApproveError::InsufficientFunds {
            balance: balance.into(),
        });
    }

    let now = get_timestamp_ns();

    // Check expiration
    if let Some(expires_at) = args.expires_at {
        if expires_at <= now {
            return Err(ApproveError::Expired { ledger_time: now });
        }
    }

    // Check created_at_time
    if let Some(created_at) = args.created_at_time {
        if created_at > now {
            return Err(ApproveError::CreatedInFuture { ledger_time: now });
        }
    }

    let key = (
        Account {
            owner: caller_principal,
            subaccount: args.from_subaccount,
        },
        args.spender,
    );

    // Check expected_allowance if provided
    if let Some(expected) = args.expected_allowance {
        let current = approval_get(key.0, key.1)
            .map(|a| a.allowance.clone())
            .unwrap_or(Nat::from(0u32));
        if current != expected {
            return Err(ApproveError::AllowanceChanged {
                current_allowance: current.clone(),
            });
        }
    }

    // Update approval
    approval_update(
        key.0,
        key.1,
        Allowance {
            allowance: args.amount.clone(),
            expires_at: args.expires_at,
        },
    );

    // Record approval in ledger
    let approval = FundsTransferApproval::new(
        from.clone(),
        args.spender.into(),
        args.amount
            .min(TokenAmountE9s::MAX.into())
            .0
            .to_u64_digits()
            .first()
            .cloned()
            .unwrap_or_default(),
        args.expires_at,
        DC_TOKEN_TRANSFER_FEE_E9S,
        args.memo.map(|m| m.0.to_vec()).unwrap_or_default(),
        args.created_at_time.unwrap_or(now),
    );

    LEDGER_MAP
        .with(|ledger| {
            let mut ledger = ledger.borrow_mut();
            approval.add_to_ledger(&mut ledger)
        })
        .map_err(|e| ApproveError::GenericError {
            error_code: 133u32.into(),
            message: e.to_string(),
        })
}

pub fn _icrc2_transfer_from(args: TransferFromArgs) -> Result<Nat, TransferFromError> {
    let caller_principal = caller();
    let spender = Account {
        owner: caller_principal,
        subaccount: args.spender_subaccount,
    };

    let from: IcrcCompatibleAccount = args.from.into();
    let to: IcrcCompatibleAccount = args.to.into();
    let amount = nat_to_balance(&args.amount);
    let fee = nat_to_balance(&args.fee.unwrap_or(DC_TOKEN_TRANSFER_FEE_E9S.into()));

    // Check allowance
    let approval = approval_get(from.clone().into(), spender);
    let mut allowed_amount = approval
        .as_ref()
        .map(|a| {
            if let Some(expires_at) = a.expires_at {
                if expires_at <= get_timestamp_ns() {
                    return 0;
                }
            }
            nat_to_balance(&a.allowance)
        })
        .unwrap_or_default();

    if allowed_amount < amount + fee {
        return Err(TransferFromError::InsufficientAllowance {
            allowance: allowed_amount.into(),
        });
    }

    // Check balance
    let balance = account_balance_get(&from);
    if balance < amount + fee {
        return Err(TransferFromError::InsufficientFunds {
            balance: balance.into(),
        });
    }

    // Update allowance
    allowed_amount -= amount + fee;
    approval_update(
        from.clone().into(),
        spender,
        Allowance {
            allowance: allowed_amount.into(),
            expires_at: approval.map(|a| a.expires_at).unwrap_or(None),
        },
    );

    // Execute transfer
    LEDGER_MAP.with(|ledger| {
        let mut ledger = ledger.borrow_mut();
        let balance_from_after = balance - amount - fee;
        let balance_to_after = account_balance_get(&to) + amount;

        let transfer = FundsTransfer::new(
            from.clone(),
            to.clone(),
            Some(fee),
            None,
            Some(args.created_at_time.unwrap_or(get_timestamp_ns())),
            args.memo.map(|m| m.0.to_vec()).unwrap_or_default(),
            amount,
            balance_from_after,
            balance_to_after,
        );

        ledger_funds_transfer(&mut ledger, transfer)
            .map(|_| ledger.get_blocks_count().into())
            .map_err(|e| TransferFromError::GenericError {
                error_code: 0u32.into(),
                message: e.to_string(),
            })
    })
}

pub fn _icrc2_allowance(args: AllowanceArgs) -> Allowance {
    approval_get(args.account, args.spender).unwrap_or(Allowance {
        allowance: 0u32.into(),
        expires_at: None,
    })
}
