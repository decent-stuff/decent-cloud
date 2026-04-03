use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
}

#[derive(Debug, Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

#[derive(Debug, Deserialize)]
struct ClaudeContent {
    text: String,
}

pub async fn call_llm_api(prompt: &str, max_tokens: u32) -> Result<String> {
    let api_key = std::env::var("LLM_API_KEY").context("LLM_API_KEY not set")?;

    let api_url = std::env::var("LLM_API_URL")
        .unwrap_or_else(|_| "https://api.anthropic.com/v1/messages".to_string());

    let api_model =
        std::env::var("LLM_API_MODEL").unwrap_or_else(|_| "claude-4.5-sonnet".to_string());

    let client = Client::new();

    let request = ClaudeRequest {
        model: api_model,
        max_tokens,
        messages: vec![ClaudeMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
    };

    let resp = client
        .post(&api_url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await
        .context("Failed to send LLM API request")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp
            .text()
            .await
            .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
        anyhow::bail!("LLM API error {}: {}", status, body);
    }

    let body = resp
        .text()
        .await
        .context("Failed to read LLM response body")?;

    let response: ClaudeResponse = serde_json::from_str(&body).with_context(|| {
        format!(
            "Failed to parse LLM API response. Body: {}",
            truncate(&body, 500)
        )
    })?;

    response
        .content
        .first()
        .map(|c| c.text.clone())
        .context("LLM response contained no content")
}

pub(crate) fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}
