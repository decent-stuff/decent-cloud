//! Proxy configuration + deploy-time validation.
//!
//! Validates eagerly at startup (spec rule: "validate at startup, never at request
//! time"). A missing `ANTHROPIC_API_KEY` or identity refuses to start, loudly.

use anyhow::{Context, Result};

/// Validated proxy configuration.
#[derive(Debug, Clone)]
pub struct Config {
    pub listen_addr: String,
    /// Upstream Anthropic-compatible base URL, WITHOUT a trailing slash and WITHOUT
    /// a path like `/v1/messages` (the proxy is path-transparent). Examples:
    ///   production: https://api.anthropic.com
    ///   dev/test:   https://api.z.ai/api/anthropic
    pub upstream: String,
    /// The platform Anthropic API key. Held only in proxy memory; never enters the
    /// customer container and never appears in logs (see `redact`).
    pub api_key: String,
    /// `anthropic-version` header value injected on every upstream request.
    pub anthropic_version: String,
    /// The single identity this proxy host serves (spec I.1: 1 customer = 1 VM).
    pub identity_id: String,
}

impl Config {
    /// Validate and build a `Config` from explicit fields.
    pub fn new(
        listen_addr: impl Into<String>,
        upstream: impl Into<String>,
        api_key: impl Into<String>,
        anthropic_version: impl Into<String>,
        identity_id: impl Into<String>,
    ) -> Result<Self> {
        let listen_addr = listen_addr.into();
        let upstream = upstream.into();
        let api_key = api_key.into();
        let anthropic_version = anthropic_version.into();
        let identity_id = identity_id.into();

        if listen_addr.trim().is_empty() {
            anyhow::bail!("listen address is required (--listen, e.g. 127.0.0.1:8787)");
        }
        if api_key.trim().is_empty() {
            anyhow::bail!(
                "ANTHROPIC_API_KEY is required — the proxy cannot inject a key it does not hold. \
                 Set ANTHROPIC_API_KEY (env) or pass --api-key. The proxy will NOT start without it."
            );
        }
        if identity_id.trim().is_empty() {
            anyhow::bail!(
                "identity is required (--identity <id>) — every proxied request must be attributable \
                 to an identity for metering (spec section G). The proxy will NOT start without it."
            );
        }
        if anthropic_version.trim().is_empty() {
            anyhow::bail!("anthropic-version is required (default: 2023-06-01)");
        }
        let upstream = upstream.trim_end_matches('/').to_string();
        let parsed = url::Url::parse(&upstream)
            .with_context(|| format!("invalid upstream URL: {upstream:?}"))?;
        match parsed.scheme() {
            "http" | "https" => {}
            other => anyhow::bail!(
                "invalid upstream URL scheme {other:?}: only http/https are supported (got {upstream:?})"
            ),
        }
        if parsed.host_str().is_none() {
            anyhow::bail!("invalid upstream URL {upstream:?}: missing host");
        }

        Ok(Self {
            listen_addr,
            upstream,
            api_key,
            anthropic_version,
            identity_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_args() -> (&'static str, &'static str, &'static str, &'static str, &'static str) {
        ("127.0.0.1:8787", "https://api.anthropic.com", "sk-test-key", "2023-06-01", "id-1")
    }

    #[test]
    fn valid_config_builds_and_strips_trailing_slash() {
        let (l, _u, k, v, i) = base_args();
        let c = Config::new(l, "https://api.anthropic.com/", k, v, i).unwrap();
        assert_eq!(c.upstream, "https://api.anthropic.com");
        assert_eq!(c.listen_addr, l);
        assert_eq!(c.api_key, k);
        assert_eq!(c.identity_id, i);
    }

    #[test]
    fn missing_key_is_rejected_loudly() {
        let (l, u, _k, v, i) = base_args();
        let err = Config::new(l, u, "   ", v, i).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("ANTHROPIC_API_KEY"), "error should name the env var: {msg}");
        assert!(msg.contains("NOT start"), "error should be loud: {msg}");
    }

    #[test]
    fn missing_identity_is_rejected() {
        let (l, u, k, v, _i) = base_args();
        let err = Config::new(l, u, k, v, "").unwrap_err();
        assert!(format!("{err}").contains("identity"));
    }

    #[test]
    fn invalid_upstream_scheme_is_rejected() {
        let (l, _u, k, v, i) = base_args();
        let err = Config::new(l, "ftp://example.com", k, v, i).unwrap_err();
        assert!(format!("{err}").contains("scheme"));
    }

    #[test]
    fn invalid_upstream_url_is_rejected() {
        let (l, _u, k, v, i) = base_args();
        let err = Config::new(l, "not a url at all", k, v, i).unwrap_err();
        assert!(format!("{err}").contains("upstream"));
    }

    #[test]
    fn zai_dev_upstream_shape_is_accepted() {
        let (l, _u, k, v, i) = base_args();
        let c = Config::new(l, "https://api.z.ai/api/anthropic", k, v, i).unwrap();
        assert_eq!(c.upstream, "https://api.z.ai/api/anthropic");
    }
}
