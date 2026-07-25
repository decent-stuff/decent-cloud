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

/// True only when both values are present and non-empty (after trim). Factored
/// out so the branch logic can be unit-tested without mutating process-global
/// env vars, which race under parallel test execution.
fn oauth_configured_from(client_id: Option<&str>, client_secret: Option<&str>) -> bool {
    let id = client_id.map(str::trim).filter(|s| !s.is_empty());
    let secret = client_secret.map(str::trim).filter(|s| !s.is_empty());
    id.is_some() && secret.is_some()
}

/// Google OAuth is considered available only when BOTH the client id and the
/// client secret are configured. Either alone is insufficient: the OAuth
/// handler (`oauth_simple::create_google_oauth_client`) fails without the id,
/// and the server's own doctor gate (`main.rs` ~line 1036) treats OAuth as
/// functional only when both are present.
pub fn google_oauth_configured() -> bool {
    oauth_configured_from(
        std::env::var("GOOGLE_OAUTH_CLIENT_ID").ok().as_deref(),
        std::env::var("GOOGLE_OAUTH_CLIENT_SECRET").ok().as_deref(),
    )
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

    // Branch logic is tested against the pure helper (`oauth_configured_from`)
    // rather than by mutating GOOGLE_OAUTH_* env vars: those are process-global
    // and the binary test harness runs in parallel, so env-mutation tests race
    // with each other and with oauth_simple::tests. The live env-read path
    // (google_oauth_configured) is covered end-to-end by the auth-capabilities
    // e2e spec, which asserts the e2e stack reports google_oauth=false.

    #[test]
    fn oauth_disabled_when_neither_credential_provided() {
        assert!(!oauth_configured_from(None, None));
    }

    #[test]
    fn oauth_disabled_when_only_client_id_provided() {
        assert!(!oauth_configured_from(Some("test-client-id"), None));
    }

    #[test]
    fn oauth_disabled_when_only_client_secret_provided() {
        assert!(!oauth_configured_from(None, Some("test-secret")));
    }

    #[test]
    fn oauth_disabled_when_a_credential_is_blank() {
        // Whitespace-only counts as missing — matches the trim+is_empty gate.
        assert!(!oauth_configured_from(Some("   "), Some("secret")));
        assert!(!oauth_configured_from(Some("id"), Some("  ")));
    }

    #[test]
    fn oauth_enabled_when_both_credentials_provided() {
        assert!(oauth_configured_from(Some("client-id"), Some("secret")));
    }

    #[test]
    fn capabilities_payload_serializes_snake_case() {
        // The frontend reads data.google_oauth; guard against camelCase drift.
        let json = serde_json::to_value(&AuthCapabilities { google_oauth: true }).unwrap();
        assert_eq!(json["google_oauth"], true);
        assert!(json.get("googleOauth").is_none());

        let json = serde_json::to_value(&AuthCapabilities { google_oauth: false }).unwrap();
        assert_eq!(json["google_oauth"], false);
    }
}
