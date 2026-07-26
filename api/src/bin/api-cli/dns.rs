//! DNS subcommand: create/get/delete/list Cloudflare records.
use anyhow::{Context, Result};
use clap::Subcommand;
use std::env;
#[derive(Subcommand)]
pub(crate) enum DnsAction {
    /// Create a DNS record
    Create {
        /// Subdomain name
        #[arg(long)]
        subdomain: String,
        /// IP address
        #[arg(long)]
        ip: String,
    },
    /// Get a DNS record
    Get {
        /// Subdomain name
        #[arg(long)]
        subdomain: String,
    },
    /// Delete a DNS record
    Delete {
        /// Subdomain name
        #[arg(long)]
        subdomain: String,
    },
    /// List all DC subdomain records
    List,
}
// =============================================================================
// DNS handlers
// =============================================================================

pub(crate) async fn handle_dns_action(action: DnsAction) -> Result<()> {
    let api_token = env::var("CLOUDFLARE_API_TOKEN").context("CLOUDFLARE_API_TOKEN not set")?;
    let zone_id = env::var("CLOUDFLARE_ZONE_ID").context("CLOUDFLARE_ZONE_ID not set")?;
    let gw_prefix = env::var("CF_GW_PREFIX").unwrap_or_else(|_| "gw".to_string());
    let domain = env::var("CF_DOMAIN").unwrap_or_else(|_| "decent-cloud.org".to_string());
    let base_domain = format!("{}.{}", gw_prefix, domain);

    let http = api::http_util::http_client();
    let base_url = format!(
        "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
        zone_id
    );

    match action {
        DnsAction::Create { subdomain, ip } => {
            let full_name = format!("{}.{}", subdomain, base_domain);
            let params = serde_json::json!({
                "type": "A",
                "name": full_name,
                "content": ip,
                "ttl": 300,
                "proxied": false,
            });

            let response = http
                .post(&base_url)
                .header("Authorization", format!("Bearer {}", api_token))
                .json(&params)
                .send()
                .await?;

            let text = response.text().await?;
            let json: serde_json::Value = serde_json::from_str(&text)?;

            if json["success"].as_bool().unwrap_or(false) {
                println!("DNS record created: {} -> {}", full_name, ip);
            } else {
                anyhow::bail!("Failed to create DNS record: {}", text);
            }
        }
        DnsAction::Get { subdomain } => {
            let full_name = format!("{}.{}", subdomain, base_domain);
            let url = format!("{}?name={}", base_url, urlencoding::encode(&full_name));

            let response = http
                .get(&url)
                .header("Authorization", format!("Bearer {}", api_token))
                .send()
                .await?;

            let text = response.text().await?;
            let json: serde_json::Value = serde_json::from_str(&text)?;

            if let Some(records) = json["result"].as_array() {
                if records.is_empty() {
                    println!("No DNS record found for {}", full_name);
                } else {
                    for record in records {
                        println!("DNS Record:");
                        println!("  Name: {}", record["name"]);
                        println!("  Type: {}", record["type"]);
                        println!("  Content: {}", record["content"]);
                        println!("  TTL: {}", record["ttl"]);
                    }
                }
            }
        }
        DnsAction::Delete { subdomain } => {
            let full_name = format!("{}.{}", subdomain, base_domain);
            let url = format!("{}?name={}", base_url, urlencoding::encode(&full_name));

            // First, find the record ID
            let response = http
                .get(&url)
                .header("Authorization", format!("Bearer {}", api_token))
                .send()
                .await?;

            let text = response.text().await?;
            let json: serde_json::Value = serde_json::from_str(&text)?;

            if let Some(records) = json["result"].as_array() {
                if records.is_empty() {
                    println!("No DNS record found for {}", full_name);
                } else {
                    for record in records {
                        if let Some(id) = record["id"].as_str() {
                            let delete_url = format!("{}/{}", base_url, id);
                            let response = http
                                .delete(&delete_url)
                                .header("Authorization", format!("Bearer {}", api_token))
                                .send()
                                .await?;

                            if response.status().is_success() {
                                println!("DNS record deleted: {}", full_name);
                            } else {
                                let text = response.text().await?;
                                anyhow::bail!("Failed to delete DNS record: {}", text);
                            }
                        }
                    }
                }
            }
        }
        DnsAction::List => {
            let response = http
                .get(&base_url)
                .header("Authorization", format!("Bearer {}", api_token))
                .send()
                .await?;

            let text = response.text().await?;
            let json: serde_json::Value = serde_json::from_str(&text)?;

            if let Some(records) = json["result"].as_array() {
                let dc_records: Vec<_> = records
                    .iter()
                    .filter(|r| {
                        r["name"]
                            .as_str()
                            .map(|n| n.contains(&base_domain))
                            .unwrap_or(false)
                    })
                    .collect();

                if dc_records.is_empty() {
                    println!("No DC gateway DNS records found.");
                } else {
                    println!("\nDC Gateway DNS Records:");
                    println!("{}", "=".repeat(80));
                    println!("{:<40} {:<10} {:<20}", "Name", "Type", "Content");
                    println!("{}", "-".repeat(80));
                    for record in &dc_records {
                        println!(
                            "{:<40} {:<10} {:<20}",
                            record["name"].as_str().unwrap_or("N/A"),
                            record["type"].as_str().unwrap_or("N/A"),
                            record["content"].as_str().unwrap_or("N/A")
                        );
                    }
                    println!("{}", "=".repeat(80));
                    println!("Total: {} record(s)", dc_records.len());
                }
            }
        }
    }
    Ok(())
}

