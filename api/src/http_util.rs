//! Shared HTTP client constructor enforcing a project-wide request timeout.
//!
//! Every outbound HTTP call in the backend MUST go through a client with a
//! timeout so a slow or stuck peer cannot hang a request handler indefinitely.
//! Use [`http_client`] instead of `reqwest::Client::new()` (which has no
//! timeout). The cloud-provider backends in `crate::cloud` already follow this
//! convention via `.timeout(REQUEST_TIMEOUT_SECS)` on the builder.

use std::time::Duration;

const HTTP_TIMEOUT_SECS: u64 = 30;

/// Build a `reqwest::Client` with the standard backend-wide request timeout.
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
