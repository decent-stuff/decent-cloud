use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LlmProvider {
    Anthropic,
    OpenAi,
}

impl LlmProvider {
    fn as_str(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAi => "openai",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LlmConfig {
    pub(crate) provider: &'static str,
    pub(crate) api_url: String,
    pub(crate) api_model: String,
    api_key: String,
}

fn detect_provider(url: &str) -> LlmProvider {
    let url = url.to_lowercase();
    let env_override = std::env::var("LLM_PROVIDER").unwrap_or_default();
    match env_override.to_lowercase().as_str() {
        "openai" => LlmProvider::OpenAi,
        "anthropic" => LlmProvider::Anthropic,
        _ => {
            if url.contains("openai.com")
                || url.contains("/chat/completions")
                || url.contains("/openai/deployments/")
            {
                LlmProvider::OpenAi
            } else {
                LlmProvider::Anthropic
            }
        }
    }
}

fn resolve_provider(url: &str) -> Result<LlmProvider> {
    let env_override = std::env::var("LLM_PROVIDER").unwrap_or_default();
    let env_override = env_override.trim();

    match env_override.to_lowercase().as_str() {
        "" => Ok(detect_provider(url)),
        "openai" => Ok(LlmProvider::OpenAi),
        "anthropic" => Ok(LlmProvider::Anthropic),
        other => anyhow::bail!(
            "LLM_PROVIDER must be 'anthropic' or 'openai', got '{}'",
            other
        ),
    }
}

fn normalize_api_url(url: &str, provider: LlmProvider) -> String {
    let trimmed = url.trim_end_matches('/');
    let lowercase = trimmed.to_lowercase();

    if lowercase.contains("/messages") || lowercase.contains("/chat/completions") {
        return trimmed.to_string();
    }

    let suffix = if lowercase.ends_with("/v1") {
        match provider {
            LlmProvider::Anthropic => "/messages",
            LlmProvider::OpenAi => "/chat/completions",
        }
    } else if lowercase.contains("/openai/deployments/") {
        "/chat/completions"
    } else {
        match provider {
            LlmProvider::Anthropic => "/v1/messages",
            LlmProvider::OpenAi => "/v1/chat/completions",
        }
    };

    format!("{trimmed}{suffix}")
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum LlmRequest {
    Anthropic {
        model: String,
        max_tokens: u32,
        messages: Vec<LlmMessage>,
    },
    OpenAi {
        model: String,
        max_tokens: u32,
        messages: Vec<LlmMessage>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct LlmMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    text: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    content: String,
}

pub(crate) fn load_llm_config() -> Result<Option<LlmConfig>> {
    let api_key = match std::env::var("LLM_API_KEY") {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };

    if api_key.trim().is_empty() {
        anyhow::bail!("LLM_API_KEY is set but empty");
    }

    let api_url = std::env::var("LLM_API_URL")
        .unwrap_or_else(|_| "https://api.anthropic.com/v1/messages".to_string());
    let provider = resolve_provider(&api_url)?;
    let api_url = normalize_api_url(&api_url, provider);
    reqwest::Url::parse(&api_url)
        .with_context(|| format!("LLM_API_URL is invalid after normalization: {}", api_url))?;

    let api_model =
        std::env::var("LLM_API_MODEL").unwrap_or_else(|_| "claude-4.5-sonnet".to_string());
    if api_model.trim().is_empty() {
        anyhow::bail!("LLM_API_MODEL is set but empty");
    }

    Ok(Some(LlmConfig {
        provider: provider.as_str(),
        api_url,
        api_model,
        api_key,
    }))
}

pub async fn call_llm_api(prompt: &str, max_tokens: u32) -> Result<String> {
    let config = load_llm_config()?.context("LLM_API_KEY not set")?;
    let provider = resolve_provider(&config.api_url)?;

    let message = LlmMessage {
        role: "user".to_string(),
        content: prompt.to_string(),
    };

    let client = Client::new();

    let request = match provider {
        LlmProvider::Anthropic => LlmRequest::Anthropic {
            model: config.api_model.clone(),
            max_tokens,
            messages: vec![message],
        },
        LlmProvider::OpenAi => LlmRequest::OpenAi {
            model: config.api_model.clone(),
            max_tokens,
            messages: vec![message],
        },
    };

    let mut req = client
        .post(&config.api_url)
        .header("content-type", "application/json")
        .json(&request);

    match provider {
        LlmProvider::Anthropic => {
            req = req
                .header("x-api-key", &config.api_key)
                .header("anthropic-version", "2023-06-01");
        }
        LlmProvider::OpenAi => {
            req = req.header("authorization", format!("Bearer {}", config.api_key));
        }
    }

    let resp = req.send().await.context("Failed to send LLM API request")?;

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

    match provider {
        LlmProvider::Anthropic => {
            let response: AnthropicResponse = serde_json::from_str(&body).with_context(|| {
                format!(
                    "Failed to parse Anthropic response. Body: {}",
                    truncate(&body, 500)
                )
            })?;
            response
                .content
                .first()
                .map(|c| c.text.clone())
                .context("Anthropic response contained no content")
        }
        LlmProvider::OpenAi => {
            let response: OpenAiResponse = serde_json::from_str(&body).with_context(|| {
                format!(
                    "Failed to parse OpenAI response. Body: {}",
                    truncate(&body, 500)
                )
            })?;
            response
                .choices
                .first()
                .map(|c| c.message.content.clone())
                .context("OpenAI response contained no choices")
        }
    }
}

pub(crate) fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn clear_llm_env() {
        std::env::remove_var("LLM_API_KEY");
        std::env::remove_var("LLM_API_URL");
        std::env::remove_var("LLM_API_MODEL");
        std::env::remove_var("LLM_PROVIDER");
    }

    #[test]
    fn test_detect_provider_anthropic_default() {
        assert_eq!(
            detect_provider("https://api.anthropic.com/v1/messages"),
            LlmProvider::Anthropic
        );
    }

    #[test]
    fn test_detect_provider_openai_url() {
        assert_eq!(
            detect_provider("https://api.openai.com/v1/chat/completions"),
            LlmProvider::OpenAi
        );
    }

    #[test]
    fn test_detect_provider_chat_completions_path() {
        assert_eq!(
            detect_provider("https://my-llm-proxy.example.com/v1/chat/completions"),
            LlmProvider::OpenAi
        );
    }

    #[test]
    fn test_detect_provider_anthropic_path() {
        assert_eq!(
            detect_provider("https://my-llm-proxy.example.com/v1/messages"),
            LlmProvider::Anthropic
        );
    }

    #[test]
    fn test_detect_provider_openai_deployment_path() {
        assert_eq!(
            detect_provider("https://example.openai.azure.com/openai/deployments/gpt-4o"),
            LlmProvider::OpenAi
        );
    }

    #[test]
    fn test_normalize_api_url_appends_openai_endpoint_to_base_url() {
        assert_eq!(
            normalize_api_url("https://api.openai.com", LlmProvider::OpenAi),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_normalize_api_url_appends_openai_endpoint_to_v1_base_url() {
        assert_eq!(
            normalize_api_url("https://api.openai.com/v1", LlmProvider::OpenAi),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_normalize_api_url_appends_openai_endpoint_to_deployment_base_url() {
        assert_eq!(
            normalize_api_url(
                "https://example.openai.azure.com/openai/deployments/gpt-4o",
                LlmProvider::OpenAi
            ),
            "https://example.openai.azure.com/openai/deployments/gpt-4o/chat/completions"
        );
    }

    #[test]
    fn test_normalize_api_url_appends_anthropic_endpoint_to_base_url() {
        assert_eq!(
            normalize_api_url("https://api.anthropic.com", LlmProvider::Anthropic),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn test_normalize_api_url_preserves_existing_endpoint() {
        assert_eq!(
            normalize_api_url(
                "https://my-llm-proxy.example.com/v1/chat/completions",
                LlmProvider::OpenAi
            ),
            "https://my-llm-proxy.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long() {
        assert_eq!(truncate("hello world", 5), "hello");
    }

    #[test]
    fn test_truncate_empty() {
        assert_eq!(truncate("", 5), "");
    }

    #[test]
    fn test_anthropic_response_parsing() {
        let body = r#"{"content":[{"text":"Hello from Claude"}]}"#;
        let response: AnthropicResponse = serde_json::from_str(body).unwrap();
        assert_eq!(response.content[0].text, "Hello from Claude");
    }

    #[test]
    fn test_openai_response_parsing() {
        let body = r#"{"choices":[{"message":{"content":"Hello from GPT"}}]}"#;
        let response: OpenAiResponse = serde_json::from_str(body).unwrap();
        assert_eq!(response.choices[0].message.content, "Hello from GPT");
    }

    #[test]
    #[serial]
    fn test_load_llm_config_normalizes_openai_base_url() {
        clear_llm_env();
        std::env::set_var("LLM_API_KEY", "test-key");
        std::env::set_var("LLM_API_URL", "https://api.openai.com/v1");
        std::env::set_var("LLM_API_MODEL", "gpt-4.1-mini");

        let config = load_llm_config().unwrap().unwrap();

        assert_eq!(config.provider, "openai");
        assert_eq!(config.api_url, "https://api.openai.com/v1/chat/completions");
        assert_eq!(config.api_model, "gpt-4.1-mini");

        clear_llm_env();
    }

    #[test]
    #[serial]
    fn test_load_llm_config_rejects_invalid_provider_override() {
        clear_llm_env();
        std::env::set_var("LLM_API_KEY", "test-key");
        std::env::set_var("LLM_PROVIDER", "azure");

        let error = load_llm_config().unwrap_err().to_string();

        assert!(error.contains("LLM_PROVIDER must be 'anthropic' or 'openai'"));

        clear_llm_env();
    }
}
