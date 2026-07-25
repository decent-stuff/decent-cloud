//! Shared HTTP client constructor enforcing a CLI-wide request timeout.
//!
//! Mirrors the api crate's `http_util::http_client`: every outbound HTTP call
//! in the CLI MUST go through a client with a timeout so a slow or stuck API
//! peer cannot hang a user-facing command indefinitely. Use [`http_client`]
//! instead of `reqwest::Client::new()` (which has no timeout).

use std::time::Duration;

const HTTP_TIMEOUT_SECS: u64 = 30;

/// Build a `reqwest::Client` with the standard CLI-wide request timeout.
///
/// Mirrors the semantics of `reqwest::Client::new()` (panics only on TLS-backend
/// initialization failure, which is effectively a startup/compile-time error and
/// never occurs at runtime with valid config) while adding a 30s timeout.
pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()
        .expect("reqwest client with default config is always buildable")
}
