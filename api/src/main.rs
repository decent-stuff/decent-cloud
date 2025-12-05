mod auth;
mod chatwoot;
mod cleanup_service;
mod database;
mod email_processor;
mod email_service;
mod icpay_client;
mod ledger_client;
mod ledger_path;
mod metadata_cache;
mod network_metrics;
mod oauth_simple;
mod openapi;
mod request_logging;
mod search;
mod stripe_client;
mod support_bot;
mod sync_service;
mod validation;

use candid::Principal;
use clap::{Parser, Subcommand};
use cleanup_service::CleanupService;
use database::Database;
use email_processor::EmailProcessor;
use email_service::EmailService;
use ledger_client::LedgerClient;
use metadata_cache::MetadataCache;
use openapi::create_combined_api;
use poem::web::Redirect;
use poem::{
    get, handler,
    listener::TcpListener,
    middleware::{CookieJarManager, Cors},
    post,
    web::Json,
    EndpointExt, Response, Route, Server,
};
use poem_openapi::OpenApiService;
use serde::Deserialize;
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

#[derive(Debug, Deserialize)]
struct CanisterRequest {
    args: Vec<serde_json::Value>,
}

struct AppContext {
    database: Arc<Database>,
    ledger_client: Arc<LedgerClient>,
    sync_interval_secs: u64,
    metadata_cache: Arc<MetadataCache>,
    email_service: Option<Arc<EmailService>>,
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

    // Email service setup (optional)
    let email_service = env::var("MAILCHANNELS_API_KEY").ok().map(|api_key| {
        let dkim_domain = env::var("DKIM_DOMAIN").ok();
        let dkim_selector = env::var("DKIM_SELECTOR").ok();
        let dkim_private_key = env::var("DKIM_PRIVATE_KEY").ok();

        if dkim_selector.is_some() {
            tracing::info!("Email service initialized with DKIM signing");
        } else {
            tracing::info!("Email service initialized without DKIM signing");
        }

        Arc::new(EmailService::new(
            api_key,
            dkim_domain,
            dkim_selector,
            dkim_private_key,
        ))
    });

    Ok(AppContext {
        database,
        ledger_client,
        sync_interval_secs,
        metadata_cache,
        email_service,
    })
}

