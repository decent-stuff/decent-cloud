use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::search::ScoredArticle;

const MAX_ARTICLES_IN_PROMPT: usize = 3;
const LOW_CONFIDENCE_THRESHOLD: f32 = 0.5;
const SUMMARIZE_MAX_TOKENS: u32 = 256;

/// Common greeting patterns that the bot can handle directly
const GREETING_PATTERNS: &[&str] = &[
    "hello",
    "hi",
    "hey",
    "greetings",
    "good morning",
    "good afternoon",
    "good evening",
    "howdy",
    "hiya",
    "yo",
    "sup",
    "what's up",
    "whats up",
];

/// Patterns indicating thanks (can respond and close or continue)
const THANKS_PATTERNS: &[&str] = &["thank", "thanks", "thx", "ty", "appreciate", "grateful"];

/// Check if message is a simple greeting
fn is_greeting(message: &str) -> bool {
    let lower = message.to_lowercase();
    let cleaned: String = lower
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect();
    let trimmed = cleaned.trim();

    // Only match if it's a short message that IS the greeting (not just contains it)
    if trimmed.len() > 30 {
        return false;
    }

    // Check if it's exactly a greeting or starts with one
    GREETING_PATTERNS
        .iter()
        .any(|g| trimmed == *g || trimmed.starts_with(&format!("{} ", g)))
}

/// Check if message is expressing thanks
fn is_thanks(message: &str) -> bool {
    let lower = message.to_lowercase();
    THANKS_PATTERNS.iter().any(|t| lower.contains(t))
}

#[derive(Debug, Clone)]
pub struct BotResponse {
    pub answer: String,
    pub sources: Vec<ArticleRef>,
    pub confidence: f32,
    pub should_escalate: bool,
}

#[derive(Debug, Clone)]
pub struct ArticleRef {
    pub title: String,
    pub slug: String,
}

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

/// Generate an answer to a question using LLM and relevant articles
pub async fn generate_answer(question: &str, articles: &[ScoredArticle]) -> Result<BotResponse> {
    // Check for explicit escalation keywords
    let question_lower = question.to_lowercase();
    if question_lower.contains("human") || question_lower.contains("agent") {
        return Ok(BotResponse {
            answer: "I'll connect you with a human agent.".to_string(),
            sources: vec![],
            confidence: 0.0,
            should_escalate: true,
        });
    }

    // Handle simple greetings without needing knowledge base
    if is_greeting(question) {
        return Ok(BotResponse {
            answer: "Hello! I'm here to help. What can I assist you with today?".to_string(),
            sources: vec![],
            confidence: 1.0,
            should_escalate: false,
        });
    }

    // Handle thanks
    if is_thanks(question) && articles.is_empty() {
        return Ok(BotResponse {
            answer: "You're welcome! Is there anything else I can help you with?".to_string(),
            sources: vec![],
            confidence: 1.0,
            should_escalate: false,
        });
    }

    // If no articles and not a conversational message, escalate
    if articles.is_empty() {
        return Ok(BotResponse {
            answer: "I couldn't find relevant information. Let me connect you with a human agent."
                .to_string(),
            sources: vec![],
            confidence: 0.0,
            should_escalate: true,
        });
    }

    // Build prompt with top articles
    let prompt = build_prompt(question, articles);

    // Call LLM API
    let answer = match call_llm_api(&prompt, 1024).await {
        Ok(text) => text,
        Err(e) => {
            tracing::error!("LLM API error: {:#}", e);
            return Ok(BotResponse {
                answer: "I'm having trouble processing your question. Let me connect you with a human agent.".to_string(),
                sources: vec![],
                confidence: 0.0,
                should_escalate: true,
            });
        }
    };

    // Calculate confidence based on article scores
    let top_articles: Vec<_> = articles.iter().take(MAX_ARTICLES_IN_PROMPT).collect();

    let avg_score = top_articles.iter().map(|a| a.score).sum::<f32>() / top_articles.len() as f32;

    let confidence = avg_score.min(1.0);

    // Build sources
    let sources: Vec<ArticleRef> = top_articles
        .iter()
        .map(|a| ArticleRef {
            title: a.article.title.clone(),
            slug: a.article.slug.clone(),
        })
        .collect();

    Ok(BotResponse {
        answer,
        sources,
        confidence,
        should_escalate: confidence < LOW_CONFIDENCE_THRESHOLD,
    })
}

