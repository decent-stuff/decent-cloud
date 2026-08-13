use std::time::Duration;

/// Project-wide HTTP request timeout budget for dc-agent's outbound clients.
///
/// Mirrors `api::http_util::HTTP_TIMEOUT_SECS` and `cli::utils::http::HTTP_TIMEOUT_SECS`:
/// every dc-agent outbound HTTP client (central API client, provisioner backends,
/// Proxmox token-verify) carries this timeout so a slow or stuck peer cannot hang
/// the agent's polling loop indefinitely. Bump here to change it everywhere.
pub const HTTP_TIMEOUT_SECS: Duration = Duration::from_secs(30);

pub mod api_client;
pub mod config;
pub mod doctor;
pub mod gateway;
pub mod geolocation;
pub mod host;
pub mod orphan_tracker;
pub mod ops;
pub mod provisioner;
pub mod registration;
pub mod setup;
pub mod upgrade;
