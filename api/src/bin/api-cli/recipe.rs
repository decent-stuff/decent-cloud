//! Recipe subcommand: validate/review/dry-run.
use crate::api_cli;
use anyhow::{Context, Result};
use clap::Subcommand;
use serde::{Deserialize, Serialize};
#[derive(Subcommand)]
pub(crate) enum RecipeAction {
    /// Validate a recipe script without executing it
    Validate {
        /// Recipe script content (use --file for file input)
        #[arg(long, group = "input")]
        script: Option<String>,
        /// Read recipe script from file
        #[arg(long, group = "input")]
        file: Option<String>,
    },
    /// Review a recipe script with the configured LLM
    Review {
        /// Recipe script content (use --file for file input)
        #[arg(long, group = "input")]
        script: Option<String>,
        /// Read recipe script from file
        #[arg(long, group = "input")]
        file: Option<String>,
    },
    /// Dry-run a contract: validate offering recipe and show what would happen
    DryRun {
        /// Offering database ID
        #[arg(long)]
        offering_id: i64,
        /// SSH public key for VM access
        #[arg(long)]
        ssh_pubkey: Option<String>,
    },
}
// =============================================================================
// Recipe handlers
// =============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecipeValidationResponse {
    valid: bool,
    issues: Vec<RecipeIssueResponse>,
}

