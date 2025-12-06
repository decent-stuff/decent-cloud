use crate::chatwoot::{ChatwootClient, HelpCenterArticle};
use anyhow::{Context, Result};
use std::collections::HashMap;

/// Documentation file to sync to Help Center.
struct DocFile {
    path: &'static str,
    slug: &'static str,
    title: &'static str,
}

/// Documentation files to sync to Chatwoot Help Center.
const DOCS_TO_SYNC: &[DocFile] = &[
    DocFile {
        path: "docs/getting-started.md",
        slug: "getting-started",
        title: "Getting Started with Decent Cloud",
    },
    DocFile {
        path: "docs/user-guide.md",
        slug: "user-guide",
        title: "User Guide",
    },
    DocFile {
        path: "docs/installation.md",
        slug: "installation",
        title: "Installation Guide",
    },
    DocFile {
        path: "docs/reputation.md",
        slug: "reputation",
        title: "Reputation System",
    },
    DocFile {
        path: "docs/token-distribution.md",
        slug: "token-distribution",
        title: "Token Distribution",
    },
    DocFile {
        path: "docs/mining-and-validation.md",
        slug: "mining-validation",
        title: "Mining and Validation Guide",
    },
    DocFile {
        path: "docs/faq-general.md",
        slug: "faq-general",
        title: "FAQ - General",
    },
    DocFile {
        path: "docs/faq-technical.md",
        slug: "faq-technical",
        title: "FAQ - Technical & Security",
    },
];

/// Sync documentation files to Chatwoot Help Center.
pub async fn sync_docs(portal_slug: &str, dry_run: bool) -> Result<()> {
    let chatwoot = ChatwootClient::from_env()?;

    println!("Syncing documentation to portal '{}'...", portal_slug);

    // Get author_id from current user's profile (required for creating articles)
    let author_id = chatwoot
        .get_profile()
        .await
        .context("Failed to get profile for author_id")?;
    println!("Using author_id: {}", author_id);

    // Get existing articles for idempotency
    let existing = chatwoot.list_articles(portal_slug).await?;
    let existing_by_slug: HashMap<&str, &HelpCenterArticle> =
        existing.iter().map(|a| (a.slug.as_str(), a)).collect();

    println!("Found {} existing articles in portal", existing.len());

    for doc in DOCS_TO_SYNC {
        // Read file content
        let content = std::fs::read_to_string(doc.path)
            .with_context(|| format!("Failed to read {}", doc.path))?;

        // Extract description from first paragraph
        let description = extract_first_paragraph(&content);

        // Strip markdown badges
        let cleaned_content = strip_markdown_badges(&content);

        if dry_run {
            if existing_by_slug.contains_key(doc.slug) {
                println!("[DRY RUN] Would UPDATE: {} -> {}", doc.path, doc.slug);
            } else {
                println!("[DRY RUN] Would CREATE: {} -> {}", doc.path, doc.slug);
            }
            continue;
        }

        // Create or update article
        if let Some(existing_article) = existing_by_slug.get(doc.slug) {
            chatwoot
                .update_article(
                    portal_slug,
                    existing_article.id,
                    doc.title,
                    &cleaned_content,
                    &description,
                )
                .await
                .with_context(|| format!("Failed to update article {}", doc.slug))?;
            println!("Updated: {} (id={})", doc.slug, existing_article.id);
        } else {
            let article_id = chatwoot
                .create_article(
                    portal_slug,
                    doc.title,
                    doc.slug,
                    &cleaned_content,
                    &description,
                    author_id,
                )
                .await
                .with_context(|| {
                    format!(
                        "Failed to create article {} (content size: {} bytes)",
                        doc.slug,
                        cleaned_content.len()
                    )
                })?;
            println!("Created: {} (id={})", doc.slug, article_id);
        }
    }

    if dry_run {
        println!("\nDry run complete. No changes were made.");
    } else {
        println!(
            "\nSync complete! {} documents processed.",
            DOCS_TO_SYNC.len()
        );
    }

    Ok(())
}

/// Extract the first paragraph from markdown content for use as description.
/// Skips title lines and returns up to 200 characters.
fn extract_first_paragraph(content: &str) -> String {
    content
        .lines()
        .skip_while(|l| l.starts_with('#') || l.trim().is_empty())
        .take_while(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(200)
        .collect()
}

/// Strip markdown badge images (shields.io) from content.
fn strip_markdown_badges(content: &str) -> String {
    content
        .lines()
        .filter(|l| !l.contains("img.shields.io") && !l.contains("shields.io"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_first_paragraph() {
        let content = r#"# Title

This is the first paragraph.
It has multiple lines.

This is the second paragraph."#;

        let result = extract_first_paragraph(content);
        assert_eq!(
            result,
            "This is the first paragraph. It has multiple lines."
        );
    }

    #[test]
    fn test_extract_first_paragraph_with_multiple_hashes() {
        let content = r#"# Main Title
## Subtitle

First paragraph here."#;

        let result = extract_first_paragraph(content);
        assert_eq!(result, "First paragraph here.");
    }

    #[test]
    fn test_extract_first_paragraph_truncates_at_200_chars() {
        let long_text = "a".repeat(300);
        let content = format!("# Title\n\n{}", long_text);

        let result = extract_first_paragraph(&content);
        assert_eq!(result.len(), 200);
    }

    #[test]
    fn test_strip_markdown_badges() {
        let content = r#"# Title

![Build Status](https://img.shields.io/badge/build-passing-green)
![Coverage](https://shields.io/coverage/90)

Some actual content here."#;

        let result = strip_markdown_badges(content);
        assert!(!result.contains("img.shields.io"));
        assert!(!result.contains("shields.io"));
        assert!(result.contains("Some actual content here."));
    }

    #[test]
    fn test_strip_markdown_badges_preserves_other_content() {
        let content = r#"Line 1
![Badge](https://img.shields.io/test)
Line 2
Line 3"#;

        let result = strip_markdown_badges(content);
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "Line 1");
        assert_eq!(lines[1], "Line 2");
        assert_eq!(lines[2], "Line 3");
    }
}
