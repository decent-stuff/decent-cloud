use poem::{
    handler, listener::TcpListener, middleware::Cors, post, web::Json, EndpointExt, Response,
    Route, Server,
};
use serde::{Deserialize, Serialize};
use std::env;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Serialize, Deserialize)]
struct HealthResponse {
    success: bool,
    message: String,
    environment: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CanisterRequest {
    args: Vec<serde_json::Value>,
}

#[handler]
async fn health() -> Json<HealthResponse> {
    let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
    Json(HealthResponse {
        success: true,
        message: "Decent Cloud API is running".to_string(),
        environment,
    })
}

/// Proxy ICP canister methods
///
/// Expected methods from cf-service.ts:
/// - Provider: provider_register_anonymous, provider_update_profile_anonymous,
///   provider_update_offering_anonymous, provider_list_checked_in,
///   provider_get_profile_by_pubkey_bytes, provider_get_profile_by_principal
/// - Offering: offering_search
/// - Contract: contract_sign_request_anonymous, contracts_list_pending,
///   contract_sign_reply_anonymous
/// - User: user_register_anonymous
/// - Check-in: get_check_in_nonce, provider_check_in_anonymous
/// - Common: get_identity_reputation, get_registration_fee
#[handler]
async fn canister_proxy(
    method: poem::web::Path<String>,
    Json(req): Json<CanisterRequest>,
) -> Response {
    tracing::info!("Proxying canister method: {}", method.0);
    tracing::debug!("Request args: {:?}", req.args);

    // TODO: Implement ICP agent and actual canister calls
    // This requires:
    // 1. Add ic-agent dependency
    // 2. Initialize agent with canister ID from env
    // 3. Parse args based on method signature
    // 4. Call canister method
    // 5. Return result in CFResponse<T> format

    unimplemented!(
        "Canister proxy for method '{}' not yet implemented. \
        Need to integrate ic-agent and implement canister method calls.",
        method.0
    );
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Load .env file if it exists
    dotenv::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);

    tracing::info!("Starting Decent Cloud API server on {}", addr);

    let app = Route::new()
        .at("/api/v1/health", poem::get(health))
        .at("/api/v1/canister/:method", post(canister_proxy))
        .with(Cors::new());

    Server::new(TcpListener::bind(&addr))
        .run(app)
        .await
}
