pub mod accounts;
pub mod chatwoot;
pub mod contracts;
pub mod core;
pub mod email;
pub mod handlers;
pub mod messages;
pub mod offerings;
pub mod providers;
pub mod recovery;
pub mod reputation;
pub mod rewards;
pub mod stats;
pub mod tokens;
pub mod types;
pub mod users;

// Re-export main types
pub use chatwoot::{ProviderResponseMetrics, ResponseTimeDistribution, SlaBreach};
pub use types::{Database, LedgerEntryData};

// Import all handler implementations

#[cfg(test)]
pub mod test_helpers;

#[cfg(test)]
mod tests;
