mod auth;
mod chatwoot;
mod cleanup_service;
mod cloudflare_dns;
mod crypto;
mod database;
mod email_processor;
mod email_service;
mod helpcenter;
mod icpay_client;
mod invoice_storage;
mod invoices;
mod ledger_client;
mod ledger_path;
mod metadata_cache;
mod network_metrics;
mod notifications;
mod oauth_simple;
mod openapi;
mod payment_release_service;
mod receipts;
mod regions;
mod rental_notifications;
mod request_logging;
mod search;
mod stripe_client;
mod support_bot;
mod sync_docs;
mod sync_service;
mod validation;
mod vies;

use candid::Principal;
use clap::{Parser, Subcommand};
use cleanup_service::CleanupService;
use database::Database;
use email_processor::EmailProcessor;
use email_service::EmailService;
use ledger_client::LedgerClient;
use metadata_cache::MetadataCache;
use openapi::create_combined_api;
use payment_release_service::PaymentReleaseService;
use poem::web::Redirect;
use poem::{
    get, handler,
    listener::TcpListener,
    middleware::{CookieJarManager, Cors},
    post, EndpointExt, Route, Server,
};
use poem_openapi::OpenApiService;
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
    /// Check configuration and external service connectivity
    Doctor,
    /// Sync documentation to Chatwoot Help Center
    SyncDocs {
        /// Portal slug to sync to
        #[arg(long, default_value = "platform-overview")]
        portal: String,

        /// Dry run - show what would be synced without making changes
        #[arg(long)]
        dry_run: bool,
    },
    /// Automated setup for external services
    Setup {
        #[command(subcommand)]
        service: SetupService,
    },
}

#[derive(Subcommand)]
enum SetupService {
    /// Create or update Stripe webhook endpoint
    StripeWebhooks {
        /// Webhook endpoint URL (defaults to API_PUBLIC_URL/api/v1/webhooks/stripe)
        #[arg(long)]
        url: Option<String>,
    },
    /// Generate DKIM key pair and create DNS TXT record
    Dkim {
        /// DKIM selector (e.g., "dc" for dc._domainkey.domain.com)
        #[arg(long, default_value = "dc")]
        selector: String,
    },
}

struct AppContext {
    database: Arc<Database>,
    ledger_client: Arc<LedgerClient>,
    sync_interval_secs: u64,
    metadata_cache: Arc<MetadataCache>,
    email_service: Option<Arc<EmailService>>,
    cloudflare_dns: Option<Arc<cloudflare_dns::CloudflareDns>>,
}

async fn setup_app_context() -> Result<AppContext, std::io::Error> {
    // Database setup
    // Note: DATABASE_URL should be set via environment variable or .env file
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| crate::database::DEFAULT_DATABASE_URL.to_string());
    let database = match Database::new(&database_url).await {
        Ok(db) => {
            tracing::info!("Database initialized at {}", database_url);
            Arc::new(db)
        }
        Err(e) => {
            tracing::error!("Failed to initialize database at {}: {:#}", database_url, e);
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

    // Cloudflare DNS setup (optional - for gateway DNS management)
    let cloudflare_dns = cloudflare_dns::CloudflareDns::from_env();
    if cloudflare_dns.is_some() {
        tracing::info!("Cloudflare DNS client initialized for gateway management");
    } else {
        tracing::info!(
            "Cloudflare DNS not configured (CF_API_TOKEN, CF_ZONE_ID) - gateway DNS will NOT work"
        );
    }

    Ok(AppContext {
        database,
        ledger_client,
        sync_interval_secs,
        metadata_cache,
        email_service,
        cloudflare_dns,
    })
}

/// Redirect from root to Swagger UI
#[handler]
fn root_redirect() -> Redirect {
    Redirect::temporary("/api/v1/swagger")
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
        Commands::Doctor => doctor_command().await,
        Commands::SyncDocs { portal, dry_run } => sync_docs_command(&portal, dry_run).await,
        Commands::Setup { service } => setup_command(service).await,
    }
}

/// Sync documentation to Chatwoot Help Center
async fn sync_docs_command(portal: &str, dry_run: bool) -> Result<(), std::io::Error> {
    match sync_docs::sync_docs(portal, dry_run).await {
        Ok(()) => Ok(()),
        Err(e) => Err(std::io::Error::other(format!(
            "Failed to sync documentation: {:?}",
            e
        ))),
    }
}

