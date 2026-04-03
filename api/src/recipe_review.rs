use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::llm_client::{call_llm_api, truncate};

const RECIPE_REVIEW_MAX_TOKENS: u32 = 768;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecipeReview {
    pub security_risk: u8,
    pub completeness: u8,
    pub user_value: u8,
    pub summary: String,
    pub concerns: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RecipeReviewRaw {
    security_risk: u8,
    completeness: u8,
    user_value: u8,
    summary: String,
    concerns: ConcernsField,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ConcernsField {
    List(Vec<String>),
    Text(String),
}

impl RecipeReviewRaw {
    fn into_review(self) -> Result<RecipeReview> {
        validate_score("security_risk", self.security_risk)?;
        validate_score("completeness", self.completeness)?;
        validate_score("user_value", self.user_value)?;

        let concerns = match self.concerns {
            ConcernsField::List(items) => items,
            ConcernsField::Text(text) => vec![text],
        }
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect();

        Ok(RecipeReview {
            security_risk: self.security_risk,
            completeness: self.completeness,
            user_value: self.user_value,
            summary: self.summary.trim().to_string(),
            concerns,
        })
    }
}

pub async fn review_recipe(script: &str) -> Result<RecipeReview> {
    let prompt = format!(
        concat!(
            "You are reviewing a VM provisioning recipe for a cloud marketplace. ",
            "Assess the recipe on three dimensions and return JSON only.\n\n",
            "Return a JSON object with exactly these keys: security_risk, completeness, user_value, summary, concerns.\n",
            "- security_risk: integer 1-10, where 1 is very safe and 10 is highly risky or malicious\n",
            "- completeness: integer 1-10, where 1 is likely broken and 10 is likely to work end-to-end\n",
            "- user_value: integer 1-10, where 1 has little practical value and 10 is highly useful to end users\n",
            "- summary: short plain-English summary\n",
            "- concerns: array of short plain-English bullet strings\n\n",
            "Focus on concrete operational and security signals in the recipe itself. ",
            "Do not invent unavailable context.\n\n",
            "Recipe:\n{}"
        ),
        script
    );

    let response = call_llm_api(&prompt, RECIPE_REVIEW_MAX_TOKENS)
        .await
        .context("Failed to review recipe with LLM")?;

    parse_recipe_review_response(&response)
}

fn validate_score(name: &str, value: u8) -> Result<()> {
    if (1..=10).contains(&value) {
        Ok(())
    } else {
        anyhow::bail!("LLM returned invalid {} score: {}", name, value);
    }
}

fn parse_recipe_review_response(response: &str) -> Result<RecipeReview> {
    let json = strip_code_fences(response);
    let parsed: RecipeReviewRaw = serde_json::from_str(json).with_context(|| {
        format!(
            "Failed to parse recipe review JSON. Raw response: {}",
            truncate(response.trim(), 500)
        )
    })?;
    parsed.into_review()
}

fn strip_code_fences(response: &str) -> &str {
    let trimmed = response.trim();
    if let Some(inner) = trimmed.strip_prefix("```") {
        let inner = inner.trim_start();
        let inner = if let Some(after_lang) = inner.strip_prefix("json") {
            after_lang
        } else {
            inner
        };
        if let Some(inner) = inner.strip_suffix("```") {
            return inner.trim();
        }
    }
    trimmed
}

#[cfg(test)]
mod tests {
    use super::{parse_recipe_review_response, RecipeReview};

    #[test]
    fn test_parse_recipe_review_response_accepts_fenced_json() {
        let review = parse_recipe_review_response(
            r#"```json
{
  "security_risk": 8,
  "completeness": 6,
  "user_value": 7,
  "summary": "High-risk but likely functional.",
  "concerns": ["Runs unverified code", "No idempotency checks"]
}
```"#,
        )
        .unwrap();

        assert_eq!(
            review,
            RecipeReview {
                security_risk: 8,
                completeness: 6,
                user_value: 7,
                summary: "High-risk but likely functional.".to_string(),
                concerns: vec![
                    "Runs unverified code".to_string(),
                    "No idempotency checks".to_string()
                ],
            }
        );
    }

    #[test]
    fn test_parse_recipe_review_response_accepts_string_concerns() {
        let review = parse_recipe_review_response(
            r#"{
  "security_risk": 10,
  "completeness": 3,
  "user_value": 1,
  "summary": "Malicious recipe.",
  "concerns": "Adds a backdoor account"
}"#,
        )
        .unwrap();

        assert_eq!(review.concerns, vec!["Adds a backdoor account".to_string()]);
    }
}
