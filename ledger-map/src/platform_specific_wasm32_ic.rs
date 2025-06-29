/// This module contains functionalities specific to the WebAssembly (WASM) 32-bit builds for the Internet Computer.
/// It provides implementations and abstractions unique to the environment.
///
pub use crate::{debug, error, info, warn}; // created in the crate root by macro_export
pub use ic_canister_log::log;
use ic_canister_log::{declare_log_buffer, export, LogEntry};
#[allow(unused_imports)]
use ic_cdk::println;

// Keep up to "capacity" last messages.
declare_log_buffer!(name = DEBUG, capacity = 10000);
declare_log_buffer!(name = INFO, capacity = 10000);
declare_log_buffer!(name = WARN, capacity = 10000);
declare_log_buffer!(name = ERROR, capacity = 10000);

#[macro_export]
macro_rules! debug {
    ($message:expr $(,$args:expr)* $(,)*) => {{
        $crate::platform_specific_wasm32_ic::log!($crate::platform_specific_wasm32_ic::DEBUG, $message $(,$args)*);
    }}
}

#[macro_export]
macro_rules! info {
    ($message:expr $(,$args:expr)* $(,)*) => {{
        $crate::platform_specific_wasm32_ic::log!($crate::platform_specific_wasm32_ic::INFO, $message $(,$args)*);
    }}
}

#[macro_export]
macro_rules! warn {
    ($message:expr $(,$args:expr)* $(,)*) => {{
        $crate::platform_specific_wasm32_ic::log!($crate::platform_specific_wasm32_ic::WARN, $message $(,$args)*);
    }}
}

#[macro_export]
macro_rules! error {
    ($message:expr $(,$args:expr)* $(,)*) => {{
        $crate::platform_specific_wasm32_ic::log!($crate::platform_specific_wasm32_ic::ERROR, $message $(,$args)*);
    }}
}

pub fn export_debug() -> Vec<LogEntry> {
    export(&DEBUG)
}

pub fn export_info() -> Vec<LogEntry> {
    export(&INFO)
}

pub fn export_warn() -> Vec<LogEntry> {
    export(&WARN)
}

pub fn export_error() -> Vec<LogEntry> {
    export(&ERROR)
}

pub const PERSISTENT_STORAGE_PAGE_SIZE: u64 = 64 * 1024;

pub fn persistent_storage_size_bytes() -> u64 {
    ic_cdk::stable::stable_size() * PERSISTENT_STORAGE_PAGE_SIZE
}

pub fn persistent_storage_last_valid_offset() -> u64 {
    persistent_storage_size_bytes()
}

pub fn persistent_storage_read(offset: u64, buf: &mut [u8]) -> Result<(), String> {
    ic_cdk::stable::stable_read(offset, buf);
    Ok(())
}

pub fn persistent_storage_write(offset: u64, buf: &[u8]) {
    let stable_memory_size_bytes = persistent_storage_size_bytes();
    if stable_memory_size_bytes < offset + buf.len() as u64 {
        let stable_memory_bytes_new = offset + (buf.len() as u64).max(PERSISTENT_STORAGE_PAGE_SIZE);
        persistent_storage_grow(
            (stable_memory_bytes_new - stable_memory_size_bytes) / PERSISTENT_STORAGE_PAGE_SIZE + 1,
        )
        .unwrap();
    }
    ic_cdk::stable::stable_write(offset, buf)
}

pub fn persistent_storage_grow(additional_pages: u64) -> Result<u64, String> {
    info!(
        "persistent_storage_grow: {} additional_pages.",
        additional_pages
    );
    ic_cdk::stable::stable_grow(additional_pages).map_err(|err| format!("{:?}", err))
}

pub(crate) fn get_timestamp_nanos() -> u64 {
    ic_cdk::api::time()
}
