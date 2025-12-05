use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::search::ScoredArticle;

const MAX_ARTICLES_IN_PROMPT: usize = 3;
const LOW_CONFIDENCE_THRESHOLD: f32 = 0.5;

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

    // If no articles, escalate
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
    let answer = match call_llm_api(&prompt).await {
        Ok(text) => text,
        Err(e) => {
            tracing::error!("LLM API error: {}", e);
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

async fn call_llm_api(prompt: &str) -> Result<String> {
    let api_key = std::env::var("LLM_API_KEY").context("LLM_API_KEY not set")?;

    let api_url = std::env::var("LLM_API_URL")
        .unwrap_or_else(|_| "https://api.anthropic.com/v1/messages".to_string());

    let client = Client::new();

    let request = ClaudeRequest {
        model: "claude-3-5-sonnet-20241022".to_string(),
        max_tokens: 1024,
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

    let response: ClaudeResponse = resp
        .json()
        .await
        .context("Failed to parse LLM API response")?;

    response
        .content
        .first()
        .map(|c| c.text.clone())
        .context("LLM response contained no content")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support_bot::search::HelpCenterArticle;

    fn make_test_article(
        id: u64,
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
}
