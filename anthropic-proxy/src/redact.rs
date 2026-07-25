//! Key-redaction helpers (security-critical).
//!
//! The platform `x-api-key` MUST NEVER appear in logs, error messages, or response
//! bodies. These helpers scrub the secret from any text we surface.

use axum::http::HeaderMap;

/// Header names whose values are sensitive (auth credentials) and must never be
/// logged verbatim, even when redaction of the configured secret is also applied.
pub const SENSITIVE_HEADERS: &[&str] = &["x-api-key", "authorization", "anthropic-version"];

/// Replace every occurrence of `secret` in `value` with `[REDACTED]`.
///
/// An empty secret is a no-op so that tests/dev without a configured key do not
/// destroy unrelated output (e.g. an empty-string redaction would blank everything).
pub fn redact_secret(value: &str, secret: &str) -> String {
    if secret.is_empty() {
        return value.to_string();
    }
    value.replace(secret, "[REDACTED]")
}

/// Render a header map for logging: sensitive header values are replaced wholesale
/// with `[REDACTED]`, and the configured secret is scrubbed anywhere it appears in
/// any other header value.
pub fn redacted_headers(headers: &HeaderMap, secret: &str) -> String {
    let mut parts: Vec<String> = Vec::with_capacity(headers.len());
    for (name, value) in headers.iter() {
        let name_str = name.as_str();
        let rendered = if SENSITIVE_HEADERS.contains(&name_str) {
            "[REDACTED]".to_string()
        } else {
            let s = value.to_str().unwrap_or("<non-ascii>");
            redact_secret(s, secret)
        };
        parts.push(format!("{name_str}: {rendered}"));
    }
    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn redact_secret_replaces_all_occurrences() {
        let out = redact_secret("key=sk-live-123 and key=sk-live-123", "sk-live-123");
        assert_eq!(out, "key=[REDACTED] and key=[REDACTED]");
    }

    #[test]
    fn redact_secret_is_noop_for_empty_secret() {
        let out = redact_secret("plain text", "");
        assert_eq!(out, "plain text");
    }

    #[test]
    fn redact_secret_preserves_surrounding_text() {
        let out = redact_secret("upstream replied: sk-secret-key was used", "sk-secret-key");
        assert_eq!(out, "upstream replied: [REDACTED] was used");
    }

    #[test]
    fn redacted_headers_masks_sensitive_names() {
        let mut h = HeaderMap::new();
        h.insert("x-api-key", HeaderValue::from_static("sk-secret-xyz"));
        h.insert("authorization", HeaderValue::from_static("Bearer evil"));
        h.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        h.insert("content-type", HeaderValue::from_static("application/json"));
        let out = redacted_headers(&h, "sk-secret-xyz");
        // Sensitive values never appear.
        assert!(!out.contains("sk-secret-xyz"));
        assert!(!out.contains("Bearer evil"));
        assert!(!out.contains("2023-06-01"));
        // Masking markers present.
        assert!(out.contains("x-api-key: [REDACTED]"));
        assert!(out.contains("authorization: [REDACTED]"));
        assert!(out.contains("anthropic-version: [REDACTED]"));
        // Non-sensitive header still visible.
        assert!(out.contains("content-type: application/json"));
    }

    #[test]
    fn redacted_headers_scrubs_secret_from_other_headers() {
        let mut h = HeaderMap::new();
        // A non-sensitive header accidentally echoing the secret is still scrubbed.
        h.insert("x-debug", HeaderValue::from_static("echoed sk-secret-xyz here"));
        let out = redacted_headers(&h, "sk-secret-xyz");
        assert!(!out.contains("sk-secret-xyz"));
        assert!(out.contains("[REDACTED]"));
    }

    #[test]
    fn redacted_headers_handles_non_ascii_values() {
        let mut h = HeaderMap::new();
        h.insert("x-binary", HeaderValue::from_bytes(b"\xff\xfe").unwrap());
        let out = redacted_headers(&h, "secret");
        assert!(out.contains("x-binary: <non-ascii>"));
    }
}
