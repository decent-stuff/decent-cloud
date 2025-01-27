// Standard description: https://github.com/dfinity/ICRC-1/blob/main/standards/ICRC-1/README.md
// Reference implementation: https://github.com/dfinity/ic/blob/master/rs/rosetta-api/icrc1/ledger/src/main.rs

// A principal can have multiple accounts. Each account of a principal is identified by a 32-byte string called subaccount. Therefore an account corresponds to a pair (principal, subaccount).
// The account identified by the subaccount with all bytes set to 0 is the default account of the principal.
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use ic_cdk::println;

use crate::canister_backend::generic::LEDGER_MAP;
use crate::DC_TOKEN_LOGO;
use crate::{
    DC_TOKEN_DECIMALS, DC_TOKEN_NAME, DC_TOKEN_SYMBOL, DC_TOKEN_TOTAL_SUPPLY,
    DC_TOKEN_TRANSFER_FEE_E9S, MEMO_BYTES_MAX, MINTING_ACCOUNT, MINTING_ACCOUNT_ICRC1,
};
use candid::types::number::Nat;
use candid::{CandidType, Principal};
use dcc_common::{
    account_balance_get, cache_transactions::RecentCache, fees_sink_accounts, get_timestamp_ns,
    ledger_funds_transfer, nat_to_balance, AHashMap, FundsTransfer, IcrcCompatibleAccount,
    TokenAmountE9s, PERMITTED_DRIFT, TX_WINDOW,
};
use ic_cdk::caller;
use icrc_ledger_types::icrc::generic_metadata_value::MetadataValue;
use icrc_ledger_types::icrc1::account::Account as Icrc1Account;
use icrc_ledger_types::icrc1::transfer::{Memo as Icrc1Memo, TransferError as Icrc1TransferError};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::cell::RefCell;

// Store transactions with their timestamps for cleanup
thread_local! {
    static RECENT_TRANSACTIONS: RefCell<AHashMap<Vec<u8>, u64>> = RefCell::new(AHashMap::default());
}

fn compute_tx_hash(caller: Principal, arg: &TransferArg) -> Vec<u8> {
    let mut hasher = sha2::Sha256::new();
    hasher.update(caller.as_slice());
    hasher.update(arg.from_subaccount.unwrap_or([0; 32]));
    hasher.update(arg.to.owner.as_slice());
    hasher.update(arg.to.subaccount.unwrap_or([0; 32]));
    hasher.update(arg.amount.0.to_bytes_be());
    if let Some(fee) = &arg.fee {
        hasher.update(fee.0.to_bytes_be());
    }
    if let Some(memo) = &arg.memo {
        hasher.update(&memo.0);
    }
    if let Some(created_at_time) = arg.created_at_time {
        hasher.update(created_at_time.to_be_bytes());
    }
    hasher.finalize().to_vec()
}

pub fn cleanup_old_transactions() {
    RECENT_TRANSACTIONS.with(|transactions| {
        let now = get_timestamp_ns();
        let mut txs = transactions.borrow_mut();
        txs.retain(|_, timestamp| *timestamp > now.saturating_sub(TX_WINDOW));
    });
}

fn check_duplicate_transaction(
    caller: Principal,
    arg: &TransferArg,
    now: u64,
) -> Option<Icrc1TransferError> {
    // Skip deduplication if created_at_time is not set
    arg.created_at_time?;

    let tx_hash = compute_tx_hash(caller, arg);

    RECENT_TRANSACTIONS.with(|transactions| {
        let mut txs = transactions.borrow_mut();

        // Check if this transaction already exists
        if txs.get(&tx_hash).is_some() {
            Some(Icrc1TransferError::Duplicate {
                duplicate_of: Nat::from(0u32), // We don't track the original tx index
            })
        } else {
            // Add new transaction
            txs.insert(tx_hash, now);
            None
        }
    })
}

pub fn _icrc1_metadata() -> Vec<(String, MetadataValue)> {
    vec![
        MetadataValue::entry("icrc1:decimals", DC_TOKEN_DECIMALS as u64),
        MetadataValue::entry("icrc1:name", DC_TOKEN_NAME.to_string()),
        MetadataValue::entry("icrc1:symbol", DC_TOKEN_SYMBOL.to_string()),
        MetadataValue::entry("icrc1:fee", DC_TOKEN_TRANSFER_FEE_E9S),
        MetadataValue::entry("icrc1:logo", DC_TOKEN_LOGO.to_string()),
    ]
}

pub fn _icrc1_balance_of(account: Icrc1Account) -> Nat {
    let account = IcrcCompatibleAccount::from(account);
    account_balance_get(&account).into()
}

pub fn _icrc1_total_supply() -> Nat {
    Nat::from(DC_TOKEN_TOTAL_SUPPLY)
}

pub fn _icrc1_name() -> String {
    DC_TOKEN_NAME.to_string()
}

pub fn _icrc1_symbol() -> String {
    DC_TOKEN_SYMBOL.to_string()
}

pub fn _icrc1_decimals() -> u8 {
    DC_TOKEN_DECIMALS
}

pub fn _icrc1_fee() -> Nat {
    Nat::from(DC_TOKEN_TRANSFER_FEE_E9S)
}

pub fn _icrc1_supported_standards() -> Vec<Icrc1StandardRecord> {
    let supported_standards = vec![
        Icrc1StandardRecord {
            name: "ICRC-1".to_string(),
            url: "https://github.com/dfinity/ICRC-1/tree/main/standards/ICRC-1".to_string(),
        },
        Icrc1StandardRecord {
            name: "ICRC-2".to_string(),
            url: "https://github.com/dfinity/ICRC-1/tree/main/standards/ICRC-2".to_string(),
        },
        // Icrc1StandardRecord {
        //     name: "ICRC-3".to_string(),
        //     url: "https://github.com/dfinity/ICRC-1/tree/main/standards/ICRC-3".to_string(),
        // },
    ];
    supported_standards
}