#[derive(Debug, Deserialize)]
struct RecipeIssueResponse {
    severity: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct ValidateRecipeRequest {
    script: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecipeReviewResponse {
    security_risk: u8,
    completeness: u8,
    user_value: u8,
    summary: String,
    concerns: Vec<String>,
}

fn load_recipe_input(script: Option<String>, file: Option<String>) -> Result<String> {
    match (script, file) {
        (Some(s), None) => Ok(s),
        (None, Some(path)) => std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read recipe file: {}", path)),
        _ => anyhow::bail!("Provide exactly one of --script or --file"),
    }
}

pub(crate) async fn handle_recipe_action(action: RecipeAction, api_url: &str) -> Result<()> {
    let format_severity = |severity: &str| match severity {
        "error" => "ERROR",
        _ => "WARN ",
    };

    match action {
        RecipeAction::Validate { script, file } => {
            let script_content = load_recipe_input(script, file)?;

            let http = api::http_util::http_client();
            let url = format!("{}/api/v1/recipes/validate", api_url);
            let response = http
                .post(&url)
                .json(&ValidateRecipeRequest {
                    script: script_content,
                })
                .send()
                .await?;
            let text = response.text().await?;
            let api_response: api_cli::client::ApiResponse<RecipeValidationResponse> =
                serde_json::from_str(&text)?;
            let result = api_response.into_result()?;

            if result.valid {
                println!("Recipe validation: PASSED");
            } else {
                println!("Recipe validation: FAILED");
            }

            if !result.issues.is_empty() {
                println!("\nIssues:");
                for issue in &result.issues {
                    println!("  [{}] {}", format_severity(&issue.severity), issue.message);
                }
            } else {
                println!("  No issues found.");
            }

            if !result.valid {
                anyhow::bail!("Recipe validation failed");
            }
        }
        RecipeAction::Review { script, file } => {
            let script_content = load_recipe_input(script, file)?;

            let http = api::http_util::http_client();
            let url = format!("{}/api/v1/recipes/review", api_url);
            let response = http
                .post(&url)
                .json(&ValidateRecipeRequest {
                    script: script_content,
                })
                .send()
                .await?;
            let text = response.text().await?;
            let api_response: api_cli::client::ApiResponse<RecipeReviewResponse> =
                serde_json::from_str(&text)?;
            let result = api_response.into_result()?;

            println!("Recipe LLM review:");
            println!("  Security risk: {}/10", result.security_risk);
            println!("  Completeness: {}/10", result.completeness);
            println!("  User value:   {}/10", result.user_value);
            println!("\nSummary:");
            println!("  {}", result.summary);

            if !result.concerns.is_empty() {
                println!("\nConcerns:");
                for concern in &result.concerns {
                    println!("  - {}", concern);
                }
            }
        }
        RecipeAction::DryRun {
            offering_id,
            ssh_pubkey,
        } => {
            let http = api::http_util::http_client();

            println!("Dry-run: simulating contract for offering {}", offering_id);

            let url = format!("{}/api/v1/offerings/{}", api_url, offering_id);
            let response = http.get(&url).send().await?;
            let text = response.text().await?;
            let api_response: api_cli::client::ApiResponse<serde_json::Value> =
                serde_json::from_str(&text)?;
            let offering = api_response.into_result()?;

            let name = offering["offerName"].as_str().unwrap_or("N/A");
            let price = offering["monthlyPrice"]
                .as_f64()
                .map(|p| format!("${:.2}/mo", p))
                .unwrap_or_else(|| "N/A".to_string());
            let ptype = offering["productType"].as_str().unwrap_or("N/A");
            let country = offering["datacenterCountry"].as_str().unwrap_or("N/A");
            let stock = offering["stockStatus"].as_str().unwrap_or("N/A");
            let script = offering["postProvisionScript"].as_str();

            println!("\n  Offering: {}", name);
            println!("  Type: {}", ptype);
            println!("  Price: {}", price);
            println!("  Location: {}", country);
            println!("  Stock: {}", stock);
            if let Some(key) = &ssh_pubkey {
                println!("  SSH key: {}...", &key[..key.len().min(40)]);
            }

            if let Some(script_content) = script {
                println!(
                    "\n  Recipe: {} bytes, {} lines",
                    script_content.len(),
                    script_content.lines().count()
                );

                if !script_content.trim().is_empty() {
                    let preview_lines: Vec<&str> = script_content.lines().take(5).collect();
                    println!("  Preview:");
                    for line in &preview_lines {
                        println!("    | {}", line);
                    }
                    let total_lines = script_content.lines().count();
                    if total_lines > 5 {
                        println!("    | ... ({} more lines)", total_lines - 5);
                    }

                    println!("\n  Validating recipe...");
                    let validate_url = format!("{}/api/v1/recipes/validate", api_url);
                    let validate_response = http
                        .post(&validate_url)
                        .json(&ValidateRecipeRequest {
                            script: script_content.to_string(),
                        })
                        .send()
                        .await?;
                    let validate_text = validate_response.text().await?;
                    let validate_api: api_cli::client::ApiResponse<RecipeValidationResponse> =
                        serde_json::from_str(&validate_text)?;
                    let validate_result = validate_api.into_result()?;

                    if validate_result.valid {
                        println!("  Recipe validation: PASSED");
                    } else {
                        println!("  Recipe validation: FAILED");
                    }
                    for issue in &validate_result.issues {
                        println!(
                            "    [{}] {}",
                            format_severity(&issue.severity),
                            issue.message
                        );
                    }

                    if !validate_result.valid {
                        anyhow::bail!("Dry-run aborted: recipe validation failed");
                    }

                    println!("\n  Requesting LLM review...");
                    let review_url = format!("{}/api/v1/recipes/review", api_url);
                    let review_response = http
                        .post(&review_url)
                        .json(&ValidateRecipeRequest {
                            script: script_content.to_string(),
                        })
                        .send()
                        .await?;
                    let review_text = review_response.text().await?;
                    let review_api: api_cli::client::ApiResponse<RecipeReviewResponse> =
                        serde_json::from_str(&review_text)?;
                    let review_result = review_api.into_result()?;

                    println!("  LLM security risk: {}/10", review_result.security_risk);
                    println!("  LLM completeness: {}/10", review_result.completeness);
                    println!("  LLM user value:   {}/10", review_result.user_value);
                    println!("  LLM summary: {}", review_result.summary);
                }
            } else {
                println!("\n  Recipe: (none)");
            }

            println!("\n  Dry-run complete: contract would be created successfully.");
        }
    }
    Ok(())
}

