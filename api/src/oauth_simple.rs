// Simplified OAuth implementation using cookies instead of sessions
// This avoids tower-sessions complexity with poem

use crate::database::Database;
use anyhow::{anyhow, Result};
use ed25519_dalek::SigningKey;
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use poem::{
    handler,
    http::StatusCode,
    web::cookie::{Cookie, CookieJar, SameSite},
    web::Data,
    web::Query,
    Response, Result as PoemResult,
};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// In-memory OAuth state storage (CSRF token + PKCE verifier)
lazy_static::lazy_static! {
    static ref OAUTH_STATES: Arc<RwLock<HashMap<String, OAuthState>>> = Arc::new(RwLock::new(HashMap::new()));
}

#[derive(Debug, Clone)]
struct OAuthState {
    pkce_verifier_secret: String,
    created_at: std::time::Instant,
}

/// Query parameters for OAuth callback
#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    code: String,
    state: String,
}

/// Request body for OAuth account registration
#[derive(Debug, Deserialize, Serialize)]
pub struct OAuthRegisterRequest {
    pub username: String,
}

/// Response for session keypair endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionKeypairResponse {
    pub success: bool,
    pub data: Option<SessionKeypairData>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionKeypairData {
    pub private_key: String, // hex-encoded
    pub public_key: String,  // hex-encoded
    pub account_id: Option<String>,
    pub username: Option<String>,
}

/// Google UserInfo response
#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    id: String,
    email: Option<String>,
}

/// Initialize Google OAuth client from environment variables
fn create_google_oauth_client() -> Result<BasicClient> {
    let client_id = std::env::var("GOOGLE_OAUTH_CLIENT_ID")
        .map_err(|_| anyhow!("GOOGLE_OAUTH_CLIENT_ID environment variable not set"))?;
    let client_secret = std::env::var("GOOGLE_OAUTH_CLIENT_SECRET")
        .map_err(|_| anyhow!("GOOGLE_OAUTH_CLIENT_SECRET environment variable not set"))?;
    let redirect_url = std::env::var("GOOGLE_OAUTH_REDIRECT_URL")
        .unwrap_or_else(|_| "http://localhost:59011/api/v1/oauth/google/callback".to_string());

    let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())?;
    let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())?;

    Ok(BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(RedirectUrl::new(redirect_url)?))
}

/// Generate a random Ed25519 keypair
fn generate_ed25519_keypair() -> Result<(Vec<u8>, Vec<u8>)> {
    let signing_key = SigningKey::generate(&mut OsRng);
    let private_key = signing_key.to_bytes().to_vec();
    let verifying_key = signing_key.verifying_key();
    let public_key = verifying_key.to_bytes().to_vec();
    Ok((private_key, public_key))
}

/// Determine if cookies should use Secure flag based on environment
/// Returns true if FRONTEND_URL starts with https:// (production)
fn should_use_secure_cookies() -> bool {
    std::env::var("FRONTEND_URL")
        .map(|url| url.starts_with("https://"))
        .unwrap_or(false)
}

