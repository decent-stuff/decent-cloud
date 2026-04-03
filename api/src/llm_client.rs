use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LlmProvider {
    Anthropic,
    OpenAi,
}

fn detect_provider(url: &str) -> LlmProvider {
    let env_override = std::env::var("LLM_PROVIDER").unwrap_or_default();
    match env_override.to_lowercase().as_str() {
        "openai" => LlmProvider::OpenAi,
        "anthropic" => LlmProvider::Anthropic,
        _ => {
            if url.contains("openai.com") || url.contains("/chat/completions") {
                LlmProvider::OpenAi
            } else {
                LlmProvider::Anthropic
            }
        }
    }
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

pub async fn call_llm_api(prompt: &str, max_tokens: u32) -> Result<String> {
    let api_key = std::env::var("LLM_API_KEY").context("LLM_API_KEY not set")?;

    let api_url = std::env::var("LLM_API_URL")
        .unwrap_or_else(|_| "https://api.anthropic.com/v1/messages".to_string());

    let api_model =
        std::env::var("LLM_API_MODEL").unwrap_or_else(|_| "claude-4.5-sonnet".to_string());

    let provider = detect_provider(&api_url);

    let message = LlmMessage {
        role: "user".to_string(),
        content: prompt.to_string(),
    };

    let client = Client::new();

    let request = match provider {
        LlmProvider::Anthropic => LlmRequest::Anthropic {
            model: api_model,
            max_tokens,
            messages: vec![message],
        },
        LlmProvider::OpenAi => LlmRequest::OpenAi {
            model: api_model,
            max_tokens,
            messages: vec![message],
        },
    };

    let mut req = client
        .post(&api_url)
        .header("content-type", "application/json")
        .json(&request);

    match provider {
        LlmProvider::Anthropic => {
            req = req
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01");
        }
        LlmProvider::OpenAi => {
            req = req.header("authorization", format!("Bearer {}", api_key));
        }
    }

    let resp = req
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
}