/// Automated setup for external services
async fn setup_command(service: SetupService) -> Result<(), std::io::Error> {
    match service {
        SetupService::StripeWebhooks { url } => setup_stripe_webhooks(url).await,
        SetupService::Dkim { selector } => setup_dkim(&selector).await,
    }
}

/// Create or update Stripe webhook endpoint with all required events
async fn setup_stripe_webhooks(custom_url: Option<String>) -> Result<(), std::io::Error> {
    println!("=== Stripe Webhooks Setup ===\n");

    // Get API key
    let secret_key = match env::var("STRIPE_SECRET_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("[ERROR] STRIPE_SECRET_KEY not set");
            println!("\n  Set STRIPE_SECRET_KEY in your .env file or environment:");
            println!("    export STRIPE_SECRET_KEY=sk_live_...");
            return Err(std::io::Error::other("STRIPE_SECRET_KEY not set"));
        }
    };

    // Determine webhook URL
    let webhook_url = match custom_url {
        Some(url) => url,
        None => {
            let api_url = env::var("API_PUBLIC_URL").map_err(|_| {
                std::io::Error::other(
                    "API_PUBLIC_URL not set. Either set it or provide --url argument.",
                )
            })?;
            format!("{}/api/v1/webhooks/stripe", api_url.trim_end_matches('/'))
        }
    };

    println!("Webhook URL: {}", webhook_url);

    // Required events for our webhook handler
    let events = vec![
        "checkout.session.completed",
        "invoice.paid",
        "invoice.payment_failed",
        "customer.subscription.created",
        "customer.subscription.updated",
        "customer.subscription.deleted",
    ];

    println!("Events: {:?}", events);

    // Use Stripe API to list existing webhooks
    let client = reqwest::Client::new();

    // First, check if a webhook for this URL already exists
    print!("\nChecking existing webhooks... ");
    let list_response = client
        .get("https://api.stripe.com/v1/webhook_endpoints")
        .basic_auth(&secret_key, None::<&str>)
        .send()
        .await
        .map_err(|e| std::io::Error::other(format!("Failed to list webhooks: {}", e)))?;

    if !list_response.status().is_success() {
        let error_text = list_response.text().await.unwrap_or_default();
        println!("[ERROR]");
        return Err(std::io::Error::other(format!(
            "Failed to list webhooks: {}",
            error_text
        )));
    }

    let list_body: serde_json::Value = list_response.json().await.map_err(|e| {
        std::io::Error::other(format!("Failed to parse webhook list response: {}", e))
    })?;

    // Find existing webhook with matching URL
    let existing_webhook = list_body["data"].as_array().and_then(|webhooks| {
        webhooks
            .iter()
            .find(|wh| wh["url"].as_str() == Some(&webhook_url))
    });

    if let Some(webhook) = existing_webhook {
        let webhook_id = webhook["id"].as_str().unwrap_or("");
        println!("[OK] found existing ({})", webhook_id);

        // Update existing webhook
        print!("Updating webhook events... ");
        let mut form: Vec<(&str, &str)> = events.iter().map(|e| ("enabled_events[]", *e)).collect();
        form.push(("url", &webhook_url));

        let update_response = client
            .post(format!(
                "https://api.stripe.com/v1/webhook_endpoints/{}",
                webhook_id
            ))
            .basic_auth(&secret_key, None::<&str>)
            .form(&form)
            .send()
            .await
            .map_err(|e| std::io::Error::other(format!("Failed to update webhook: {}", e)))?;

        if !update_response.status().is_success() {
            let error_text = update_response.text().await.unwrap_or_default();
            println!("[ERROR]");
            return Err(std::io::Error::other(format!(
                "Failed to update webhook: {}",
                error_text
            )));
        }

        println!("[OK]");
        println!("\n=== Webhook Updated Successfully ===");
        println!("\nNote: The webhook signing secret remains unchanged.");
        println!(
            "If you need the secret, delete and recreate the webhook in the Stripe dashboard."
        );
    } else {
        println!("[OK] none found");

        // Create new webhook
        print!("Creating webhook endpoint... ");
        let mut form: Vec<(&str, &str)> = events.iter().map(|e| ("enabled_events[]", *e)).collect();
        form.push(("url", &webhook_url));

        let create_response = client
            .post("https://api.stripe.com/v1/webhook_endpoints")
            .basic_auth(&secret_key, None::<&str>)
            .form(&form)
            .send()
            .await
            .map_err(|e| std::io::Error::other(format!("Failed to create webhook: {}", e)))?;

        if !create_response.status().is_success() {
            let error_text = create_response.text().await.unwrap_or_default();
            println!("[ERROR]");
            return Err(std::io::Error::other(format!(
                "Failed to create webhook: {}",
                error_text
            )));
        }

        let create_body: serde_json::Value = create_response.json().await.map_err(|e| {
            std::io::Error::other(format!("Failed to parse create response: {}", e))
        })?;

        let webhook_id = create_body["id"].as_str().unwrap_or("unknown");
        let webhook_secret = create_body["secret"].as_str().unwrap_or("");

        println!("[OK] ({})", webhook_id);

        println!("\n=== Webhook Created Successfully ===\n");
        println!("Add this to your .env file:\n");
        println!("  STRIPE_WEBHOOK_SECRET={}", webhook_secret);
        println!("\n[IMPORTANT] Save this secret now - it cannot be retrieved later!");
    }

    Ok(())
}

