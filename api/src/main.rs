mod api_handlers;
mod auth;
mod database;
mod ledger_client;
mod ledger_path;
mod metadata_cache;
mod network_metrics;
mod sync_service;

use candid::Principal;
use clap::{Parser, Subcommand};
use database::Database;
use ledger_client::LedgerClient;
use metadata_cache::MetadataCache;
use poem::{
    handler, listener::TcpListener, middleware::Cors, post, web::Json, EndpointExt, Response,
    Route, Server,
};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
use sync_service::SyncService;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "api-server")]
#[command(about = "Decent Cloud API Server")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the API server
    Serve,
    /// Run the sync service
    Sync,
}

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

struct AppContext {
    database: Arc<Database>,
    ledger_client: Arc<LedgerClient>,
    sync_interval_secs: u64,
    metadata_cache: Arc<MetadataCache>,
}

async fn setup_app_context() -> Result<AppContext, std::io::Error> {
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
    let network_url = env::var("NETWORK_URL").unwrap_or_else(|_| "https://icp-api.io".to_string());
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

    // Sync interval setup
    let sync_interval_secs = env::var("SYNC_INTERVAL_SECS")
        .unwrap_or_else(|_| "30".to_string())
        .parse::<u64>()
        .unwrap_or(30);

    // Metadata cache setup
    let metadata_refresh_interval = env::var("METADATA_REFRESH_INTERVAL_SECS")
        .unwrap_or_else(|_| "60".to_string())
        .parse::<u64>()
        .unwrap_or(60);
    let metadata_cache = Arc::new(MetadataCache::new(
        ledger_client.clone(),
        metadata_refresh_interval,
    ));

    Ok(AppContext {
        database,
        ledger_client,
        sync_interval_secs,
        metadata_cache,
    })
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
    let cli = Cli::parse();

    // Load .env file if it exists
    dotenv::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    match cli.command {
        Commands::Serve => serve_command().await,
        Commands::Sync => sync_command().await,
    }
}

async fn serve_command() -> Result<(), std::io::Error> {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let ctx = setup_app_context().await?;

    tracing::info!("Starting Decent Cloud API server on {}", addr);

    let app = Route::new()
        // Health check
        .at("/api/v1/health", poem::get(health))
        // Legacy canister proxy
        .at("/api/v1/canister/:method", post(canister_proxy))
        // Provider endpoints
        .at("/api/v1/providers", poem::get(api_handlers::list_providers))
        .at(
            "/api/v1/providers/active/:days",
            poem::get(api_handlers::get_active_providers),
        )
        .at(
            "/api/v1/providers/:pubkey",
            poem::get(api_handlers::get_provider_profile),
        )
        .at(
            "/api/v1/providers/:pubkey/contacts",
            poem::get(api_handlers::get_provider_contacts),
        )
        .at(
            "/api/v1/providers/:pubkey/offerings",
            poem::get(api_handlers::get_provider_offerings),
        )
        .at(
            "/api/v1/providers/:pubkey/contracts",
            poem::get(api_handlers::get_provider_contracts),
        )
        .at(
            "/api/v1/providers/:pubkey/stats",
            poem::get(api_handlers::get_provider_stats),
        )
        // Offering endpoints
        .at(
            "/api/v1/offerings",
            poem::get(api_handlers::search_offerings),
        )
        .at(
            "/api/v1/offerings/:id",
            poem::get(api_handlers::get_offering),
        )
        // Contract endpoints
        .at("/api/v1/contracts", poem::get(api_handlers::list_contracts))
        .at(
            "/api/v1/contracts/:id",
            poem::get(api_handlers::get_contract),
        )
        .at(
            "/api/v1/users/:pubkey/contracts",
            poem::get(api_handlers::get_user_contracts),
        )
        // User profile endpoints
        .at(
            "/api/v1/users/:pubkey/profile",
            poem::get(api_handlers::get_user_profile),
        )
        .at(
            "/api/v1/users/:pubkey/contacts",
            poem::get(api_handlers::get_user_contacts),
        )
        .at(
            "/api/v1/users/:pubkey/socials",
            poem::get(api_handlers::get_user_socials),
        )
        .at(
            "/api/v1/users/:pubkey/keys",
            poem::get(api_handlers::get_user_public_keys),
        )
        // User profile update endpoints (authenticated)
        .at(
            "/api/v1/users/:pubkey/profile",
            poem::put(api_handlers::update_user_profile),
        )
        .at(
            "/api/v1/users/:pubkey/contacts",
            poem::post(api_handlers::upsert_user_contact),
        )
        .at(
            "/api/v1/users/:pubkey/contacts/:contact_type",
            poem::delete(api_handlers::delete_user_contact),
        )
        .at(
            "/api/v1/users/:pubkey/socials",
            poem::post(api_handlers::upsert_user_social),
        )
        .at(
            "/api/v1/users/:pubkey/socials/:platform",
            poem::delete(api_handlers::delete_user_social),
        )
        .at(
            "/api/v1/users/:pubkey/keys",
            poem::post(api_handlers::add_user_public_key),
        )
        .at(
            "/api/v1/users/:pubkey/keys/:key_fingerprint",
            poem::delete(api_handlers::delete_user_public_key),
        )
        // Token endpoints
        .at(
            "/api/v1/transfers",
            poem::get(api_handlers::get_recent_transfers),
        )
        .at(
            "/api/v1/accounts/:account/transfers",
            poem::get(api_handlers::get_account_transfers),
        )
        .at(
            "/api/v1/accounts/:account/balance",
            poem::get(api_handlers::get_account_balance),
        )
        // Stats endpoints
        .at("/api/v1/stats", poem::get(api_handlers::get_platform_stats))
        .at(
            "/api/v1/reputation/:pubkey",
            poem::get(api_handlers::get_reputation),
        )
        .data(ctx.database)
        .data(ctx.metadata_cache.clone())
        .with(Cors::new());

    // Start metadata cache service in background
    let cache_for_task = ctx.metadata_cache.clone();
    let metadata_cache_task = tokio::spawn(async move {
        cache_for_task.run().await;
    });

    let server_result = Server::new(TcpListener::bind(&addr)).run(app).await;

    metadata_cache_task.abort();
    server_result
}

async fn sync_command() -> Result<(), std::io::Error> {
    let ctx = setup_app_context().await?;

    // Run sync service
    let sync_service = SyncService::new(
        ctx.ledger_client.clone(),
        ctx.database.clone(),
        ctx.sync_interval_secs,
    );

    tracing::info!(
        "Running sync service with {}s interval",
        ctx.sync_interval_secs
    );
    sync_service.run().await;

    Ok(())
}