/// GET /api/v1/oauth/google/authorize
#[handler]
pub async fn google_authorize() -> PoemResult<Response> {
    let client = create_google_oauth_client().map_err(|e| {
        poem::Error::from_string(
            format!("OAuth client error: {}", e),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    let state_key = csrf_token.secret().clone();
    OAUTH_STATES.write().await.insert(
        state_key,
        OAuthState {
            pkce_verifier_secret: pkce_verifier.secret().clone(),
            created_at: std::time::Instant::now(),
        },
    );

    Ok(Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", auth_url.to_string())
        .finish())
}

/// GET /api/v1/oauth/google/callback
#[handler]
pub async fn google_callback(
    Query(params): Query<OAuthCallbackQuery>,
    cookie_jar: &CookieJar,
    db: Data<&Arc<Database>>,
) -> PoemResult<Response> {
    // Verify CSRF token and retrieve PKCE verifier
    let mut states = OAUTH_STATES.write().await;
    let oauth_state = states.remove(&params.state).ok_or_else(|| {
        poem::Error::from_string("Invalid or expired OAuth state", StatusCode::BAD_REQUEST)
    })?;

    // Clean up old states (older than 10 minutes)
    states.retain(|_, state| state.created_at.elapsed().as_secs() < 600);
    drop(states);

    // Exchange code for token
    let client = create_google_oauth_client().map_err(|e| {
        poem::Error::from_string(
            format!("OAuth client error: {}", e),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;

    // Reconstruct PKCE verifier from secret
    let pkce_verifier = oauth2::PkceCodeVerifier::new(oauth_state.pkce_verifier_secret);

    let token_response = client
        .exchange_code(AuthorizationCode::new(params.code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(oauth2::reqwest::async_http_client)
        .await
        .map_err(|e| {
            poem::Error::from_string(
                format!("Token exchange failed: {}", e),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

    // Fetch user info
    let access_token = token_response.access_token().secret();
    let user_info: GoogleUserInfo = reqwest::Client::new()
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| {
            poem::Error::from_string(
                format!("Failed to fetch user info: {}", e),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?
        .json()
        .await
        .map_err(|e| {
            poem::Error::from_string(
                format!("Failed to parse user info: {}", e),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

    // Generate Ed25519 keypair
    let (private_key, public_key) = generate_ed25519_keypair().map_err(|e| {
        poem::Error::from_string(
            format!("Failed to generate keypair: {}", e),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;

    // Check if OAuth account exists
    let existing_oauth_account = db
        .get_oauth_account_by_provider_and_external_id("google_oauth", &user_info.id)
        .await
        .map_err(|e| {
            poem::Error::from_string(
                format!("Database error: {}", e),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

    let (account_id, username) = if let Some(oauth_acc) = existing_oauth_account {
        // OAuth account already linked - fetch account
        let account = db
            .get_account_by_id(&oauth_acc.account_id)
            .await
            .map_err(|e| {
                poem::Error::from_string(
                    format!("Database error: {}", e),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?
            .ok_or_else(|| poem::Error::from_string("Account not found", StatusCode::NOT_FOUND))?;
        (
            Some(hex::encode(&oauth_acc.account_id)),
            Some(account.username),
        )
    } else if let Some(email) = &user_info.email {
        // Check if an account exists with this email - if so, link OAuth to it
        if let Some(existing_account) = db.get_account_by_email(email).await.map_err(|e| {
            poem::Error::from_string(
                format!("Database error: {}", e),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })? {
            // Link this OAuth account to the existing account
            db.create_oauth_account(
                &existing_account.id,
                "google_oauth",
                &user_info.id,
                Some(email),
            )
            .await
            .map_err(|e| {
                poem::Error::from_string(
                    format!("Failed to link OAuth account: {}", e),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;

            // Mark email as verified since OAuth provider has verified it
            db.set_email_verified(&existing_account.id, true)
                .await
                .map_err(|e| {
                    poem::Error::from_string(
                        format!("Failed to set email verified: {}", e),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )
                })?;

            tracing::info!(
                "Linked Google OAuth account {} to existing account {} by email",
                user_info.id,
                existing_account.username
            );

            (
                Some(hex::encode(&existing_account.id)),
                Some(existing_account.username),
            )
        } else {
            // No existing account - user needs to complete registration
            (None, None)
        }
    } else {
        // No email from OAuth - user needs to complete registration
        (None, None)
    };

    if let (Some(ref acc_id), Some(ref uname)) = (&account_id, &username) {
        // User has account - add session key to database and set cookie
        let account_id_bytes = hex::decode(acc_id).map_err(|e| {
            poem::Error::from_string(
                format!("Failed to decode account ID: {}", e),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

        // Add new session key to account (for signing API requests)
        db.add_account_key(&account_id_bytes, &public_key)
            .await
            .map_err(|e| {
                poem::Error::from_string(
                    format!("Failed to add session key: {}", e),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;

        tracing::info!(
            "Added OAuth session key for account {} ({})",
            uname,
            hex::encode(&public_key)
        );

        let cookie_data = SessionKeypairData {
            private_key: hex::encode(&private_key),
            public_key: hex::encode(&public_key),
            account_id: Some(acc_id.clone()),
            username: Some(uname.clone()),
        };

        let cookie_value = serde_json::to_string(&cookie_data).map_err(|e| {
            poem::Error::from_string(
                format!("Failed to serialize cookie: {}", e),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

        let mut cookie = Cookie::new_with_str("oauth_keypair", cookie_value);
        cookie.set_path("/");
        cookie.set_http_only(true);
        cookie.set_secure(should_use_secure_cookies());
        cookie.set_same_site(Some(SameSite::Lax)); // CSRF protection
        cookie.set_max_age(std::time::Duration::from_secs(7 * 24 * 60 * 60)); // 7 days
        cookie_jar.add(cookie);
    } else {
        // New user - only set temporary oauth_info cookie with keypair data
        let oauth_info = serde_json::json!({
            "provider": "google_oauth",
            "external_id": user_info.id,
            "email": user_info.email,
            "private_key": hex::encode(&private_key),
            "public_key": hex::encode(&public_key),
        })
        .to_string();

        let mut oauth_info_cookie = Cookie::new_with_str("oauth_info", oauth_info);
        oauth_info_cookie.set_path("/");
        oauth_info_cookie.set_http_only(true);
        oauth_info_cookie.set_secure(should_use_secure_cookies());
        oauth_info_cookie.set_same_site(Some(SameSite::Lax)); // CSRF protection
        oauth_info_cookie.set_max_age(std::time::Duration::from_secs(15 * 60)); // 15 minutes
        cookie_jar.add(oauth_info_cookie);
    }

    let frontend_url =
        std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:59010".to_string());

    let redirect_path = if username.is_some() {
        "/dashboard/marketplace"
    } else {
        "/login?oauth=google&step=username"
    };

    Ok(Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", format!("{}{}", frontend_url, redirect_path))
        .finish())
}

/// GET /api/v1/oauth/session/keypair
#[handler]
pub async fn get_session_keypair(
    cookie_jar: &CookieJar,
) -> PoemResult<poem::web::Json<SessionKeypairResponse>> {
    if let Some(cookie) = cookie_jar.get("oauth_keypair") {
        let cookie_value: &str = cookie.value_str();
        let cookie_data: SessionKeypairData = serde_json::from_str(cookie_value).map_err(|e| {
            poem::Error::from_string(
                format!("Failed to parse cookie: {}", e),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

        Ok(poem::web::Json(SessionKeypairResponse {
            success: true,
            data: Some(cookie_data),
            error: None,
        }))
    } else {
        Ok(poem::web::Json(SessionKeypairResponse {
            success: false,
            data: None,
            error: Some("No OAuth session found".to_string()),
        }))
    }
}

/// Response for OAuth info endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthInfoResponse {
    pub success: bool,
    pub data: Option<OAuthInfoData>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthInfoData {
    pub email: Option<String>,
    pub provider: String,
}

/// GET /api/v1/oauth/info
/// Get OAuth info (email, provider) for username prefill during registration
#[handler]
pub async fn get_oauth_info(
    cookie_jar: &CookieJar,
) -> PoemResult<poem::web::Json<OAuthInfoResponse>> {
    if let Some(cookie) = cookie_jar.get("oauth_info") {
        let cookie_value: &str = cookie.value_str();
        let oauth_info: serde_json::Value = serde_json::from_str(cookie_value).map_err(|e| {
            poem::Error::from_string(
                format!("Failed to parse OAuth info: {}", e),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

        let email = oauth_info["email"].as_str().map(|s| s.to_string());
        let provider = oauth_info["provider"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        Ok(poem::web::Json(OAuthInfoResponse {
            success: true,
            data: Some(OAuthInfoData { email, provider }),
            error: None,
        }))
    } else {
        Ok(poem::web::Json(OAuthInfoResponse {
            success: false,
            data: None,
            error: Some("No OAuth info found".to_string()),
        }))
    }
}

/// POST /api/v1/oauth/logout
/// Clear OAuth session cookies
#[handler]
pub async fn oauth_logout(
    cookie_jar: &CookieJar,
) -> PoemResult<poem::web::Json<serde_json::Value>> {
    // Clear oauth_keypair cookie
    let mut clear_keypair = Cookie::new_with_str("oauth_keypair", "");
    clear_keypair.set_path("/");
    clear_keypair.set_http_only(true);
    clear_keypair.set_secure(should_use_secure_cookies());
    clear_keypair.set_same_site(Some(SameSite::Lax));
    clear_keypair.set_max_age(std::time::Duration::from_secs(0));
    cookie_jar.add(clear_keypair);

    // Clear oauth_info cookie
    let mut clear_info = Cookie::new_with_str("oauth_info", "");
    clear_info.set_path("/");
    clear_info.set_http_only(true);
    clear_info.set_secure(should_use_secure_cookies());
    clear_info.set_same_site(Some(SameSite::Lax));
    clear_info.set_max_age(std::time::Duration::from_secs(0));
    cookie_jar.add(clear_info);

    Ok(poem::web::Json(serde_json::json!({
        "success": true
    })))
}

/// Response for OAuth registration endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthRegisterResponse {
    pub success: bool,
    pub data: Option<OAuthRegisterData>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthRegisterData {
    pub account_id: String,
    pub username: String,
    pub email: Option<String>,
}

/// POST /api/v1/oauth/register
/// Complete OAuth account registration by setting username
#[handler]
pub async fn oauth_register(
    poem::web::Json(req): poem::web::Json<OAuthRegisterRequest>,
    cookie_jar: &CookieJar,
    db: Data<&Arc<Database>>,
) -> PoemResult<poem::web::Json<OAuthRegisterResponse>> {
    // Get OAuth info from cookie
    let oauth_info_cookie = cookie_jar.get("oauth_info").ok_or_else(|| {
        poem::Error::from_string(
            "OAuth session expired or not found".to_string(),
            StatusCode::UNAUTHORIZED,
        )
    })?;

    let oauth_info: serde_json::Value = serde_json::from_str(oauth_info_cookie.value_str())
        .map_err(|e| {
            poem::Error::from_string(
                format!("Failed to parse OAuth info: {}", e),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

    // Extract OAuth provider info and keypair from oauth_info cookie
    let provider = oauth_info["provider"].as_str().ok_or_else(|| {
        poem::Error::from_string(
            "Missing provider in OAuth info".to_string(),
            StatusCode::BAD_REQUEST,
        )
    })?;

    let external_id = oauth_info["external_id"].as_str().ok_or_else(|| {
        poem::Error::from_string(
            "Missing external_id in OAuth info".to_string(),
            StatusCode::BAD_REQUEST,
        )
    })?;

    let email = oauth_info["email"].as_str();

    let private_key_hex = oauth_info["private_key"].as_str().ok_or_else(|| {
        poem::Error::from_string(
            "Missing private_key in OAuth info".to_string(),
            StatusCode::BAD_REQUEST,
        )
    })?;

    let public_key_hex = oauth_info["public_key"].as_str().ok_or_else(|| {
        poem::Error::from_string(
            "Missing public_key in OAuth info".to_string(),
            StatusCode::BAD_REQUEST,
        )
    })?;

    // Decode public key
    let public_key = hex::decode(public_key_hex).map_err(|e| {
        poem::Error::from_string(
            format!("Failed to decode public key: {}", e),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;

    // Validate username
    let username = crate::validation::validate_account_username(&req.username).map_err(|e| {
        poem::Error::from_string(format!("Invalid username: {}", e), StatusCode::BAD_REQUEST)
    })?;

    // Create account with OAuth link
    let (account, _oauth_account) = db
        .create_oauth_linked_account(
            &username,
            &public_key,
            email.unwrap_or_default(), // Use empty string if no email
            provider,
            external_id,
        )
        .await
        .map_err(|e| {
            poem::Error::from_string(
                format!("Failed to create account: {}", e),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

    // Set oauth_keypair cookie with account info (first time for new users)
    let keypair_data = SessionKeypairData {
        private_key: private_key_hex.to_string(),
        public_key: public_key_hex.to_string(),
        account_id: Some(hex::encode(&account.id)),
        username: Some(account.username.clone()),
    };

    let cookie_value = serde_json::to_string(&keypair_data).map_err(|e| {
        poem::Error::from_string(
            format!("Failed to serialize cookie: {}", e),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;

    let mut cookie = Cookie::new_with_str("oauth_keypair", cookie_value);
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_secure(should_use_secure_cookies());
    cookie.set_same_site(Some(SameSite::Lax)); // CSRF protection
    cookie.set_max_age(std::time::Duration::from_secs(7 * 24 * 60 * 60)); // 7 days
    cookie_jar.add(cookie);

    // Remove oauth_info cookie as it's no longer needed
    let mut clear_oauth_info = Cookie::new_with_str("oauth_info", "");
    clear_oauth_info.set_path("/");
    clear_oauth_info.set_same_site(Some(SameSite::Lax));
    clear_oauth_info.set_max_age(std::time::Duration::from_secs(0));
    cookie_jar.add(clear_oauth_info);

    tracing::info!(
        "Created new account {} via OAuth provider {}",
        account.username,
        provider
    );

    Ok(poem::web::Json(OAuthRegisterResponse {
        success: true,
        data: Some(OAuthRegisterData {
            account_id: hex::encode(&account.id),
            username: account.username,
            email: account.email,
        }),
        error: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcc_common::DccIdentity;

    #[test]
    fn test_generate_ed25519_keypair() {
        let (private_key, public_key) = generate_ed25519_keypair().unwrap();

        // Verify correct lengths
        assert_eq!(private_key.len(), 32, "Private key should be 32 bytes");
        assert_eq!(public_key.len(), 32, "Public key should be 32 bytes");

        // Verify keypair is valid by signing and verifying
        let identity = DccIdentity::new_signing_from_bytes(&private_key).unwrap();
        assert_eq!(
            identity.verifying_key().to_bytes(),
            public_key.as_slice(),
            "Public key should match verifying key from private key"
        );
    }

    #[test]
    fn test_generate_ed25519_keypair_uniqueness() {
        // Generate two keypairs and verify they're different
        let (priv1, pub1) = generate_ed25519_keypair().unwrap();
        let (priv2, pub2) = generate_ed25519_keypair().unwrap();

        assert_ne!(priv1, priv2, "Private keys should be unique");
        assert_ne!(pub1, pub2, "Public keys should be unique");
    }

    #[test]
    fn test_create_google_oauth_client_missing_client_id() {
        // Clear environment variables
        std::env::remove_var("GOOGLE_OAUTH_CLIENT_ID");
        std::env::remove_var("GOOGLE_OAUTH_CLIENT_SECRET");

        let result = create_google_oauth_client();
        assert!(
            result.is_err(),
            "Should fail without GOOGLE_OAUTH_CLIENT_ID"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("GOOGLE_OAUTH_CLIENT_ID"),
            "Error should mention missing CLIENT_ID"
        );
    }

    #[test]
    fn test_create_google_oauth_client_missing_client_secret() {
        std::env::set_var("GOOGLE_OAUTH_CLIENT_ID", "test_id");
        std::env::remove_var("GOOGLE_OAUTH_CLIENT_SECRET");

        let result = create_google_oauth_client();
        assert!(
            result.is_err(),
            "Should fail without GOOGLE_OAUTH_CLIENT_SECRET"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("GOOGLE_OAUTH_CLIENT_SECRET"),
            "Error should mention missing CLIENT_SECRET"
        );

        // Cleanup
        std::env::remove_var("GOOGLE_OAUTH_CLIENT_ID");
    }

    #[test]
    fn test_create_google_oauth_client_success() {
        std::env::set_var("GOOGLE_OAUTH_CLIENT_ID", "test_client_id");
        std::env::set_var("GOOGLE_OAUTH_CLIENT_SECRET", "test_client_secret");
        std::env::set_var(
            "GOOGLE_OAUTH_REDIRECT_URL",
            "http://localhost:59011/api/v1/oauth/google/callback",
        );

        let result = create_google_oauth_client();
        assert!(result.is_ok(), "Should succeed with all required env vars");

        // Cleanup
        std::env::remove_var("GOOGLE_OAUTH_CLIENT_ID");
        std::env::remove_var("GOOGLE_OAUTH_CLIENT_SECRET");
        std::env::remove_var("GOOGLE_OAUTH_REDIRECT_URL");
    }

    #[test]
    fn test_create_google_oauth_client_default_redirect_url() {
        std::env::set_var("GOOGLE_OAUTH_CLIENT_ID", "test_id");
        std::env::set_var("GOOGLE_OAUTH_CLIENT_SECRET", "test_secret");
        std::env::remove_var("GOOGLE_OAUTH_REDIRECT_URL");

        let result = create_google_oauth_client();
        assert!(
            result.is_ok(),
            "Should use default redirect URL if not provided"
        );

        // Cleanup
        std::env::remove_var("GOOGLE_OAUTH_CLIENT_ID");
        std::env::remove_var("GOOGLE_OAUTH_CLIENT_SECRET");
    }

    #[test]
    fn test_should_use_secure_cookies_with_https() {
        std::env::set_var("FRONTEND_URL", "https://decent-cloud.org");
        assert!(
            should_use_secure_cookies(),
            "Should return true for HTTPS frontend URL"
        );
        std::env::remove_var("FRONTEND_URL");
    }

    #[test]
    fn test_should_use_secure_cookies_with_http() {
        std::env::set_var("FRONTEND_URL", "http://localhost:59010");
        assert!(
            !should_use_secure_cookies(),
            "Should return false for HTTP frontend URL"
        );
        std::env::remove_var("FRONTEND_URL");
    }

    #[test]
    fn test_should_use_secure_cookies_not_set() {
        std::env::remove_var("FRONTEND_URL");
        assert!(
            !should_use_secure_cookies(),
            "Should return false when FRONTEND_URL not set"
        );
    }
}
