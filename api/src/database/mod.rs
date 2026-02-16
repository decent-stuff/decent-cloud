/// Default database URL for local development (docker-compose)
pub const DEFAULT_DATABASE_URL: &str = "postgres://test:test@localhost:5432/test";

pub mod accounts;
pub mod acme_dns;
pub mod agent_delegations;
pub mod agent_pools;
pub mod bandwidth;
pub mod chatwoot;
pub mod cloud_accounts;
pub mod cloud_resources;
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
pub mod subscriptions;
pub mod telegram_tracking;
pub mod tokens;
pub mod types;
pub mod users;
pub mod visibility_allowlist;

// Re-export main types
pub use agent_delegations::{AgentDelegation, AgentPermission, AgentStatus};
pub use agent_pools::{AgentPool, AgentPoolWithStats, SetupToken};
pub use cloud_accounts::CloudAccount;
pub use cloud_resources::CloudResourceWithDetails;
pub use notification_config::UserNotificationConfig;
pub use subscriptions::{AccountSubscription, SubscriptionEventInput, SubscriptionPlan};
pub use types::{Database, LedgerEntryData};

// Import all handler implementations

#[cfg(test)]
pub mod test_helpers;

#[cfg(test)]
mod migration_tests;

#[cfg(test)]
mod tests;