/// Generate a concise summary of the conversation for human agent escalation.
/// Returns a brief summary of the customer's request and key context.
pub async fn summarize_for_escalation(
    conversation_history: &[(String, String)], // (role, content) pairs
    escalation_reason: &str,
) -> Result<String> {
    if conversation_history.is_empty() {
        return Ok(format!("Escalation reason: {}", escalation_reason));
    }

    let mut prompt = String::from(
        "Summarize this customer support conversation for a human agent in 2-3 sentences. \
        Focus on: what the customer needs, any relevant context, and why they're being escalated.\n\n",
    );

    prompt.push_str("Conversation:\n");
    for (role, content) in conversation_history {
        let label = if role == "customer" {
            "Customer"
        } else {
            "Bot"
        };
        prompt.push_str(&format!("{}: {}\n", label, content));
    }

    prompt.push_str(&format!(
        "\nEscalation reason: {}\n\nSummary:",
        escalation_reason
    ));

    match call_llm_api(&prompt, SUMMARIZE_MAX_TOKENS).await {
        Ok(summary) => Ok(summary.trim().to_string()),
        Err(e) => {
            tracing::warn!("Failed to generate escalation summary: {:#}", e);
            // Fallback: use last customer message
            let last_customer_msg = conversation_history
                .iter()
                .rev()
                .find(|(role, _)| role == "customer")
                .map(|(_, content)| content.as_str())
                .unwrap_or("No message available");
            Ok(format!(
                "Customer request: {}. Escalation reason: {}",
                truncate(last_customer_msg, 150),
                escalation_reason
            ))
        }
    }
}

fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}

fn build_prompt(question: &str, articles: &[ScoredArticle]) -> String {
    let mut prompt = String::from("You are a helpful customer support assistant. Answer the user's question based on the following knowledge base articles. Be concise and helpful.\n\n");

    prompt.push_str("Knowledge Base:\n");
    for (i, scored) in articles.iter().take(MAX_ARTICLES_IN_PROMPT).enumerate() {
        prompt.push_str(&format!(
            "\n[Article {}] {}\n{}\n",
            i + 1,
            scored.article.title,
            scored.article.content
        ));
    }

    prompt.push_str(&format!(
        "\n\nUser Question: {}\n\nPlease provide a helpful answer:",
        question
    ));
    prompt
}

