//! Single source of truth for environment classification.
//!
//! The production manifest sets `ENVIRONMENT=prod`. Every production-gated code
//! path (rate limiter default, Stripe-required-at-boot, CORS) MUST go through
//! [`is_production`] so the gating can never drift — historically the rate limiter
//! checked `== "production"` while CORS checked `== "prod"`, silently leaving
//! rate limiting OFF in production (smoke finding 2026-08-03).
//!
//! `"production"` is intentionally NOT recognized: no manifest emits that value,
//! and accepting both would re-introduce the silent-mismatch class of bug.

/// Returns `true` when `environment` denotes a production deployment.
///
/// Canonical value: `"prod"`. Other values (`dev`, `test`, `stage`, `play`, …)
/// are non-production.
pub fn is_production(environment: &str) -> bool {
    environment == "prod"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prod_is_production() {
        assert!(is_production("prod"));
    }

    #[test]
    fn test_non_prod_values_are_not_production() {
        // Every value a manifest actually emits for non-prod envs.
        assert!(!is_production("dev"));
        assert!(!is_production("test"));
        assert!(!is_production("stage"));
        assert!(!is_production("play"));
    }

    #[test]
    fn test_legacy_literal_is_not_recognized() {
        // "production" is NOT a value any manifest emits; accepting it would
        // re-create the silent-mismatch footgun. Only `prod` counts.
        assert!(!is_production("production"));
        assert!(!is_production(""));
    }
}
