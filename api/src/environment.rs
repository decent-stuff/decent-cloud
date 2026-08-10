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

/// Reads `ENVIRONMENT` and reports whether this process is running in
/// production. Centralizes the env-var read so callers (account creation,
/// rate limiter, Stripe-at-boot, …) cannot drift on the default value.
///
/// Missing `ENVIRONMENT` is treated as non-production (`dev`) — matching the
/// slim local stack and the test runner, neither of which sets it.
pub fn is_production_env() -> bool {
    let environment =
        std::env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string());
    is_production(&environment)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

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

    // is_production_env reads/writes the process-wide ENVIRONMENT var, so these
    // must not run concurrently with each other (or with any other test that
    // touches ENVIRONMENT).
    #[test]
    #[serial(env)]
    fn test_is_production_env_prod() {
        std::env::set_var("ENVIRONMENT", "prod");
        assert!(is_production_env());
        std::env::remove_var("ENVIRONMENT");
    }

    #[test]
    #[serial(env)]
    fn test_is_production_env_non_prod() {
        // Unset → treated as dev (non-prod): the local/test stack never sets it.
        std::env::remove_var("ENVIRONMENT");
        assert!(!is_production_env());

        std::env::set_var("ENVIRONMENT", "dev");
        assert!(!is_production_env());

        std::env::set_var("ENVIRONMENT", "test");
        assert!(!is_production_env());

        std::env::remove_var("ENVIRONMENT");
    }
}
