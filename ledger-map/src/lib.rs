//! This module implements a key-value storage system called LedgerMap.
//!
//! The LedgerMap struct provides methods for inserting, deleting, and retrieving key-value entries.
//! It journals the entries in a binary file on x86-64 systems or in stable memory in the
//! Internet Computer canister. Each entry is appended to the file along with its length,
//! allowing efficient retrieval and updates.
//!
//! The LedgerMap struct maintains an in-memory index of the entries for quick lookups. It uses a HashMap (IndexMap)
//! to store the entries, where the key is an enum value representing the label of the entry, and the value
//! is an IndexMap of key-value pairs.
//!
//! The LedgerMap struct also maintains metadata to keep track of the number of entries, the last offset
//! in the persistent storage, and the chain (a.k.a root) hash of the ledger.
//!
//! The LedgerMap struct provides methods for inserting and deleting entries, as well as iterating over the entries
//! by label or in raw form. It also supports re-reading the in-memory index and metadata from the binary file.
//!
//! Entries of LedgerMap are stored in blocks. Each block contains a vector of entries, and the block is committed
//! to the binary file when the user calls the commit_block method. A block also contains metadata such as the
//! offset of block in the persistent storage, the timestamp, and the parent hash.
//!
//! Example usage:
//!
//! ```rust
//! use ledger_map::{LedgerMap};
//! use env_logger::Env;
//!
//! // Set log level to info by default
//! env_logger::try_init_from_env(Env::default().default_filter_or("info")).unwrap();
//!
//! // Optional: Use custom file path for the persistent storage
//! let ledger_path = None;
//! // let ledger_path = Some(std::path::PathBuf::from("/tmp/ledger_map/test_data.bin"));
//!
//! // Create a new LedgerMap instance
//! let mut ledger_map = LedgerMap::new_with_path(None, ledger_path).expect("Failed to create LedgerMap");
//!
//! // Insert a few new entries, each with a separate label
//! ledger_map.upsert("Label1", b"key1".to_vec(), b"value1".to_vec()).unwrap();
//! ledger_map.upsert("Label2", b"key2".to_vec(), b"value2".to_vec()).unwrap();
//! ledger_map.commit_block().unwrap();
//!
//! // Retrieve all entries
//! let entries = ledger_map.iter(None).collect::<Vec<_>>();
//! println!("All entries: {:?}", entries);
//! // Only entries with the Label1 label
//! let entries = ledger_map.iter(Some("Label1")).collect::<Vec<_>>();
//! println!("Label1 entries: {:?}", entries);
//! // Only entries with the Label2 label
//! let entries = ledger_map.iter(Some("Label2")).collect::<Vec<_>>();
//! println!("Label2 entries: {:?}", entries);
//!
//! // Delete an entry
//! ledger_map.delete("Label1", b"key1".to_vec()).unwrap();
//! ledger_map.commit_block().unwrap();
//! // Label1 entries are now empty
//! assert_eq!(ledger_map.iter(Some("Label1")).count(), 0);
//! // Label2 entries still exist
//! assert_eq!(ledger_map.iter(Some("Label2")).count(), 1);
//! ```

#[cfg(all(target_arch = "wasm32", feature = "ic"))]
#[macro_use]
pub mod platform_specific_wasm32_ic;

#[cfg(all(target_arch = "wasm32", feature = "ic"))]
pub use platform_specific_wasm32_ic as platform_specific;

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
#[macro_use]
pub mod platform_specific_x86_64;
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub use platform_specific_x86_64 as platform_specific;

// Core modules
mod errors;
pub mod ledger_entry;
mod ledger_map;
mod metadata;
pub mod partition_table;

// Re-exports
pub use errors::LedgerError;
pub use ledger_entry::{EntryKey, EntryValue, LedgerBlock, LedgerEntry, Operation};
pub use ledger_map::LedgerMap;
pub use metadata::Metadata;

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub use platform_specific::{debug, error, info, warn};
pub use platform_specific::{export_debug, export_error, export_info, export_warn};

// Type aliases
use std::{collections::HashSet, hash::BuildHasherDefault};
pub type AHashSet<K> = HashSet<K, BuildHasherDefault<ahash::AHasher>>;
