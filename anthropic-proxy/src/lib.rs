//! Anthropic API key reverse proxy (GitHub issue #427).
//!
//! Listens for Anthropic Messages-API calls from a customer container, strips any
//! client-supplied auth header, injects the platform's `x-api-key` upstream, meters
//! token usage per identity, and streams the response back unchanged. The key NEVER
//! enters the customer container.
//!
//! Spec: `docs/specs/2026-04-25-decent-agents-identity-provisioning-spec.md`
//! sections F.3 (Anthropic key), G (metering handoff to #415), H (key strategy),
//! I.1 (1 customer = 1 VM for beta).

pub mod config;
pub mod identity;
pub mod metering;
pub mod proxy;
pub mod redact;

pub use config::Config;
pub use identity::{IdentityRef, IdentityResolver, SharedResolver, SingleIdentityResolver};
pub use metering::{InMemoryRecorder, LoggingRecorder, MeteringRecorder, SharedRecorder, Usage};
pub use proxy::{build_app, proxy_handler, ProxyState};
