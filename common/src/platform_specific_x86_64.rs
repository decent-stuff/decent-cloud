use crate::TokenAmountE9s;
use icrc_ledger_types::icrc1::account::Account;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

pub static COMMIT_INTERVAL: OnceCell<Duration> = OnceCell::new();
pub static LAST_REWARD_DISTRIBUTION_TIMESTAMP: OnceCell<Arc<Mutex<u64>>> = OnceCell::new();
pub static CURRENT_TIMESTAMP_NANOS: OnceCell<Arc<Mutex<u64>>> = OnceCell::new();

pub fn platform_specific_init() {
    if COMMIT_INTERVAL.get().is_none() {
        COMMIT_INTERVAL.set(Duration::from_secs(10)).unwrap();
    }
    if LAST_REWARD_DISTRIBUTION_TIMESTAMP.get().is_none() {
        LAST_REWARD_DISTRIBUTION_TIMESTAMP
            .set(Arc::new(Mutex::new(0)))
            .unwrap();
    }
    if CURRENT_TIMESTAMP_NANOS.get().is_none() {
        CURRENT_TIMESTAMP_NANOS
            .set(Arc::new(Mutex::new(0)))
            .unwrap();
    }
}

fn current_timestamp_lock() -> tokio::sync::MutexGuard<'static, u64> {
    CURRENT_TIMESTAMP_NANOS
        .get()
        .expect("CURRENT_TIMESTAMP_NANOS not initialized")
        .blocking_lock()
}

pub fn get_timestamp_ns() -> u64 {
    *current_timestamp_lock()
}

pub fn is_test_config() -> bool {
    true
}

pub fn set_test_config(_val: bool) {}

#[allow(dead_code)]
pub fn set_timestamp_ns(timestamp: u64) {
    if !is_test_config() {
        #[cfg(target_arch = "wasm32")]
        ic_cdk::trap("invalid request");
    }
    *current_timestamp_lock() = timestamp;
}

#[allow(dead_code)]
pub(crate) fn get_commit_interval() -> Duration {
    *COMMIT_INTERVAL
        .get()
        .expect("COMMIT_INTERVAL not initialized")
}

pub fn ledger_get_account_balance(_account: Account) -> Result<TokenAmountE9s, String> {
    Ok(0)
}
