#[cfg(all(target_arch = "wasm32", feature = "ic"))]
#[allow(unused_imports)]
use ic_cdk::println;
use std::cell::RefCell;

thread_local! {
    static TEST_CONFIG: RefCell<bool> = const { RefCell::new(false) };
    static CURRENT_TIMESTAMP_NANOS: RefCell<u64> = const { RefCell::new(0) };
}

pub fn is_test_config() -> bool {
    TEST_CONFIG.with(|test_config| *test_config.borrow())
}

pub fn set_test_config(val: bool) {
    TEST_CONFIG.with(|test_config| *test_config.borrow_mut() = val)
}

pub fn get_timestamp_ns() -> u64 {
    let forced_timestamp =
        CURRENT_TIMESTAMP_NANOS.with(|current_timestamp| *current_timestamp.borrow());
    if forced_timestamp > 0 {
        forced_timestamp
    } else {
        ic_cdk::api::time()
    }
}

pub fn set_timestamp_ns(timestamp: u64) {
    if is_test_config() {
        CURRENT_TIMESTAMP_NANOS.with(|current_timestamp| {
            *current_timestamp.borrow_mut() = timestamp;
        });
    }
}
