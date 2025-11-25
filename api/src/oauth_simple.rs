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
    web::cookie::{Cookie, CookieJar},
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
    csrf_token: String,
    pkce_verifier_secret: String,
    created_at: std::time::Instant,
}

/// Query parameters for OAuth callback
#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    code: String,
    state: String,
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
        .unwrap_or_else(|_| "http://localhost:59001/api/v1/oauth/google/callback".to_string());

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
        state_key.clone(),
        OAuthState {
            csrf_token: state_key,
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
    } else {
        (None, None)
    };

    // Store keypair and account info in cookie as JSON
    let cookie_data = SessionKeypairData {
        private_key: hex::encode(&private_key),
        public_key: hex::encode(&public_key),
        account_id,
        username: username.clone(),
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
    cookie.set_max_age(std::time::Duration::from_secs(7 * 24 * 60 * 60)); // 7 days
    cookie_jar.add(cookie);

    // Also store OAuth provider info for account linking
    let oauth_info = serde_json::json!({
        "provider": "google_oauth",
        "external_id": user_info.id,
        "email": user_info.email,
    })
    .to_string();

    let mut oauth_info_cookie = Cookie::new_with_str("oauth_info", oauth_info);
    oauth_info_cookie.set_path("/");
    oauth_info_cookie.set_http_only(true);
    oauth_info_cookie.set_max_age(std::time::Duration::from_secs(15 * 60)); // 15 minutes
    cookie_jar.add(oauth_info_cookie);

    let frontend_url =
        std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:59000".to_string());

    let redirect_path = if username.is_some() {
        "/dashboard/marketplace"
    } else {
        "/auth?oauth=google&step=username"
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
