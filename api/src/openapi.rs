pub mod accounts;
pub mod admin;
pub mod chatwoot;
pub mod common;
pub mod contracts;
pub mod offerings;
pub mod providers;
pub mod resellers;
pub mod stats;
pub mod system;
pub mod transfers;
pub mod users;
pub mod validators;
pub mod webhooks;

pub use accounts::AccountsApi;
pub use admin::AdminApi;
pub use chatwoot::ChatwootApi;
pub use contracts::ContractsApi;
pub use offerings::OfferingsApi;
pub use providers::ProvidersApi;
pub use resellers::ResellersApi;
pub use stats::StatsApi;
pub use system::SystemApi;
pub use transfers::TransfersApi;
pub use users::UsersApi;
pub use validators::ValidatorsApi;

use poem_openapi::OpenApi;

/// Combines all API modules into a single OpenAPI specification
pub fn create_combined_api() -> impl OpenApi {
    (
        SystemApi,
        AccountsApi,
        AdminApi,
        ChatwootApi,
        ProvidersApi,
        ValidatorsApi,
        OfferingsApi,
        ContractsApi,
        UsersApi,
        TransfersApi,
        StatsApi,
        ResellersApi,
    )
}
