//! Offering subcommand: list/get.
use crate::api_cli;
use crate::Offering;
use anyhow::Result;
use clap::Subcommand;
#[derive(Subcommand)]
pub(crate) enum OfferingAction {
    /// List all offerings
    List {
        /// Filter query (DSL)
        #[arg(long)]
        filter: Option<String>,
        /// Maximum number of results
        #[arg(long, default_value = "50")]
        limit: i32,
    },
    /// Get offering details
    Get {
        /// Offering ID
        offering_id: String,
    },
}
// =============================================================================
// Offering handlers
// =============================================================================

pub(crate) async fn handle_offering_action(action: OfferingAction, api_url: &str) -> Result<()> {
    let http = api::http_util::http_client();

    match action {
        OfferingAction::List { filter, limit } => {
            let mut url = format!("{}/api/v1/offerings?limit={}", api_url, limit);
            if let Some(f) = filter {
                url.push_str(&format!("&q={}", urlencoding::encode(&f)));
            }

            let response = http.get(&url).send().await?;
            let text = response.text().await?;
            let api_response: api_cli::client::ApiResponse<Vec<Offering>> =
                serde_json::from_str(&text)?;
            let offerings = api_response.into_result()?;

            if offerings.is_empty() {
                println!("No offerings found.");
            } else {
                println!("\nOfferings:");
                println!("{}", "=".repeat(100));
                for o in &offerings {
                    println!("ID: {} ({})", o.id, o.offering_id);
                    println!("  Name: {}", o.offer_name.as_deref().unwrap_or("N/A"));
                    println!("  Type: {}", o.product_type.as_deref().unwrap_or("N/A"));
                    println!("  Price: ${:.2}/mo", o.monthly_price.unwrap_or(0.0));
                    println!("{}", "-".repeat(100));
                }
            }
        }
        OfferingAction::Get { offering_id } => {
            let url = format!("{}/api/v1/offerings/{}", api_url, offering_id);
            let response = http.get(&url).send().await?;
            let text = response.text().await?;
            let api_response: api_cli::client::ApiResponse<Offering> = serde_json::from_str(&text)?;
            let offering = api_response.into_result()?;

            println!("Offering: {}", offering.offering_id);
            println!("  ID: {}", offering.id);
            println!(
                "  Name: {}",
                offering.offer_name.as_deref().unwrap_or("N/A")
            );
            println!(
                "  Type: {}",
                offering.product_type.as_deref().unwrap_or("N/A")
            );
            println!("  Price: ${:.2}/mo", offering.monthly_price.unwrap_or(0.0));
            println!(
                "  Stock: {}",
                offering.stock_status.as_deref().unwrap_or("N/A")
            );
        }
    }
    Ok(())
}

