//! Simple keyword-based article search using basic tokenization and weighted scoring

const MIN_SCORE_THRESHOLD: f32 = 0.1;
const TITLE_WEIGHT: f32 = 2.0;
const CONTENT_WEIGHT: f32 = 1.0;

#[derive(Debug, Clone)]
pub struct HelpCenterArticle {
    pub id: u64,
    pub title: String,
    pub content: String,
    pub slug: String,
}

#[derive(Debug, Clone)]
pub struct ScoredArticle {
    pub article: HelpCenterArticle,
    pub score: f32,
}

/// Search articles by keyword matching
/// Returns articles sorted by relevance score (highest first), filtered by threshold
pub fn search_articles(query: &str, articles: &[HelpCenterArticle]) -> Vec<ScoredArticle> {
    if query.trim().is_empty() || articles.is_empty() {
        return Vec::new();
    }

    let keywords = tokenize(query);
    if keywords.is_empty() {
        return Vec::new();
    }

    let mut scored: Vec<ScoredArticle> = articles
        .iter()
        .map(|article| {
            let score = calculate_score(&keywords, article);
            ScoredArticle {
                article: article.clone(),
                score,
            }
        })
        .filter(|scored| scored.score >= MIN_SCORE_THRESHOLD)
        .collect();

    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored
}

/// Tokenize query into lowercase keywords
fn tokenize(query: &str) -> Vec<String> {
    query
        .to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

/// Calculate relevance score for an article
fn calculate_score(keywords: &[String], article: &HelpCenterArticle) -> f32 {
    let title_lower = article.title.to_lowercase();
    let content_lower = article.content.to_lowercase();

    let mut title_matches = 0;
    let mut content_matches = 0;

    for keyword in keywords {
        if title_lower.contains(keyword) {
            title_matches += 1;
        }
        if content_lower.contains(keyword) {
            content_matches += 1;
        }
    }

    let weighted_score =
        (title_matches as f32 * TITLE_WEIGHT) + (content_matches as f32 * CONTENT_WEIGHT);
    let max_possible = keywords.len() as f32 * (TITLE_WEIGHT + CONTENT_WEIGHT);

    if max_possible > 0.0 {
        weighted_score / max_possible
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_article(id: u64, title: &str, content: &str) -> HelpCenterArticle {
        HelpCenterArticle {
            id,
            title: title.to_string(),
            content: content.to_string(),
            slug: format!("article-{}", id),
        }
    }

    #[test]
    fn test_search_empty_query() {
        let articles = vec![create_article(1, "Test", "Content")];
        let results = search_articles("", &articles);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_whitespace_query() {
        let articles = vec![create_article(1, "Test", "Content")];
        let results = search_articles("   ", &articles);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_empty_articles() {
        let articles = vec![];
        let results = search_articles("test", &articles);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_no_matches() {
        let articles = vec![
            create_article(1, "Bitcoin Wallet", "How to create a Bitcoin wallet"),
            create_article(2, "Ethereum Guide", "Guide to Ethereum"),
        ];
        let results = search_articles("completely unrelated query", &articles);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_single_match() {
        let articles = vec![
            create_article(1, "Bitcoin Wallet", "How to create a Bitcoin wallet"),
            create_article(2, "Ethereum Guide", "Guide to Ethereum"),
        ];
        let results = search_articles("bitcoin", &articles);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].article.id, 1);
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn test_search_title_weight_higher_than_content() {
        let articles = vec![
            create_article(1, "Wallet Guide", "How to use bitcoin in your transactions"),
            create_article(
                2,
                "Other Topic",
                "This is about wallets and how to use them",
            ),
        ];
        let results = search_articles("wallet", &articles);
        assert_eq!(results.len(), 2);
        // Article 1 should rank higher because "wallet" is in the title
        assert_eq!(results[0].article.id, 1);
        assert!(results[0].score > results[1].score);
    }

    #[test]
    fn test_search_multiple_keywords_ranked_correctly() {
        let articles = vec![
            create_article(
                1,
                "Bitcoin Wallet Setup",
                "Complete guide to Bitcoin wallet setup and security",
            ),
            create_article(2, "Bitcoin Guide", "Introduction to Bitcoin"),
            create_article(
                3,
                "Wallet Security",
                "How to secure any cryptocurrency wallet",
            ),
        ];
        let results = search_articles("bitcoin wallet", &articles);

        assert!(results.len() >= 2);
        // Article 1 should rank highest (both keywords in title)
        assert_eq!(results[0].article.id, 1);
        // Verify it has highest score
        assert!(results[0].score > results[1].score);
    }

    #[test]
    fn test_search_case_insensitive() {
        let articles = vec![create_article(
            1,
            "Bitcoin Wallet",
            "Guide to BITCOIN wallets",
        )];
        let results = search_articles("BITCOIN wallet", &articles);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].article.id, 1);
    }

    #[test]
    fn test_tokenize() {
        assert_eq!(tokenize("hello world"), vec!["hello", "world"]);
        assert_eq!(tokenize("  spaced  out  "), vec!["spaced", "out"]);
        assert_eq!(tokenize("MiXeD CaSe"), vec!["mixed", "case"]);
        assert_eq!(tokenize(""), Vec::<String>::new());
    }

    #[test]
    fn test_score_normalization() {
        let articles = vec![create_article(1, "Test Title", "Test content")];
        let results = search_articles("test", &articles);
        assert_eq!(results.len(), 1);
        // Score should be normalized between 0.0 and 1.0
        assert!(results[0].score > 0.0);
        assert!(results[0].score <= 1.0);
    }
}
