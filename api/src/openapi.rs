pub mod accounts;
pub mod admin;
pub mod allowlist;
pub mod agents;
pub mod agents_waitlist;
pub mod auth;
pub mod chatwoot;
pub mod cloud;
pub mod common;
pub mod contracts;
pub mod invoices;
pub mod notifications;
pub mod offerings;
pub mod offering_csv;
pub mod pools;
pub mod provider_stats;
pub mod providers;
pub mod resellers;
pub mod signature;
pub mod sla;
pub mod stats;
pub mod subscriptions;
pub mod system;
pub mod transfers;
pub mod users;
pub mod validators;
pub mod vat;
pub mod webhooks;

pub use accounts::AccountsApi;
pub use admin::AdminApi;
pub use allowlist::AllowlistApi;
pub use agents::AgentsApi;
pub use agents_waitlist::AgentsWaitlistApi;
pub use auth::AuthApi;
pub use chatwoot::ChatwootApi;
pub use cloud::CloudApi;
pub use contracts::ContractsApi;
pub use invoices::InvoicesApi;
pub use notifications::NotificationsApi;
pub use offering_csv::OfferingCsvApi;
pub use offerings::OfferingsApi;
pub use pools::PoolsApi;
pub use provider_stats::ProviderStatsApi;
pub use providers::{contract_status_events, password_reset_events, ProvidersApi};
pub use resellers::ResellersApi;
pub use sla::SlaApi;
pub use stats::StatsApi;
pub use subscriptions::SubscriptionsApi;
pub use system::SystemApi;
pub use transfers::TransfersApi;
pub use users::UsersApi;
pub use validators::ValidatorsApi;
pub use vat::VatApi;

use poem_openapi::OpenApi;

/// Combines all API modules into a single OpenAPI specification
pub fn create_combined_api() -> impl OpenApi {
    (
        (
            SystemApi,
            AuthApi,
            AccountsApi,
            AdminApi,
            AgentsApi,
            ChatwootApi,
            CloudApi,
            ProvidersApi,
            ValidatorsApi,
            PoolsApi,
            NotificationsApi,
            SlaApi,
            AllowlistApi,
        ),
        (
            OfferingsApi,
            ContractsApi,
            InvoicesApi,
            UsersApi,
            TransfersApi,
            StatsApi,
            ResellersApi,
            SubscriptionsApi,
            VatApi,
            AgentsWaitlistApi,
            OfferingCsvApi,
            ProviderStatsApi,
        ),
    )
}
