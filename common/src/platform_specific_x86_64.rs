use crate::TokenAmount;
use icrc_ledger_types::icrc1::account::Account;
use std::{cell::RefCell, time::Duration};

thread_local! {
    static COMMIT_INTERVAL: Duration = const { Duration::from_secs(10) };
    static LAST_REWARD_DISTRIBUTION_TIMESTAMP: RefCell<u64> = const { RefCell::new(0) };
    static CURRENT_TIMESTAMP_NANOS: RefCell<u64> = const { RefCell::new(0) };
}

pub fn get_timestamp_ns() -> u64 {
    CURRENT_TIMESTAMP_NANOS.with(|timestamp| *timestamp.borrow())
}

pub fn is_test_config() -> bool {
    true
}

pub fn set_test_config(_val: bool) {}

#[allow(dead_code)]
pub fn set_timestamp_ns(timestamp: u64) {
    CURRENT_TIMESTAMP_NANOS.with(|current_timestamp| {
        *current_timestamp.borrow_mut() = timestamp;
    });
}

#[allow(dead_code)]
pub(crate) fn get_commit_interval() -> Duration {
    COMMIT_INTERVAL.with(|commit_interval| *commit_interval)
}

pub fn ledger_get_account_balance(_account: Account) -> Result<TokenAmount, String> {
    Ok(0)
}
