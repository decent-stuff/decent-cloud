//! Embedding service for semantic article search.
//! Uses OpenAI's text-embedding-3-small model for generating embeddings.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Cache TTL in seconds (1 hour)
const CACHE_TTL_SECS: u64 = 3600;

/// Cached embedding with timestamp
struct CachedEmbedding {
    embedding: Vec<f32>,
    created_at: Instant,
}

/// In-memory embedding cache
static EMBEDDING_CACHE: RwLock<Option<HashMap<String, CachedEmbedding>>> = RwLock::new(None);

/// OpenAI embedding request
#[derive(Debug, Serialize)]
struct EmbeddingRequest<'a> {
    model: &'a str,
    input: &'a str,
}

/// OpenAI embedding response
#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

/// Check if embedding service is configured
pub fn is_configured() -> bool {
    std::env::var("OPENAI_API_KEY").is_ok()
}

/// Get embedding for text, using cache if available
pub async fn get_embedding(text: &str) -> Result<Vec<f32>> {
    let cache_key = text.to_string();

    // Check cache first
    {
        let cache = EMBEDDING_CACHE.read().unwrap();
        if let Some(ref map) = *cache {
            if let Some(cached) = map.get(&cache_key) {
                if cached.created_at.elapsed() < Duration::from_secs(CACHE_TTL_SECS) {
                    return Ok(cached.embedding.clone());
                }
            }
        }
    }

    // Fetch from API
    let embedding = fetch_embedding(text).await?;

    // Store in cache
    {
        let mut cache = EMBEDDING_CACHE.write().unwrap();
        if cache.is_none() {
            *cache = Some(HashMap::new());
        }
        if let Some(ref mut map) = *cache {
            map.insert(
                cache_key,
                CachedEmbedding {
                    embedding: embedding.clone(),
                    created_at: Instant::now(),
                },
            );
        }
    }

    Ok(embedding)
}

/// Fetch embedding from OpenAI API
async fn fetch_embedding(text: &str) -> Result<Vec<f32>> {
    let api_key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;

    let client = Client::new();
    let resp = client
        .post("https://api.openai.com/v1/embeddings")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&EmbeddingRequest {
            model: "text-embedding-3-small",
            input: text,
        })
        .send()
        .await
        .context("Failed to send embedding request")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("OpenAI API error {}: {}", status, body);
    }

    let response: EmbeddingResponse = resp.json().await.context("Failed to parse response")?;

    response
        .data
        .into_iter()
        .next()
        .map(|d| d.embedding)
        .context("No embedding in response")
}

/// Compute cosine similarity between two embeddings
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

/// Clear the embedding cache (for testing)
#[cfg(test)]
pub fn clear_cache() {
    let mut cache = EMBEDDING_CACHE.write().unwrap();
    *cache = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &b).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) + 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_normalized() {
        let a = vec![3.0, 4.0]; // norm = 5
        let b = vec![3.0, 4.0]; // norm = 5
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_is_configured() {
        let orig = std::env::var("OPENAI_API_KEY").ok();

        std::env::remove_var("OPENAI_API_KEY");
        assert!(!is_configured());

        std::env::set_var("OPENAI_API_KEY", "test_key");
        assert!(is_configured());

        if let Some(v) = orig {
            std::env::set_var("OPENAI_API_KEY", v);
        } else {
            std::env::remove_var("OPENAI_API_KEY");
        }
    }
}