pub fn _icrc1_minting_account() -> Option<Icrc1Account> {
    Some(MINTING_ACCOUNT_ICRC1)
}

fn validate_transaction_time(created_at_time: Option<u64>) -> Result<(), Icrc1TransferError> {
    if let Some(created_at) = created_at_time {
        let now = get_timestamp_ns();
        // Check if transaction is too old
        if created_at < now.saturating_sub(TX_WINDOW + PERMITTED_DRIFT) {
            return Err(Icrc1TransferError::TooOld);
        }
        // Check if transaction is created in future
        if created_at > now + PERMITTED_DRIFT {
            return Err(Icrc1TransferError::CreatedInFuture { ledger_time: now });
        }
    }
    Ok(())
}

pub fn _icrc1_transfer(arg: TransferArg) -> Result<Nat, Icrc1TransferError> {
    if let Some(memo) = &arg.memo {
        if memo.0.len() > MEMO_BYTES_MAX {
            ic_cdk::trap("the memo field is too large");
        }
    }

    let caller_principal = caller();
    let now = get_timestamp_ns();

    // Validate transaction timing
    validate_transaction_time(arg.created_at_time)?;

    // Check for duplicate transaction
    if let Some(err) = check_duplicate_transaction(caller_principal, &arg, now) {
        return Err(err);
    }
    let from = IcrcCompatibleAccount::new(
        caller_principal,
        arg.from_subaccount.map(|subaccount| subaccount.to_vec()),
    );

    let balance_from = account_balance_get(&from);
    let amount = nat_to_balance(&arg.amount);
    if balance_from < amount {
        return Err(Icrc1TransferError::InsufficientFunds {
            balance: balance_from.into(),
        });
    }
    let balance_from_after = balance_from - amount;
    let to: IcrcCompatibleAccount = arg.to.into();

    LEDGER_MAP.with(|ledger| {
        let fee = nat_to_balance(&arg.fee.unwrap_or_default());
        let balance_to_after: TokenAmountE9s = if to.is_minting_account() {
            if fee != 0 {
                return Err(Icrc1TransferError::BadFee {
                    expected_fee: 0u32.into(),
                });
            }
            let min_burn_amount = DC_TOKEN_TRANSFER_FEE_E9S.min(balance_from_after);
            if amount < min_burn_amount {
                return Err(Icrc1TransferError::BadBurn {
                    min_burn_amount: min_burn_amount.into(),
                });
            }
            0
        } else {
            if fee != DC_TOKEN_TRANSFER_FEE_E9S {
                return Err(Icrc1TransferError::BadFee {
                    expected_fee: DC_TOKEN_TRANSFER_FEE_E9S.into(),
                });
            }
            account_balance_get(&to) + amount
        };
        // It's safe to subtract here because we checked above that the balance will not be negative
        let balance_from_after = balance_from_after.saturating_sub(fee);
        let mut ledger_ref = ledger.borrow_mut();
        let transfer = FundsTransfer::new(
            from,
            to,
            Some(fee),
            Some(fees_sink_accounts()),
            Some(arg.created_at_time.unwrap_or(get_timestamp_ns())),
            arg.memo.unwrap_or_default().0.into_vec(),
            amount,
            balance_from_after,
            balance_to_after,
        );
        ledger_funds_transfer(&mut ledger_ref, transfer.clone())
            .unwrap_or_else(|err| ic_cdk::trap(&err.to_string()));

        RecentCache::append_entry(transfer.into());
        RecentCache::get_max_tx_num().map(Nat::from).ok_or_else(|| {
            Icrc1TransferError::GenericError {
                error_code: Nat::from(10000u32),
                message: "Failed to get max transaction number".to_string(),
            }
        })
    })
}

// test only
pub fn _mint_tokens_for_test(
    account: Icrc1Account,
    amount: TokenAmountE9s,
    memo: Option<Icrc1Memo>,
) -> Nat {
    if !dcc_common::is_test_config() {
        ic_cdk::trap("invalid request");
    }

    LEDGER_MAP.with(|ledger| {
        println!(
            "mint_tokens_for_test: account {} minted {}",
            account, amount
        );
        let balance_to_after = account_balance_get(&account.into()) + amount;
        let mut ledger_ref = ledger.borrow_mut();
        ledger_funds_transfer(
            &mut ledger_ref,
            FundsTransfer::new(
                MINTING_ACCOUNT,
                account.into(),
                None,
                None,
                Some(get_timestamp_ns()),
                memo.unwrap_or_default().0.into_vec(),
                amount,
                0,
                balance_to_after,
            ),
        )
        .unwrap_or_else(|err| ic_cdk::trap(&err.to_string()));

        ledger_ref.get_blocks_count().into()
    })
}

pub type Icrc1Subaccount = [u8; 32];
/// For ICP Ledger compatibility: Position of a block in the chain. The first block has position 0.
pub type BlockIndex = u64;

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TransferArg {
    #[serde(default)]
    pub from_subaccount: Option<Icrc1Subaccount>,
    pub to: Icrc1Account,
    #[serde(default)]
    pub fee: Option<Nat>,
    #[serde(default)]
    pub created_at_time: Option<u64>,
    #[serde(default)]
    pub memo: Option<Icrc1Memo>,
    pub amount: Nat,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Icrc1StandardRecord {
    pub name: String,
    pub url: String,
}
