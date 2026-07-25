//! Pre-login authentication capability endpoint.
//!
//! The login page needs to know which sign-in methods the server actually
//! supports before rendering the form, so it can default to the right surface
//! (e.g. expand the seed-phrase form when Google OAuth is not configured,
//! avoiding a dead "Sign in with Google" button and an extra click to reach
//! credential sign-in — #436). This module exposes that as a public,
//! unauthenticated endpoint driven by server-side env config.

use super::common::ApiTags;
use poem_openapi::payload::Json;
use poem_openapi::Object;
use poem_openapi::OpenApi;

pub struct AuthApi;

/// Whether each auth method is available on this server. Public — it leaks no
/// secret, only which sign-in surfaces the login page should show.
#[derive(Debug, serde::Serialize, serde::Deserialize, Object)]
pub struct AuthCapabilities {
    /// True only when Google OAuth is fully configured (client id AND secret
    /// both set and non-empty), matching the server's own gate in
    /// `main.rs`/`oauth_simple::create_google_oauth_client`. Mirrors what the
    /// OAuth routes actually require to function.
    pub google_oauth: bool,
}

/// Reports whether a non-empty value is set for `var`. Shared with the unit
/// tests so the enabled/disabled branches can be exercised without depending
/// on whichever OAuth env vars happen to be set in the test environment.
fn env_is_set_and_nonempty(var: &str) -> bool {
    matches!(std::env::var(var), Ok(v) if !v.trim().is_empty())
}

/// Google OAuth is considered available only when BOTH the client id and the
/// client secret are configured. Either alone is insufficient: the OAuth
/// handler (`oauth_simple::create_google_oauth_client`) fails without the id,
/// and the server's own doctor gate (`main.rs` ~line 1036) treats OAuth as
/// functional only when both are present.
pub fn google_oauth_configured() -> bool {
    env_is_set_and_nonempty("GOOGLE_OAUTH_CLIENT_ID")
        && env_is_set_and_nonempty("GOOGLE_OAUTH_CLIENT_SECRET")
}

#[OpenApi]
impl AuthApi {
    /// Auth capabilities
    ///
    /// Reports which authentication methods this server supports, so the login
    /// page can render the right sign-in surface (e.g. default to seed-phrase
    /// sign-in when Google OAuth is not configured). Public/unauthenticated.
    #[oai(path = "/auth/capabilities", method = "get", tag = "ApiTags::Auth")]
    async fn capabilities(&self) -> Json<AuthCapabilities> {
        Json(AuthCapabilities {
            google_oauth: google_oauth_configured(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Snapshot the two OAuth env vars, run `body`, then restore them exactly
    /// (including re-adding ones that were previously unset). Env-mutation
    /// tests must be serial within the process; these run under the default
    /// single-threaded test harness for the api crate's lib tests.
    fn with_env_vars<R>(body: impl FnOnce() -> R) -> R {
        let id_was = std::env::var("GOOGLE_OAUTH_CLIENT_ID").ok();
        let secret_was = std::env::var("GOOGLE_OAUTH_CLIENT_SECRET").ok();
        let result = body();
        match id_was {
            Some(v) => std::env::set_var("GOOGLE_OAUTH_CLIENT_ID", v),
            None => std::env::remove_var("GOOGLE_OAUTH_CLIENT_ID"),
        }
        match secret_was {
            Some(v) => std::env::set_var("GOOGLE_OAUTH_CLIENT_SECRET", v),
            None => std::env::remove_var("GOOGLE_OAUTH_CLIENT_SECRET"),
        }
        result
    }

    #[test]
    fn google_oauth_disabled_when_neither_env_set() {
        with_env_vars(|| {
            std::env::remove_var("GOOGLE_OAUTH_CLIENT_ID");
            std::env::remove_var("GOOGLE_OAUTH_CLIENT_SECRET");
            assert!(!google_oauth_configured());
        });
    }

    #[test]
    fn google_oauth_disabled_when_only_client_id_set() {
        with_env_vars(|| {
            std::env::set_var("GOOGLE_OAUTH_CLIENT_ID", "test-client-id");
            std::env::remove_var("GOOGLE_OAUTH_CLIENT_SECRET");
            assert!(!google_oauth_configured());
        });
    }

    #[test]
    fn google_oauth_disabled_when_value_is_blank() {
        with_env_vars(|| {
            std::env::set_var("GOOGLE_OAUTH_CLIENT_ID", "   ");
            std::env::set_var("GOOGLE_OAUTH_CLIENT_SECRET", "secret");
            assert!(!google_oauth_configured());
        });
    }

    #[test]
    fn google_oauth_enabled_when_both_set_and_nonempty() {
        with_env_vars(|| {
            std::env::set_var("GOOGLE_OAUTH_CLIENT_ID", "test-client-id");
            std::env::set_var("GOOGLE_OAUTH_CLIENT_SECRET", "test-secret");
            assert!(google_oauth_configured());
        });
    }

    #[test]
    fn capabilities_payload_serializes_snake_case() {
        with_env_vars(|| {
            std::env::set_var("GOOGLE_OAUTH_CLIENT_ID", "id");
            std::env::set_var("GOOGLE_OAUTH_CLIENT_SECRET", "secret");
            let caps = AuthCapabilities {
                google_oauth: google_oauth_configured(),
            };
            let json = serde_json::to_value(&caps).unwrap();
            assert_eq!(json["google_oauth"], true);
            // Snake_case contract the frontend depends on — no camelCase drift.
            assert!(json.get("googleOauth").is_none());
        });
    }
}
