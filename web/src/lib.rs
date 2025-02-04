// #[cfg(target_arch = "wasm32")]
pub mod wasm;

pub(crate) use ledger_map::platform_specific::*;
pub(crate) use ledger_map::{info, LedgerBlock, LedgerEntry, LedgerError, LedgerMap};
