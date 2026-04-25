//! Decent Agents beta waitlist signup endpoint (issue #423).
//!
//! Public, unauthenticated. Soft-launch gating only — captures email +
//! GitHub handle so we can hand-invite ~20 customers before opening the
//! self-serve flow. Rate limiting is handled by the global Strict tier
//! configured in `crate::rate_limit` (10 req/min/IP for unauthenticated POST).
use crate::database::Database;
use poem::web::Data;
use poem_openapi::{payload::Json, Object, OpenApi};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Local OpenAPI tag — kept module-local so the public `ApiTags` enum in
/// `super::common` does not need a new variant for a soft-launch-only feature.
#[derive(poem_openapi::Tags)]
enum ApiTags {
    /// Decent Agents beta waitlist signup
    AgentsWaitlist,
}

/// Request body posted by the Decent Agents landing page.
#[derive(Debug, Deserialize, Serialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AgentsWaitlistRequest {
    pub email: String,
    /// GitHub username (1-39 chars, alphanumeric or hyphen).
    #[oai(rename = "github_handle")]
    #[serde(rename = "github_handle")]
    pub github_handle: String,
    /// Optional acquisition channel (e.g. "landing", "linkedin", "hn").
    #[oai(rename = "source")]
    #[serde(default, rename = "source")]
    pub source: Option<String>,
}

/// Response: `{ok: true, position: <count>}` or `{ok: false, error: "..."}`.
#[derive(Debug, Serialize, Object)]
#[oai(skip_serializing_if_is_none)]
pub struct AgentsWaitlistResponse {
    pub ok: bool,
    /// 1-based position in the waitlist (only present on success).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub position: Option<i64>,
    /// Human-readable error (only present on failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub error: Option<String>,
}

impl AgentsWaitlistResponse {
    fn ok(position: i64) -> Self {
        Self {
            ok: true,
            position: Some(position),
            error: None,
        }
    }
    fn err(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            position: None,
            error: Some(msg.into()),
        }
    }
}

pub struct AgentsWaitlistApi;

#[OpenApi]
impl AgentsWaitlistApi {
    /// Sign up for the Decent Agents beta waitlist.
    ///
    /// Public, unauthenticated. Idempotent on email — duplicate retries
    /// return the same row's position rather than an error.
    #[oai(
        path = "/agents-waitlist",
        method = "post",
        tag = "ApiTags::AgentsWaitlist"
    )]
    async fn signup(
        &self,
        db: Data<&Arc<Database>>,
        req: Json<AgentsWaitlistRequest>,
    ) -> Json<AgentsWaitlistResponse> {
        let email = req.email.trim();
        let handle = req.github_handle.trim();
        let source = req.source.as_deref().map(str::trim).filter(|s| !s.is_empty());

        if let Err(e) = crate::validation::validate_email(email) {
            return Json(AgentsWaitlistResponse::err(format!("invalid email: {}", e)));
        }
        if let Err(e) = validate_github_handle(handle) {
            return Json(AgentsWaitlistResponse::err(format!(
                "invalid github_handle: {}",
                e
            )));
        }
        if let Some(s) = source {
            if s.len() > 64 {
                return Json(AgentsWaitlistResponse::err(
                    "source must be at most 64 characters",
                ));
            }
        }

        match db.add_to_waitlist(email, handle, source).await {
            Ok((_entry, position)) => {
                tracing::info!(email = %email, github = %handle, position, "agents waitlist signup");
                Json(AgentsWaitlistResponse::ok(position))
            }
            Err(e) => {
                tracing::error!(error = ?e, "agents waitlist signup failed");
                Json(AgentsWaitlistResponse::err(format!(
                    "failed to record signup: {}",
                    e
                )))
            }
        }
    }
}

/// GitHub username rules: 1-39 chars, alphanumeric or hyphen, cannot start
/// or end with a hyphen, no consecutive hyphens. Mirrors GitHub's own
/// validation closely enough to reject obvious garbage at the door.
fn validate_github_handle(s: &str) -> Result<(), &'static str> {
    if s.is_empty() {
        return Err("must not be empty");
    }
    if s.len() > 39 {
        return Err("must be at most 39 characters");
    }
    if s.starts_with('-') || s.ends_with('-') {
        return Err("must not start or end with a hyphen");
    }
    if s.contains("--") {
        return Err("must not contain consecutive hyphens");
    }
    if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("must contain only alphanumeric characters and hyphens");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_github_handle_valid() {
        for ok in &["a", "octocat", "user-name", "u1-2-3", "A1B2C3", &"a".repeat(39)] {
            assert!(
                validate_github_handle(ok).is_ok(),
                "expected {ok:?} to be valid"
            );
        }
    }

    #[test]
    fn test_validate_github_handle_rejects_bad_input() {
        let cases: &[(&str, &str)] = &[
            ("", "must not be empty"),
            ("-leading", "start or end"),
            ("trailing-", "start or end"),
            ("double--hyphen", "consecutive hyphens"),
            ("with space", "alphanumeric"),
            ("with_underscore", "alphanumeric"),
            ("dot.in.name", "alphanumeric"),
        ];
        for (bad, expected_substr) in cases {
            let err = validate_github_handle(bad).expect_err(&format!("expected {bad:?} invalid"));
            assert!(
                err.contains(expected_substr),
                "for {bad:?}: error {err:?} should contain {expected_substr:?}"
            );
        }
        // Length: 40 chars
        let too_long = "a".repeat(40);
        let err = validate_github_handle(&too_long).expect_err("40 chars should be too long");
        assert!(err.contains("at most 39"));
    }

    #[test]
    fn test_request_deserialization_snake_case() {
        // The landing page posts snake_case (github_handle), not camelCase.
        let json = r#"{"email":"a@b.com","github_handle":"octocat"}"#;
        let req: AgentsWaitlistRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.email, "a@b.com");
        assert_eq!(req.github_handle, "octocat");
        assert!(req.source.is_none());
    }

    #[test]
    fn test_request_deserialization_with_source() {
        let json = r#"{"email":"a@b.com","github_handle":"octocat","source":"hn"}"#;
        let req: AgentsWaitlistRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.source.as_deref(), Some("hn"));
    }

    #[test]
    fn test_response_ok_serialization_omits_error() {
        let resp = AgentsWaitlistResponse::ok(7);
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["position"], 7);
        assert!(json.get("error").is_none());
    }

    #[test]
    fn test_response_err_serialization_omits_position() {
        let resp = AgentsWaitlistResponse::err("invalid email");
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["ok"], false);
        assert_eq!(json["error"], "invalid email");
        assert!(json.get("position").is_none());
    }
}