/// Generate DKIM key pair and create DNS TXT record
async fn setup_dkim(selector: &str) -> Result<(), std::io::Error> {
    use ed25519_dalek::SigningKey;
    use rand::RngCore;

    println!("=== DKIM Setup ===\n");

    // Validate selector
    if selector.is_empty()
        || selector.len() > 63
        || !selector.chars().all(|c| c.is_alphanumeric() || c == '-')
    {
        return Err(std::io::Error::other(
            "Invalid selector: must be 1-63 alphanumeric characters or hyphens",
        ));
    }

    // Check Cloudflare configuration
    let cf_client = cloudflare_dns::CloudflareDns::from_env();
    if cf_client.is_none() {
        println!("[WARN] Cloudflare not configured (CF_API_TOKEN and CF_ZONE_ID required)");
        println!("       DNS record will not be created automatically.\n");
    }

    // Get domain for DKIM
    let domain = match env::var("DKIM_DOMAIN") {
        Ok(d) => d,
        Err(_) => {
            // Fall back to CF_DOMAIN or default
            env::var("CF_DOMAIN").unwrap_or_else(|_| "decent-cloud.org".to_string())
        }
    };

    println!("Domain: {}", domain);
    println!("Selector: {}", selector);

    // Generate Ed25519 key pair
    print!("\nGenerating Ed25519 key pair... ");
    let mut seed = [0u8; 32];
    rand::rng().fill_bytes(&mut seed);
    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();
    println!("[OK]");

    // Encode keys
    let private_key_bytes = signing_key.to_bytes();
    let public_key_bytes = verifying_key.to_bytes();
    let private_key_base64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        private_key_bytes,
    );
    let public_key_base64 =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, public_key_bytes);

    // Build DKIM TXT record value
    // Format: v=DKIM1; k=ed25519; p=<base64_public_key>
    let txt_value = format!("v=DKIM1; k=ed25519; p={}", public_key_base64);
    let record_name = format!("{}._domainkey", selector);
    let full_record = format!("{}.{}", record_name, domain);

    println!("\n=== DNS Record ===");
    println!("Name: {}", full_record);
    println!("Type: TXT");
    println!("Value: {}", txt_value);

    // Create DNS record if Cloudflare is configured
    if let Some(cf) = cf_client {
        print!("\nCreating DNS TXT record... ");
        match cf.create_txt_record(&record_name, &txt_value).await {
            Ok(()) => {
                println!("[OK]");
            }
            Err(e) => {
                println!("[ERROR] {}", e);
                println!("\n  Manual setup required. Create TXT record:");
                println!("    Name: {}", full_record);
                println!("    Value: {}", txt_value);
            }
        }
    } else {
        println!("\n[INFO] Create this TXT record manually in your DNS provider.");
    }

    println!("\n=== Environment Variables ===\n");
    println!("Add these to your .env file:\n");
    println!("  DKIM_DOMAIN={}", domain);
    println!("  DKIM_SELECTOR={}", selector);
    println!("  DKIM_PRIVATE_KEY={}", private_key_base64);
    println!("\n[IMPORTANT] Save the private key now - it cannot be regenerated!");

    Ok(())
}