async fn call_llm_api(prompt: &str, max_tokens: u32) -> Result<String> {
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
        let body = resp.text().await.unwrap_or_default();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chatwoot::HelpCenterArticle;

    fn make_test_article(
        id: i64,
        title: &str,
        content: &str,
        slug: &str,
        score: f32,
    ) -> ScoredArticle {
        ScoredArticle {
            article: HelpCenterArticle {
                id,
                title: title.to_string(),
                content: content.to_string(),
                slug: slug.to_string(),
            },
            score,
        }
    }

    #[tokio::test]
    async fn test_escalate_on_human_keyword() {
        let articles = vec![make_test_article(1, "FAQ", "Some content", "faq", 0.8)];

        let response = generate_answer("I want to talk to a human", &articles)
            .await
            .unwrap();

        assert!(response.should_escalate);
        assert_eq!(response.confidence, 0.0);
    }

    #[tokio::test]
    async fn test_escalate_on_agent_keyword() {
        let articles = vec![make_test_article(1, "FAQ", "Some content", "faq", 0.8)];

        let response = generate_answer("Connect me with an agent", &articles)
            .await
            .unwrap();

        assert!(response.should_escalate);
        assert_eq!(response.confidence, 0.0);
    }

    #[tokio::test]
    async fn test_escalate_on_no_articles() {
        let articles = vec![];

        let response = generate_answer("How do I reset my password?", &articles)
            .await
            .unwrap();

        assert!(response.should_escalate);
        assert_eq!(response.confidence, 0.0);
        assert!(response.sources.is_empty());
    }

    #[tokio::test]
    async fn test_confidence_calculation() {
        let articles = vec![
            make_test_article(1, "Article 1", "Content 1", "article-1", 0.9),
            make_test_article(2, "Article 2", "Content 2", "article-2", 0.7),
            make_test_article(3, "Article 3", "Content 3", "article-3", 0.5),
        ];

        // Mock LLM by setting invalid API key - will trigger error path
        std::env::remove_var("LLM_API_KEY");

        let response = generate_answer("Test question", &articles).await.unwrap();

        // Should escalate due to API error
        assert!(response.should_escalate);
    }

    #[tokio::test]
    async fn test_low_confidence_triggers_escalation() {
        let articles = vec![
            make_test_article(1, "Article 1", "Content 1", "article-1", 0.3),
            make_test_article(2, "Article 2", "Content 2", "article-2", 0.2),
        ];

        std::env::remove_var("LLM_API_KEY");

        let response = generate_answer("Test question", &articles).await.unwrap();

        assert!(response.should_escalate);
    }

    #[test]
    fn test_build_prompt() {
        let articles = vec![
            make_test_article(1, "Title 1", "Content 1", "slug-1", 0.9),
            make_test_article(2, "Title 2", "Content 2", "slug-2", 0.8),
        ];

        let prompt = build_prompt("What is this?", &articles);

        assert!(prompt.contains("Title 1"));
        assert!(prompt.contains("Content 1"));
        assert!(prompt.contains("Title 2"));
        assert!(prompt.contains("Content 2"));
        assert!(prompt.contains("What is this?"));
    }

    #[test]
    fn test_prompt_limits_articles() {
        let articles: Vec<_> = (1..=10)
            .map(|i| make_test_article(i, &format!("Title {}", i), "Content", "slug", 0.9))
            .collect();

        let prompt = build_prompt("Question", &articles);

        // Should only include MAX_ARTICLES_IN_PROMPT (3)
        assert!(prompt.contains("Title 1"));
        assert!(prompt.contains("Title 2"));
        assert!(prompt.contains("Title 3"));
        assert!(!prompt.contains("Title 4"));
    }

    #[test]
    fn test_is_greeting_simple() {
        assert!(is_greeting("hello"));
        assert!(is_greeting("Hello!"));
        assert!(is_greeting("hi"));
        assert!(is_greeting("hey"));
        assert!(is_greeting("hello..."));
        assert!(is_greeting("Hi there"));
        assert!(is_greeting("good morning"));
    }

    #[test]
    fn test_is_greeting_not_greeting() {
        assert!(!is_greeting("How do I reset my password?"));
        assert!(!is_greeting("I need help with billing"));
        assert!(!is_greeting("What are your hours?"));
    }

    #[test]
    fn test_is_thanks() {
        assert!(is_thanks("thank you"));
        assert!(is_thanks("Thanks!"));
        assert!(is_thanks("thx"));
        assert!(is_thanks("I appreciate your help"));
    }

    #[tokio::test]
    async fn test_greeting_responds_without_escalating() {
        let articles = vec![];

        let response = generate_answer("hello", &articles).await.unwrap();

        assert!(!response.should_escalate);
        assert_eq!(response.confidence, 1.0);
        assert!(response.answer.contains("Hello"));
    }

    #[tokio::test]
    async fn test_greeting_with_punctuation() {
        let articles = vec![];

        let response = generate_answer("hello...", &articles).await.unwrap();

        assert!(!response.should_escalate);
        assert_eq!(response.confidence, 1.0);
    }

    #[tokio::test]
    async fn test_thanks_responds_without_escalating() {
        let articles = vec![];

        let response = generate_answer("thank you!", &articles).await.unwrap();

        assert!(!response.should_escalate);
        assert_eq!(response.confidence, 1.0);
        assert!(response.answer.contains("welcome"));
    }

    #[tokio::test]
    async fn test_summarize_empty_history() {
        let history: Vec<(String, String)> = vec![];
        let result = summarize_for_escalation(&history, "test reason")
            .await
            .unwrap();
        assert_eq!(result, "Escalation reason: test reason");
    }

    #[tokio::test]
    async fn test_summarize_fallback_on_api_error() {
        // No LLM_API_KEY set, so API call will fail and fallback will be used
        std::env::remove_var("LLM_API_KEY");

        let history = vec![
            ("customer".to_string(), "Hello, I need help".to_string()),
            ("bot".to_string(), "Hi! How can I help?".to_string()),
            ("customer".to_string(), "My account is locked".to_string()),
        ];

        let result = summarize_for_escalation(&history, "Low confidence")
            .await
            .unwrap();

        // Should use fallback format with last customer message
        assert!(result.contains("My account is locked"));
        assert!(result.contains("Low confidence"));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello");
        assert_eq!(truncate("", 5), "");
    }
}
