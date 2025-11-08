mod database;
mod ledger_client;
mod sync_service;

use candid::Principal;
use database::Database;
use ledger_client::LedgerClient;
use poem::{
    handler, listener::TcpListener, middleware::Cors, post, web::Json, EndpointExt, Response,
    Route, Server,
};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
use sync_service::SyncService;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Serialize, Deserialize)]
struct HealthResponse {
    success: bool,
    message: String,
    environment: String,
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

    // For now, return a meaningful error response instead of panicking
    let error_response = serde_json::json!({
        "success": false,
        "error": format!("Canister method '{}' not yet implemented", method.0),
        "message": "ICP canister integration is pending implementation"
    });

    poem::Response::builder()
        .status(poem::http::StatusCode::NOT_IMPLEMENTED)
        .header(poem::http::header::CONTENT_TYPE, "application/json")
        .body(error_response.to_string())
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Load .env file if it exists
    dotenv::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);

    // Database setup
    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:./ledger.db?mode=rwc".to_string());
    let database = match Database::new(&database_url).await {
        Ok(db) => {
            tracing::info!("Database initialized at {}", database_url);
            Arc::new(db)
        }
        Err(e) => {
            tracing::error!("Failed to initialize database at {}: {}", database_url, e);
            return Err(std::io::Error::other(format!(
                "Database initialization failed: {}",
                e
            )));
        }
    };

    // Ledger client setup
    let network_url = env::var("NETWORK_URL").unwrap_or_else(|_| "https://ic0.app".to_string());
    let canister_id = env::var("CANISTER_ID")
        .expect("CANISTER_ID environment variable required")
        .parse::<Principal>()
        .expect("Invalid CANISTER_ID");
    let ledger_client = Arc::new(
        LedgerClient::new(&network_url, canister_id)
            .await
            .expect("Failed to initialize ledger client"),
    );
    tracing::info!("Ledger client initialized for canister {}", canister_id);

    // Start sync service in background
    let sync_interval_secs = env::var("SYNC_INTERVAL_SECS")
        .unwrap_or_else(|_| "30".to_string())
        .parse::<u64>()
        .unwrap_or(30);
    let sync_service =
        SyncService::new(ledger_client.clone(), database.clone(), sync_interval_secs);
    tokio::spawn(async move {
        tracing::info!(
            "Starting sync service with {}s interval",
            sync_interval_secs
        );
        sync_service.run().await;
    });

    tracing::info!("Starting Decent Cloud API server on {}", addr);

    let app = Route::new()
        .at("/api/v1/health", poem::get(health))
        .at("/api/v1/canister/:method", post(canister_proxy))
        .with(Cors::new());

    Server::new(TcpListener::bind(&addr)).run(app).await
}
