#[cfg(all(target_arch = "wasm32", feature = "ic"))]
#[allow(unused_imports)]
use ic_cdk::println;
use icrc_ledger_types::icrc1::transfer::{BlockIndex, TransferError as Icrc1TransferError};
use ledger_map::LedgerError;

use crate::Icrc1Account;
use crate::TokenAmountE9s;
use candid::Nat;

#[derive(Debug)]
pub enum TransferError {
    BadFee {
        expected_fee: TokenAmountE9s,
    },
    BadBurn {
        min_burn_amount: TokenAmountE9s,
    },
    InsufficientFunds {
        account: Icrc1Account,
        current_balance: TokenAmountE9s,
        requested_amount: TokenAmountE9s,
    },
    // From amount does not match to amount
    AmountMismatch {
        from_amount: TokenAmountE9s,
        to_amount: TokenAmountE9s,
    },
    TooOld,
    CreatedInFuture {
        ledger_time: u64,
    },
    TemporarilyUnavailable,
    Duplicate {
        duplicate_of_block: BlockIndex,
    },
    LedgerError {
        message: String,
    },
    SerdeError {
        message: String,
    },
    BalanceError {
        message: String,
    },
    InvalidFeePayer {
        fee_payer_index: u32,
    },
    // Too many accounts provided in a single transaction
    TooManyAccounts {
        max_accounts: u32,
    },
    GenericError {
        error_code: Nat,
        message: String,
    },
}

impl From<TransferError> for Icrc1TransferError {
    fn from(e: TransferError) -> Icrc1TransferError {
        match e {
            TransferError::BadFee { expected_fee } => Icrc1TransferError::BadFee {
                expected_fee: expected_fee.into(),
            },
            TransferError::BadBurn { min_burn_amount } => Icrc1TransferError::BadBurn {
                min_burn_amount: min_burn_amount.into(),
            },
            TransferError::InsufficientFunds {
                current_balance, ..
            } => Icrc1TransferError::InsufficientFunds {
                balance: current_balance.into(),
            },
            TransferError::AmountMismatch {
                from_amount,
                to_amount,
            } => Icrc1TransferError::GenericError {
                error_code: Nat::from(1u8),
                message: format!("Amount mismatch: from {} to {}", from_amount, to_amount),
            },
            TransferError::TooOld => Icrc1TransferError::TooOld {},
            TransferError::CreatedInFuture { ledger_time } => {
                Icrc1TransferError::CreatedInFuture { ledger_time }
            }
            TransferError::TemporarilyUnavailable => Icrc1TransferError::TemporarilyUnavailable {},
            TransferError::Duplicate { duplicate_of_block } => Icrc1TransferError::Duplicate {
                duplicate_of: duplicate_of_block,
            },
            TransferError::LedgerError { message }
            | TransferError::SerdeError { message }
            | TransferError::BalanceError { message } => Icrc1TransferError::GenericError {
                error_code: Nat::from(2u8),
                message,
            },
            TransferError::InvalidFeePayer { fee_payer_index } => {
                Icrc1TransferError::GenericError {
                    error_code: Nat::from(3u8),
                    message: format!("Invalid fee payer index: {}", fee_payer_index),
                }
            }
            TransferError::TooManyAccounts { max_accounts } => Icrc1TransferError::GenericError {
                error_code: Nat::from(4u8),
                message: format!("Too many accounts. Maximum allowed: {}", max_accounts),
            },
            TransferError::GenericError {
                error_code,
                message,
            } => Icrc1TransferError::GenericError {
                error_code,
                message,
            },
        }
    }
}

impl std::fmt::Display for TransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferError::BadFee { expected_fee } => {
                write!(f, "BadFee: expected fee is {}", expected_fee)
            }
            TransferError::BadBurn { min_burn_amount } => {
                write!(f, "BadBurn: min burn amount is {}", min_burn_amount)
            }
            TransferError::InsufficientFunds {
                account,
                current_balance,
                requested_amount,
            } => write!(
                f,
                "InsufficientFunds: account {} has {} and requested {}",
                account, current_balance, requested_amount
            ),
            TransferError::AmountMismatch {
                from_amount,
                to_amount,
            } => write!(
                f,
                "AmountMismatch: from amount {} does not match to amount {}",
                from_amount, to_amount
            ),
            TransferError::TooOld => write!(f, "TooOld"),
            TransferError::CreatedInFuture { ledger_time } => {
                write!(f, "CreatedInFuture: ledger time is {}", ledger_time)
            }
            TransferError::TemporarilyUnavailable => write!(f, "TemporarilyUnavailable"),
            TransferError::Duplicate { duplicate_of_block } => {
                write!(f, "Duplicate: block {} is a duplicate", duplicate_of_block)
            }
            TransferError::LedgerError { message } => write!(f, "LedgerError: {}", message),
            TransferError::SerdeError { message } => write!(f, "SerdeError: {}", message),
            TransferError::BalanceError { message } => write!(f, "BalanceError: {}", message),
            TransferError::InvalidFeePayer { fee_payer_index } => {
                write!(f, "InvalidFeePayer: fee payer index is {}", fee_payer_index)
            }
            TransferError::TooManyAccounts { max_accounts } => {
                write!(f, "TooManyAccounts: max accounts is {}", max_accounts)
            }
            TransferError::GenericError {
                error_code,
                message,
            } => {
                write!(
                    f,
                    "GenericError: error code is {} and message is {}",
                    error_code, message
                )
            }
        }
    }
}

impl From<LedgerError> for TransferError {
    fn from(e: LedgerError) -> Self {
        Self::LedgerError {
            message: e.to_string(),
        }
    }
}

impl From<borsh::io::Error> for TransferError {
    fn from(e: borsh::io::Error) -> Self {
        Self::SerdeError {
            message: e.to_string(),
        }
    }
}
