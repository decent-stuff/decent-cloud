pub mod accounts;
pub mod accounts_billing;
pub mod accounts_contacts;
pub mod accounts_keys;
pub mod admin;
pub mod allowlist;
pub mod agents;
pub mod agents_waitlist;
pub mod auth;
pub mod chatwoot;
pub mod cloud;
pub mod common;
pub mod contract_telemetry;
pub mod contracts;
pub mod email_verification;
pub mod invoices;
pub mod notifications;
pub mod offerings;
pub mod offering_csv;
pub mod offering_stats;
pub mod pools;
pub mod provider_stats;
pub mod providers;
pub mod recovery;
pub mod resellers;
pub mod signature;
pub mod sla;
pub mod stats;
	pub mod system;
	pub mod totp;
	pub mod users;
pub mod validators;
pub mod vat;
pub mod webhooks;
pub mod webhooks_disputes;

pub use accounts::AccountsApi;
pub use accounts_billing::AccountBillingApi;
pub use accounts_contacts::AccountContactsApi;
pub use accounts_keys::AccountKeysApi;
pub use admin::AdminApi;
pub use allowlist::AllowlistApi;
pub use agents::AgentsApi;
pub use agents_waitlist::AgentsWaitlistApi;
pub use auth::AuthApi;
pub use chatwoot::ChatwootApi;
pub use cloud::CloudApi;
pub use contracts::ContractsApi;
pub use contract_telemetry::ContractTelemetryApi;
pub use email_verification::EmailVerificationApi;
pub use invoices::InvoicesApi;
pub use notifications::NotificationsApi;
pub use offering_csv::OfferingCsvApi;
pub use offering_stats::OfferingStatsApi;
pub use offerings::OfferingsApi;
pub use pools::PoolsApi;
pub use provider_stats::ProviderStatsApi;
pub use providers::{contract_status_events, password_reset_events, ProvidersApi};
pub use recovery::RecoveryApi;
pub use resellers::ResellersApi;
pub use sla::SlaApi;
pub use stats::StatsApi;
pub use system::SystemApi;
pub use totp::TotpApi;
pub use users::UsersApi;
pub use validators::ValidatorsApi;
pub use vat::VatApi;

use poem_openapi::OpenApi;

#[cfg(test)]
mod spec_snapshot;

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
            ContractTelemetryApi,
            AccountBillingApi,
        ),
        (
            OfferingsApi,
            ContractsApi,
            InvoicesApi,
            UsersApi,
            StatsApi,
            ResellersApi,
            VatApi,
            AgentsWaitlistApi,
            OfferingCsvApi,
            OfferingStatsApi,
            ProviderStatsApi,
            TotpApi,
            RecoveryApi,
            EmailVerificationApi,
            AccountKeysApi,
            AccountContactsApi,
        ),
    )
}
