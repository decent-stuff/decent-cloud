//! Health subcommand: api/all/cloudflare/stripe/mailchannels/telegram checks.
use anyhow::{Context, Result};
use clap::Subcommand;
use std::env;
#[derive(Subcommand)]
pub(crate) enum HealthAction {
    /// Check API health
    Api,
    /// Check all external services
    All,
    /// Check Cloudflare DNS
    Cloudflare,
    /// Check Stripe
    Stripe,
    /// Check MailChannels
    Mailchannels,
    /// Check Telegram Bot
    Telegram,
}
// =============================================================================
// Health handlers
// =============================================================================

pub(crate) async fn handle_health_action(action: HealthAction, api_url: &str) -> Result<()> {
    let http = api::http_util::http_client();

    async fn check_health(name: &str, result: Result<String, anyhow::Error>) {
        match result {
            Ok(time) => println!("{}: ✓ healthy ({})", name, time),
            Err(e) => println!("{}: ✗ unhealthy - {}", name, e),
        }
    }

    match action {
        HealthAction::Api => {
            let start = std::time::Instant::now();
            let url = format!("{}/api/v1/offerings?limit=1", api_url);
            let result = http.get(&url).send().await;
            match result {
                Ok(resp) if resp.status().is_success() => {
                    println!(
                        "API Server: ✓ healthy ({:.0}ms)",
                        start.elapsed().as_millis()
                    );
                }
                Ok(resp) => {
                    println!("API Server: ✗ unhealthy - status {}", resp.status());
                }
                Err(e) => {
                    println!("API Server: ✗ unhealthy - {}", e);
                }
            }
        }
        HealthAction::All => {
            println!("\nService Health Checks:");
            println!("{}", "=".repeat(60));

            // API
            let start = std::time::Instant::now();
            let url = format!("{}/api/v1/offerings?limit=1", api_url);
            let api_result = http
                .get(&url)
                .send()
                .await
                .map(|_| format!("{:.0}ms", start.elapsed().as_millis()))
                .map_err(|e| anyhow::anyhow!("{}", e));
            check_health("API Server", api_result).await;

            // Database (via API health)
            let start = std::time::Instant::now();
            let url = format!("{}/api/v1/providers?limit=1", api_url);
            let db_result = http
                .get(&url)
                .send()
                .await
                .map(|_| format!("{:.0}ms", start.elapsed().as_millis()))
                .map_err(|e| anyhow::anyhow!("{}", e));
            check_health("Database", db_result).await;

            // Cloudflare
            if env::var("CLOUDFLARE_API_TOKEN").is_ok() {
                let start = std::time::Instant::now();
                let cf_result = http
                    .get("https://api.cloudflare.com/client/v4/user/tokens/verify")
                    .header(
                        "Authorization",
                        format!("Bearer {}", env::var("CLOUDFLARE_API_TOKEN").unwrap()),
                    )
                    .send()
                    .await
                    .map(|_| format!("{:.0}ms", start.elapsed().as_millis()))
                    .map_err(|e| anyhow::anyhow!("{}", e));
                check_health("Cloudflare DNS", cf_result).await;
            } else {
                println!("Cloudflare DNS: - not configured");
            }

            // Stripe
            if env::var("STRIPE_SECRET_KEY").is_ok() {
                let start = std::time::Instant::now();
                let stripe_result = http
                    .get(format!(
                        "{}/v1/balance",
                        api::stripe_client::STRIPE_API_BASE
                    ))
                    .header(
                        "Authorization",
                        format!("Bearer {}", env::var("STRIPE_SECRET_KEY").unwrap()),
                    )
                    .send()
                    .await
                    .map(|_| format!("{:.0}ms", start.elapsed().as_millis()))
                    .map_err(|e| anyhow::anyhow!("{}", e));
                check_health("Stripe", stripe_result).await;
            } else {
                println!("Stripe: - not configured");
            }

            // MailChannels
            if env::var("MAILCHANNELS_API_KEY").is_ok() {
                println!("MailChannels: - configured (no health endpoint)");
            } else {
                println!("MailChannels: - not configured");
            }

            // Telegram
            if let Ok(token) = env::var("TELEGRAM_BOT_TOKEN") {
                let start = std::time::Instant::now();
                let url = format!("https://api.telegram.org/bot{}/getMe", token);
                let tg_result = http
                    .get(&url)
                    .send()
                    .await
                    .map(|_| format!("{:.0}ms", start.elapsed().as_millis()))
                    .map_err(|e| anyhow::anyhow!("{}", e));
                check_health("Telegram Bot", tg_result).await;
            } else {
                println!("Telegram Bot: - not configured");
            }

            println!("{}", "=".repeat(60));
        }
        HealthAction::Cloudflare => {
            let token = env::var("CLOUDFLARE_API_TOKEN").context("CLOUDFLARE_API_TOKEN not set")?;
            let start = std::time::Instant::now();
            let response = http
                .get("https://api.cloudflare.com/client/v4/user/tokens/verify")
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await?;
            if response.status().is_success() {
                println!(
                    "Cloudflare DNS: ✓ healthy ({:.0}ms)",
                    start.elapsed().as_millis()
                );
            } else {
                let text = response.text().await?;
                println!("Cloudflare DNS: ✗ unhealthy - {}", text);
            }
        }
        HealthAction::Stripe => {
            let key = env::var("STRIPE_SECRET_KEY").context("STRIPE_SECRET_KEY not set")?;
            let start = std::time::Instant::now();
            let response = http
                .get(format!("{}/v1/balance", api::stripe_client::STRIPE_API_BASE))
                .header("Authorization", format!("Bearer {}", key))
                .send()
                .await?;
            if response.status().is_success() {
                println!("Stripe: ✓ healthy ({:.0}ms)", start.elapsed().as_millis());
            } else {
                let text = response.text().await?;
                println!("Stripe: ✗ unhealthy - {}", text);
            }
        }
        HealthAction::Mailchannels => {
            if env::var("MAILCHANNELS_API_KEY").is_ok() {
                println!("MailChannels: ✓ configured (no health endpoint available)");
            } else {
                println!("MailChannels: ✗ not configured (MAILCHANNELS_API_KEY not set)");
            }
        }
        HealthAction::Telegram => {
            let token = env::var("TELEGRAM_BOT_TOKEN").context("TELEGRAM_BOT_TOKEN not set")?;
            let start = std::time::Instant::now();
            let url = format!("https://api.telegram.org/bot{}/getMe", token);
            let response = http.get(&url).send().await?;
            if response.status().is_success() {
                println!(
                    "Telegram Bot: ✓ healthy ({:.0}ms)",
                    start.elapsed().as_millis()
                );
            } else {
                let text = response.text().await?;
                println!("Telegram Bot: ✗ unhealthy - {}", text);
            }
        }
    }
    Ok(())
}