/// Redirect from root to Swagger UI
#[handler]
fn root_redirect() -> Redirect {
    Redirect::temporary("/api/v1/swagger")
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
    let port = env::var("API_SERVER_PORT").unwrap_or_else(|_| "59011".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let ctx = setup_app_context().await?;

    tracing::info!("Starting Decent Cloud API server on {}", addr);

    // Set up OpenAPI service with Swagger UI
    let api_service =
        OpenApiService::new(create_combined_api(), "Decent Cloud API", "1.0.0").server("/api/v1");
    let swagger_ui = api_service.swagger_ui();
    let openapi_spec = api_service.spec_endpoint();

    // Configure CORS based on environment
    let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string());
    let cors = if environment == "prod" {
        Cors::new()
            .allow_origin("https://decent-cloud.org")
            .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allow_headers(vec![
                "content-type",
                "authorization",
                "x-api-key",
                "x-account-id",
                "x-nonce",
                "x-public-key",
                "x-signature",
                "x-timestamp",
            ])
            .allow_credentials(true)
    } else {
        // Development: allow all localhost origins for testing
        tracing::info!("CORS: Development mode - allowing all localhost origins");
        Cors::new()
            .allow_origin("https://dev.decent-cloud.org")
            .allow_origin("http://localhost:59000")
            .allow_origin("http://localhost:59010")
            .allow_origin("http://localhost:3000")
            .allow_origin("http://localhost:5173")
            .allow_origin("http://localhost:5174")
            .allow_origin("http://127.0.0.1:59000")
            .allow_origin("http://127.0.0.1:59010")
            .allow_origin("http://127.0.0.1:3000")
            .allow_origin("http://127.0.0.1:5173")
            .allow_origin("http://127.0.0.1:5174")
            .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allow_headers(vec![
                "content-type",
                "authorization",
                "x-api-key",
                "x-account-id",
                "x-nonce",
                "x-public-key",
                "x-signature",
                "x-timestamp",
            ])
            .allow_credentials(true)
    };

    let app = Route::new()
        // Redirect root to Swagger UI
        .at("/", root_redirect)
        // OpenAPI documentation and Swagger UI
        .nest("/api/v1/swagger", swagger_ui)
        .nest("/api/v1/openapi", openapi_spec)
        .nest("/api/v1", api_service)
        // OAuth endpoints
        .at(
            "/api/v1/oauth/google/authorize",
            get(oauth_simple::google_authorize),
        )
        .at(
            "/api/v1/oauth/google/callback",
            get(oauth_simple::google_callback),
        )
        .at(
            "/api/v1/oauth/session/keypair",
            get(oauth_simple::get_session_keypair),
        )
        .at("/api/v1/oauth/info", get(oauth_simple::get_oauth_info))
        .at("/api/v1/oauth/register", post(oauth_simple::oauth_register))
        .at("/api/v1/oauth/logout", post(oauth_simple::oauth_logout))
        // Webhook endpoints
        .at(
            "/api/v1/webhooks/stripe",
            post(openapi::webhooks::stripe_webhook),
        )
        .at(
            "/api/v1/webhooks/chatwoot",
            post(openapi::webhooks::chatwoot_webhook),
        )
        // Legacy endpoints (canister proxy - ICP integration pending)
        // NOTE: CSV operations are now included in OpenAPI schema above
        .at("/api/v1/canister/:method", post(canister_proxy))
        .data(ctx.database.clone())
        .data(ctx.metadata_cache.clone())
        .data(ctx.email_service.clone())
        .with(CookieJarManager::new())
        .with(request_logging::RequestLogging)
        .with(cors);

    // Start metadata cache service in background
    let cache_for_task = ctx.metadata_cache.clone();
    let metadata_cache_task = tokio::spawn(async move {
        cache_for_task.run().await;
    });

    // Start cleanup service in background (runs every 24 hours, 180-day retention)
    let cleanup_interval_hours = env::var("CLEANUP_INTERVAL_HOURS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(24);
    let cleanup_retention_days = env::var("CLEANUP_RETENTION_DAYS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(180);

    let db_for_cleanup = ctx.database.clone();
    let cleanup_task = tokio::spawn(async move {
        let cleanup_service = CleanupService::new(
            db_for_cleanup,
            cleanup_interval_hours,
            cleanup_retention_days,
        );
        tracing::info!(
            "Starting cleanup service (interval: {}h, retention: {}d)",
            cleanup_interval_hours,
            cleanup_retention_days
        );
        cleanup_service.run().await;
    });

    // Start email processor in background if email service is configured
    let email_processor_task = if let Some(email_svc) = ctx.email_service.clone() {
        let email_interval_secs = env::var("EMAIL_PROCESSOR_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);
        let email_batch_size = env::var("EMAIL_BATCH_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);

        let db_for_email = ctx.database.clone();
        Some(tokio::spawn(async move {
            let email_processor = EmailProcessor::new(
                db_for_email,
                email_svc,
                email_interval_secs,
                email_batch_size,
            );
            tracing::info!(
                "Starting email processor (interval: {}s, batch: {})",
                email_interval_secs,
                email_batch_size
            );
            email_processor.run().await;
        }))
    } else {
        tracing::info!("Email processor not started (no email service configured)");
        None
    };

    let server_result = Server::new(TcpListener::bind(&addr)).run(app).await;

    metadata_cache_task.abort();
    cleanup_task.abort();
    if let Some(task) = email_processor_task {
        task.abort();
    }
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

#[cfg(test)]
mod tests;
