//! anthropic-proxy binary entry point (GitHub issue #427).
//!
//! Host-side reverse proxy for Anthropic Messages-API: injects the platform key
//! per-request, meters usage per identity, streams responses back to the customer
//! container. The key never enters the container.
//!
//! dc-agent spawns this process on the host with `ANTHROPIC_API_KEY` in its env and
//! `--identity <id>` for the single identity on that VM (spec I.1).

use std::env;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use anthropic_proxy::config::Config;
use anthropic_proxy::identity::{IdentityRef, SingleIdentityResolver};
use anthropic_proxy::metering::LoggingRecorder;
use anthropic_proxy::proxy::{build_app, ProxyState};

#[derive(Parser, Debug)]
#[command(
    name = "anthropic-proxy",
    about = "Host-side reverse proxy for the Anthropic Messages API: injects the platform key per-request and meters usage per identity (#427).",
    version
)]
struct Cli {
    /// Address to listen on (the customer container reaches this via the docker gateway IP).
    #[arg(long, default_value = "127.0.0.1:8787")]
    listen: String,

    /// Upstream Anthropic-compatible base URL (no trailing slash, no /v1/messages).
    /// Production: https://api.anthropic.com  Dev/test: https://api.z.ai/api/anthropic
    #[arg(long, default_value = "https://api.anthropic.com")]
    upstream: String,

    /// The single identity this proxy host serves (UUID; written into agent_runs rows).
    #[arg(long)]
    identity: String,

    /// anthropic-version header value injected on every upstream request.
    #[arg(long, default_value = "2023-06-01")]
    anthropic_version: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let cli = Cli::parse();
    // The key is read from env only — never accepted on the CLI, so it never appears
    // in `ps`/shell history.
    let api_key = env::var("ANTHROPIC_API_KEY").context(
        "ANTHROPIC_API_KEY is not set — the proxy holds the platform Anthropic key in \
         memory and injects it per-request. It will NOT start without one.",
    )?;

    let config = Config::new(
        &cli.listen,
        &cli.upstream,
        &api_key,
        &cli.anthropic_version,
        &cli.identity,
    )?;

    tracing::info!(
        listen = %config.listen_addr,
        upstream = %config.upstream,
        identity = %config.identity_id,
        anthropic_version = %config.anthropic_version,
        "anthropic-proxy starting"
    );

    // A connect timeout keeps startup/connect failures loud; NO total timeout so
    // long streaming responses are not aborted.
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .build()
        .context("failed to build upstream HTTP client")?;

    let state = ProxyState {
        config: Arc::new(config.clone()),
        client,
        resolver: std::sync::Arc::new(SingleIdentityResolver::new(IdentityRef::new(
            config.identity_id.clone(),
        ))),
        // Production recorder logs usage; the DB-backed recorder that writes
        // agent_runs.claude_{input,output}_tokens lands with #415/#416.
        recorder: std::sync::Arc::new(LoggingRecorder),
    };

    let app = build_app(state);
    let listener = tokio::net::TcpListener::bind(&config.listen_addr)
        .await
        .with_context(|| format!("failed to bind {}", config.listen_addr))?;
    let bound = listener.local_addr().ok();
    tracing::info!(addr = ?bound, "anthropic-proxy listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;
    tracing::info!("anthropic-proxy stopped");
    Ok(())
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("installing Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("installing SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("received SIGINT, shutting down"),
        _ = terminate => tracing::info!("received SIGTERM, shutting down"),
    }
}
