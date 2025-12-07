pub mod accounts;
pub mod chatwoot;
pub mod contracts;
pub mod core;
pub mod email;
pub mod handlers;
pub mod notification_config;
pub mod offerings;
pub mod providers;
pub mod recovery;
pub mod reputation;
pub mod reseller;
pub mod rewards;
pub mod stats;
pub mod telegram_tracking;
pub mod tokens;
pub mod types;
pub mod users;

// Re-export main types
pub use notification_config::UserNotificationConfig;
pub use types::{Database, LedgerEntryData};

// Import all handler implementations

#[cfg(test)]
pub mod test_helpers;

#[cfg(test)]
mod tests;
