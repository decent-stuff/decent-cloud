use icrc_ledger_types::icrc1::transfer::BlockIndex;
use ledger_map::LedgerError;

use crate::Icrc1Account;
use crate::NumTokens;
use candid::Nat;

#[derive(Debug)]
pub enum TransferError {
    BadFee {
        expected_fee: NumTokens,
    },
    BadBurn {
        min_burn_amount: NumTokens,
    },
    InsufficientFunds {
        account: Icrc1Account,
        current_balance: NumTokens,
        requested_amount: NumTokens,
    },
    // From amount does not match to amount
    AmountMismatch {
        from_amount: NumTokens,
        to_amount: NumTokens,
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
