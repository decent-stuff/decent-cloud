//! Provider subcommand: list/status/offerings/pool-upgrade.
use crate::api_cli::{self, Identity, SignedClient};
use crate::Offering;
use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};
#[derive(Subcommand)]
pub(crate) enum ProviderAction {
    /// List all providers
    List {
        /// Maximum number of results
        #[arg(long, default_value = "50")]
        limit: i32,
    },
    /// Get provider status
    Status {
        /// Provider public key (hex)
        #[arg(long)]
        pubkey: String,
    },
    /// List provider's offerings
    Offerings {
        /// Provider public key (hex)
        #[arg(long)]
        pubkey: String,
    },
    /// Request agent upgrade for a pool
    PoolUpgrade {
        /// Identity name (from ~/.dc-test-keys/)
        #[arg(long)]
        identity: String,
        /// Pool ID
        #[arg(long)]
        pool_id: String,
        /// Target version (e.g. "0.4.21"), omit to cancel pending upgrade
        #[arg(long)]
        version: Option<String>,
    },
}
// =============================================================================
// Provider handlers
// =============================================================================

#[derive(Debug, Deserialize)]
struct ProviderProfile {
    #[serde(default)]
    pubkey: Option<String>,
    name: Option<String>,
    #[serde(alias = "website")]
    website_url: Option<String>,
}

pub(crate) async fn handle_provider_action(action: ProviderAction, api_url: &str) -> Result<()> {
    let http = api::http_util::http_client();

    match action {
        ProviderAction::List { limit } => {
            let url = format!("{}/api/v1/providers?limit={}", api_url, limit);
            let response = http.get(&url).send().await?;
            let text = response.text().await?;
            let api_response: api_cli::client::ApiResponse<Vec<ProviderProfile>> =
                serde_json::from_str(&text)?;
            let providers = api_response.into_result()?;

            if providers.is_empty() {
                println!("No providers found.");
            } else {
                println!("\nProviders:");
                println!("{}", "=".repeat(100));
                println!("{:<66} {:<20} {:<30}", "Public Key", "Name", "Website");
                println!("{}", "-".repeat(100));
                for p in &providers {
                    let name = p.name.as_deref().unwrap_or("N/A");
                    let website = p.website_url.as_deref().unwrap_or("N/A");
                    let pubkey = p.pubkey.as_deref().unwrap_or("N/A");
                    println!("{:<66} {:<20} {:<30}", pubkey, name, website);
                }
                println!("{}", "=".repeat(100));
                println!("Total: {} provider(s)", providers.len());
            }
        }
        ProviderAction::Status { pubkey } => {
            let url = format!("{}/api/v1/providers/{}", api_url, pubkey);
            let response = http.get(&url).send().await?;
            let text = response.text().await?;
            let api_response: api_cli::client::ApiResponse<ProviderProfile> =
                serde_json::from_str(&text)?;
            let provider = api_response.into_result()?;

            println!(
                "Provider: {}",
                provider.pubkey.as_deref().unwrap_or(&pubkey)
            );
            println!("  Name: {}", provider.name.as_deref().unwrap_or("N/A"));
            println!(
                "  Website: {}",
                provider.website_url.as_deref().unwrap_or("N/A")
            );
        }
        ProviderAction::Offerings { pubkey } => {
            let url = format!("{}/api/v1/providers/{}/offerings", api_url, pubkey);
            let response = http.get(&url).send().await?;
            let text = response.text().await?;
            let api_response: api_cli::client::ApiResponse<Vec<Offering>> =
                serde_json::from_str(&text)?;
            let offerings = api_response.into_result()?;

            if offerings.is_empty() {
                println!("No offerings found for this provider.");
            } else {
                println!("\nProvider Offerings:");
                println!("{}", "=".repeat(100));
                for o in &offerings {
                    println!(
                        "ID: {} - {}",
                        o.id,
                        o.offer_name.as_deref().unwrap_or("N/A")
                    );
                    println!(
                        "  Type: {}, Price: ${:.2}/mo, Stock: {}",
                        o.product_type.as_deref().unwrap_or("N/A"),
                        o.monthly_price.unwrap_or(0.0),
                        o.stock_status.as_deref().unwrap_or("N/A")
                    );
                    println!("{}", "-".repeat(100));
                }
                println!("Total: {} offering(s)", offerings.len());
            }
        }
        ProviderAction::PoolUpgrade {
            identity,
            pool_id,
            version,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;
            let path = format!("/providers/{}/pools/{}/upgrade", id.public_key_hex, pool_id);

            #[derive(Serialize)]
            struct UpgradeBody {
                version: Option<String>,
            }

            let result: bool = client
                .post_api(
                    &path,
                    &UpgradeBody {
                        version: version.clone(),
                    },
                )
                .await?;

            if result {
                match &version {
                    Some(v) => println!("Upgrade to {} requested for pool {}", v, pool_id),
                    None => println!("Pending upgrade cancelled for pool {}", pool_id),
                }
            } else {
                println!("Pool {} not found", pool_id);
            }
        }
    }
    Ok(())
}