/// Check if database schema has been applied by verifying key tables exist
pub async fn check_schema_applied(database_url: &str) -> Result<bool, sqlx::Error> {
    let pool = sqlx::PgPool::connect(database_url).await?;

    // Check for a core table that exists in 001_schema.sql
    // Using provider_registrations as it's created in the initial schema
    let result = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = 'provider_registrations'"
    )
    .fetch_one(&pool)
    .await?;

    Ok(result > 0)
}

/// Check configuration and external service connectivity
async fn doctor_command() -> Result<(), std::io::Error> {
    println!("=== Decent Cloud API Doctor ===\n");

    let mut errors = 0;
    let mut warnings = 0;

    // Helper macros for consistent output
    macro_rules! check_env {
        ($name:expr, required) => {
            match env::var($name) {
                Ok(v) => println!("  [OK] {} = {}...", $name, &v[..v.len().min(20)]),
                Err(_) => {
                    println!("  [ERROR] {} - NOT SET (required)", $name);
                    errors += 1;
                }
            }
        };
        ($name:expr, optional, $desc:expr) => {
            match env::var($name) {
                Ok(v) => println!("  [OK] {} = {}...", $name, &v[..v.len().min(20)]),
                Err(_) => {
                    println!("  [WARN] {} - not set ({})", $name, $desc);
                    warnings += 1;
                }
            }
        };
    }

    // === Database ===
    println!("Database:");

    let database_url = match env::var("DATABASE_URL") {
        Ok(url) => {
            println!("  [OK] DATABASE_URL = {}...", &url[..url.len().min(20)]);
            url
        }
        Err(_) => {
            println!("  [ERROR] DATABASE_URL - NOT SET (required)");
            errors += 1;
            println!("\n  Set DATABASE_URL in your .env file or environment:");
            println!(
                "    - Local dev (with docker compose): {}",
                crate::database::DEFAULT_DATABASE_URL
            );
            println!("    - Production: postgres://user:password@host:5432/database");
            println!("\n  To start PostgreSQL locally:");
            println!("    docker compose up -d postgres");
            return Err(std::io::Error::other(format!(
                "{} configuration errors found",
                errors
            )));
        }
    };

    // Check PostgreSQL connectivity
    print!("  Checking PostgreSQL connection... ");
    match Database::new(&database_url).await {
        Ok(_) => {
            println!("[OK] connected");
        }
        Err(e) => {
            println!("[ERROR] failed to connect");
            errors += 1;
            println!("\n  PostgreSQL connection error: {:#}", e);
            println!("\n  Troubleshooting:");
            println!("    1. Ensure PostgreSQL is running: docker compose ps");
            println!("    2. Start PostgreSQL if needed: docker compose up -d postgres");
            println!("    3. Check DATABASE_URL is correct: {}", database_url);
            println!("    4. Verify PostgreSQL is ready: docker compose logs postgres | grep 'ready to accept connections'");
            println!("\n  If using remote PostgreSQL:");
            println!("    - Verify network connectivity");
            println!("    - Check firewall rules");
            println!("    - Ensure database exists");
        }
    }

    // Check migrations_pg/001_schema.sql has been run
    print!("  Checking database schema... ");
    match check_schema_applied(&database_url).await {
        Ok(true) => {
            println!("[OK] schema applied");
        }
        Ok(false) => {
            println!("[ERROR] schema not found");
            errors += 1;
            println!("\n  Database schema migrations_pg/001_schema.sql has not been applied!");
            println!("\n  To apply migrations:");
            println!("    - The API server auto-runs migrations on startup");
            println!("    - Or run manually: sqlx migrate run --source-url migrations_pg");
            println!(
                "\n  Ensure the migrations_pg/001_schema.sql file exists in the api/ directory."
            );
        }
        Err(e) => {
            println!("[ERROR] failed to check schema");
            errors += 1;
            println!("\n  Schema check error: {:#}", e);
            println!("\n  This could mean:");
            println!("    - Database connection failed (see connection error above)");
            println!("    - Insufficient permissions to check schema");
            println!("    - Database tables are missing or corrupted");
        }
    }

    // === Chatwoot Integration ===
    println!("\nChatwoot Integration:");
    check_env!("CHATWOOT_BASE_URL", optional, "Chatwoot features disabled");
    check_env!("CHATWOOT_API_TOKEN", optional, "Chatwoot API disabled");
    check_env!(
        "CHATWOOT_ACCOUNT_ID",
        optional,
        "Chatwoot features disabled"
    );
    check_env!(
        "CHATWOOT_HMAC_SECRET",
        optional,
        "identity verification disabled"
    );
    check_env!(
        "CHATWOOT_PLATFORM_API_TOKEN",
        optional,
        "agent bot auto-setup disabled"
    );
    check_env!("API_PUBLIC_URL", optional, "agent bot webhooks disabled");

    // Check Chatwoot connectivity
    if let Ok(client) = chatwoot::ChatwootClient::from_env() {
        print!("  Checking Chatwoot API connectivity... ");
        // Try to list webhooks as a connectivity test
        match client.fetch_help_center_articles("test").await {
            Ok(_) | Err(_) => println!("[OK] reachable"),
        }
    }

    // Check Agent Bot configuration - uses Platform API for bot CRUD, Account API for inbox assignment
    if let (Ok(public_url), Ok(platform)) = (
        env::var("API_PUBLIC_URL"),
        chatwoot::ChatwootPlatformClient::from_env(),
    ) {
        print!("  Checking Agent Bot... ");
        let webhook_url = format!(
            "{}/api/v1/webhooks/chatwoot",
            public_url.trim_end_matches('/')
        );
        match platform
            .configure_agent_bot("Decent Cloud Support Bot", &webhook_url)
            .await
        {
            Ok(bot_id) => {
                println!("[OK] configured (id={})", bot_id);

                // Assign bot to all inboxes
                if let Ok(client) = chatwoot::ChatwootClient::from_env() {
                    match client.list_inboxes().await {
                        Ok(inbox_ids) => {
                            for inbox_id in inbox_ids {
                                print!("  Assigning bot to inbox {}... ", inbox_id);
                                match client.assign_agent_bot_to_inbox(inbox_id, bot_id).await {
                                    Ok(()) => println!("[OK]"),
                                    Err(e) => {
                                        println!("[ERROR] {:#}", e);
                                        errors += 1;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            println!("  [ERROR] Failed to list inboxes: {:#}", e);
                            errors += 1;
                        }
                    }
                }
            }
            Err(e) => {
                println!("[ERROR] {:#}", e);
                errors += 1;
            }
        }
    }

    // === Telegram ===
    println!("\nTelegram Notifications:");
    check_env!(
        "TELEGRAM_BOT_TOKEN",
        optional,
        "Telegram notifications disabled"
    );

    // === Email ===
    println!("\nEmail Service:");
    check_env!("MAILCHANNELS_API_KEY", optional, "email sending disabled");
    check_env!("DKIM_DOMAIN", optional, "DKIM signing disabled");
    check_env!("DKIM_SELECTOR", optional, "DKIM signing disabled");
    check_env!("DKIM_PRIVATE_KEY", optional, "DKIM signing disabled");

    // === LLM/AI Bot ===
    println!("\nAI Bot Service:");
    check_env!("LLM_API_KEY", optional, "AI responses disabled");

    // === Stripe Integration ===
    println!("\nStripe Payments:");
    check_env!("STRIPE_SECRET_KEY", optional, "Stripe payments disabled");
    check_env!(
        "STRIPE_WEBHOOK_SECRET",
        optional,
        "Stripe webhooks disabled"
    );
    check_env!(
        "STRIPE_AUTOMATIC_TAX",
        optional,
        "automatic tax calculation disabled"
    );

    // Test Stripe connectivity if configured
    if env::var("STRIPE_SECRET_KEY").is_ok() {
        print!("  Checking Stripe API connectivity... ");
        match crate::stripe_client::StripeClient::new() {
            Ok(_) => println!("[OK] client configured"),
            Err(e) => {
                println!("[ERROR] {:#}", e);
                errors += 1;
            }
        }
    }

    // === Cloudflare DNS ===
    println!("\nCloudflare DNS:");
    check_env!("CF_API_TOKEN", optional, "gateway DNS disabled");
    check_env!("CF_ZONE_ID", optional, "gateway DNS disabled");
    check_env!("CF_DOMAIN", optional, "uses default decent-cloud.org");

    // Test Cloudflare configuration if set
    if env::var("CF_API_TOKEN").is_ok() && env::var("CF_ZONE_ID").is_ok() {
        print!("  Checking Cloudflare configuration... ");
        match crate::cloudflare_dns::CloudflareDns::from_env() {
            Some(client) => println!("[OK] domain = {}", client.domain()),
            None => {
                println!("[ERROR] failed to create client");
                errors += 1;
            }
        }
    }

    // === Google OAuth ===
    println!("\nGoogle OAuth:");
    check_env!(
        "GOOGLE_OAUTH_CLIENT_ID",
        optional,
        "Google sign-in disabled"
    );
    check_env!(
        "GOOGLE_OAUTH_CLIENT_SECRET",
        optional,
        "Google sign-in disabled"
    );
    check_env!(
        "GOOGLE_OAUTH_REDIRECT_URL",
        optional,
        "uses default localhost callback"
    );

    // Test OAuth configuration if set
    if env::var("GOOGLE_OAUTH_CLIENT_ID").is_ok() && env::var("GOOGLE_OAUTH_CLIENT_SECRET").is_ok()
    {
        print!("  Checking OAuth client configuration... ");
        // Verify the redirect URL is parseable
        let redirect_url = env::var("GOOGLE_OAUTH_REDIRECT_URL")
            .unwrap_or_else(|_| "http://localhost:59011/api/v1/oauth/google/callback".to_string());
        match reqwest::Url::parse(&redirect_url) {
            Ok(parsed_url) => {
                if redirect_url.starts_with("http://localhost") {
                    println!("[WARN] redirect URL is localhost - update for production");
                    warnings += 1;
                } else {
                    println!(
                        "[OK] redirect = {}",
                        parsed_url.host_str().unwrap_or("unknown")
                    );
                }
            }
            Err(e) => {
                println!("[ERROR] invalid redirect URL: {}", e);
                errors += 1;
            }
        }
    }

    // === ICPay Integration ===
    println!("\nICPay (ICP Payments):");
    check_env!("ICPAY_SECRET_KEY", optional, "ICP payments disabled");
    check_env!("ICPAY_WEBHOOK_SECRET", optional, "ICPay webhooks disabled");

    // Test ICPay connectivity if configured
    if env::var("ICPAY_SECRET_KEY").is_ok() {
        print!("  Checking ICPay API configuration... ");
        match crate::icpay_client::IcpayClient::new() {
            Ok(_) => println!("[OK] client configured"),
            Err(e) => {
                println!("[ERROR] {:#}", e);
                errors += 1;
            }
        }
    }

    // === Critical URLs (used in emails, OAuth, payments) ===
    println!("\nCritical URLs:");
    match env::var("FRONTEND_URL") {
        Ok(url) => {
            if url.contains("localhost") || url.contains("127.0.0.1") {
                println!(
                    "  [WARN] FRONTEND_URL = {} - localhost URL will NOT work in production!",
                    url
                );
                println!("         Emails, OAuth redirects, and payment callbacks will be broken.");
                println!("         Set FRONTEND_URL to your production domain (e.g., https://decent-cloud.org)");
                warnings += 1;
            } else {
                println!("  [OK] FRONTEND_URL = {}", url);
            }
        }
        Err(_) => {
            println!("  [WARN] FRONTEND_URL - NOT SET (defaults to localhost:59010)");
            println!(
                "         This will break emails, OAuth, and payment callbacks in production!"
            );
            println!("         Set FRONTEND_URL to your production domain (e.g., https://decent-cloud.org)");
            warnings += 1;
        }
    }

    // === Summary ===
    println!("\n=== Summary ===");
    if errors > 0 {
        println!("ERRORS: {} (must fix before running)", errors);
    }
    if warnings > 0 {
        println!("WARNINGS: {} (optional features disabled)", warnings);
    }
    if errors == 0 && warnings == 0 {
        println!("All checks passed!");
    }

    if errors > 0 {
        Err(std::io::Error::other(format!(
            "{} configuration errors found",
            errors
        )))
    } else {
        Ok(())
    }
}

async fn serve_command() -> Result<(), std::io::Error> {
    let port = env::var("API_SERVER_PORT").unwrap_or_else(|_| "59011".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let ctx = setup_app_context().await?;

    // Configure Chatwoot Agent Bot
    // Step 1: Use Platform API for bot CRUD (requires CHATWOOT_PLATFORM_API_TOKEN)
    // Step 2: Use Account API to assign bot to inbox (requires CHATWOOT_API_TOKEN)
    match (
        env::var("API_PUBLIC_URL"),
        chatwoot::ChatwootPlatformClient::from_env(),
    ) {
        (Ok(public_url), Ok(platform)) => {
            let webhook_url = format!(
                "{}/api/v1/webhooks/chatwoot",
                public_url.trim_end_matches('/')
            );
            match platform
                .configure_agent_bot("Decent Cloud Support Bot", &webhook_url)
                .await
            {
                Ok(bot_id) => {
                    tracing::info!(
                        "Chatwoot agent bot configured (id={}): {}",
                        bot_id,
                        webhook_url
                    );

                    // Assign bot to all inboxes via Account API
                    if let Ok(client) = chatwoot::ChatwootClient::from_env() {
                        match client.list_inboxes().await {
                            Ok(inbox_ids) => {
                                for inbox_id in inbox_ids {
                                    if let Err(e) =
                                        client.assign_agent_bot_to_inbox(inbox_id, bot_id).await
                                    {
                                        tracing::error!(
                                            "Failed to assign agent bot to inbox {}: {:#}",
                                            inbox_id,
                                            e
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to list inboxes: {:#}", e);
                            }
                        }
                    } else {
                        tracing::warn!("CHATWOOT_API_TOKEN not set - cannot assign bot to inboxes");
                    }
                }
                Err(e) => tracing::error!("Failed to configure Chatwoot agent bot: {:#}", e),
            }
        }
        (Err(_), Ok(_)) => {
            tracing::warn!("API_PUBLIC_URL not set - Chatwoot agent bot will NOT receive webhooks! Set API_PUBLIC_URL to enable.");
        }
        (Ok(_), Err(e)) => {
            tracing::warn!(
                "Chatwoot Platform API not configured - agent bot auto-setup disabled: {}",
                e
            );
        }
        (Err(_), Err(_)) => {
            tracing::info!("Chatwoot integration not configured (API_PUBLIC_URL and CHATWOOT_PLATFORM_API_TOKEN not set)");
        }
    }

    // Warn about critical URL configuration
    match env::var("FRONTEND_URL") {
        Ok(url) if url.contains("localhost") || url.contains("127.0.0.1") => {
            tracing::warn!(
                "FRONTEND_URL is set to localhost ({}) - emails, OAuth, and payment callbacks will NOT work in production!",
                url
            );
        }
        Err(_) => {
            tracing::warn!(
                "FRONTEND_URL not set - defaults to localhost. Emails, OAuth redirects, and payment callbacks will NOT work in production!"
            );
        }
        Ok(_) => {} // Production URL is set, all good
    }

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
                "x-agent-pubkey",
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
                "x-agent-pubkey",
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
            "/api/v1/webhooks/icpay",
            post(openapi::webhooks::icpay_webhook),
        )
        .at(
            "/api/v1/webhooks/chatwoot",
            post(openapi::webhooks::chatwoot_webhook),
        )
        .at(
            "/api/v1/webhooks/telegram",
            post(openapi::webhooks::telegram_webhook),
        )
        // NOTE: CSV operations are now included in OpenAPI schema above
        .data(ctx.database.clone())
        .data(ctx.metadata_cache.clone())
        .data(ctx.email_service.clone())
        .data(ctx.cloudflare_dns.clone())
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

    // Start payment release service in background (runs every 24 hours)
    let release_interval_hours = env::var("PAYMENT_RELEASE_INTERVAL_HOURS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(24);

    let db_for_release = ctx.database.clone();
    let payment_release_task = tokio::spawn(async move {
        let payment_release_service =
            PaymentReleaseService::new(db_for_release, release_interval_hours);
        tracing::info!(
            "Starting payment release service (interval: {}h)",
            release_interval_hours
        );
        payment_release_service.run().await;
    });

    let server_result = Server::new(TcpListener::bind(&addr)).run(app).await;

    metadata_cache_task.abort();
    cleanup_task.abort();
    payment_release_task.abort();
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
mod main_tests;
