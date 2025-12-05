//! AI Bot webhook handler for answering customer questions

use super::llm::{generate_answer, ArticleRef};
use super::search::search_articles;
use crate::chatwoot::ChatwootClient;
use crate::database::Database;
use anyhow::{Context, Result};

/// Format bot response message with answer and sources
pub fn format_bot_message(answer: &str, sources: &[ArticleRef]) -> String {
    let mut message = answer.to_string();

    if !sources.is_empty() {
        message.push_str("\n\n**Sources:**");
        for source in sources {
            message.push_str(&format!("\n- {}", source.title));
        }
    }

    message.push_str("\n\n*If you need to speak with a human, just say \"human\".*");
    message
}

/// Handle incoming customer message - search articles, generate answer, and respond or escalate
pub async fn handle_customer_message(
    db: &Database,
    chatwoot: &ChatwootClient,
    conversation_id: u64,
    contract_id: &str,
    message_content: &str,
) -> Result<()> {
    // 1. Get contract to find provider pubkey
    let contract_id_bytes = hex::decode(contract_id).context("Invalid contract_id hex")?;
    let contract = db
        .get_contract(&contract_id_bytes)
        .await?
        .context("Contract not found")?;

    let provider_pubkey_bytes =
        hex::decode(&contract.provider_pubkey).context("Invalid provider pubkey hex")?;

    // 2. Get provider notification config to find portal slug
    let notification_config = db
        .get_provider_notification_config(&provider_pubkey_bytes)
        .await?;

    let portal_slug = match notification_config {
        Some(config) => config.chatwoot_portal_slug,
        None => None,
    };

    let Some(portal_slug) = portal_slug else {
        tracing::warn!(
            "No portal slug configured for provider {}, escalating",
            contract.provider_pubkey
        );
        chatwoot
            .update_conversation_status(conversation_id, "open")
            .await?;
        return Ok(());
    };

    // 3. Fetch articles from Help Center
    let articles = chatwoot
        .fetch_help_center_articles(&portal_slug)
        .await
        .context("Failed to fetch help center articles")?;

    if articles.is_empty() {
        tracing::warn!("No articles found in portal {}, escalating", portal_slug);
        chatwoot
            .update_conversation_status(conversation_id, "open")
            .await?;
        return Ok(());
    }

    // 4. Search articles
    let scored_articles = search_articles(message_content, &articles);

    // 5. Generate answer
    let bot_response = generate_answer(message_content, &scored_articles).await?;

    // 6. Respond or escalate
    if bot_response.should_escalate {
        tracing::info!(
            "Bot escalating conversation {} (confidence: {:.2})",
            conversation_id,
            bot_response.confidence
        );
        chatwoot
            .update_conversation_status(conversation_id, "open")
            .await?;
    } else {
        let message = format_bot_message(&bot_response.answer, &bot_response.sources);
        chatwoot.send_message(conversation_id, &message).await?;
        tracing::info!(
            "Bot responded to conversation {} (confidence: {:.2})",
            conversation_id,
            bot_response.confidence
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bot_message_with_sources() {
        let answer = "To reset your password, go to Settings.";
        let sources = vec![
            ArticleRef {
                title: "Password Reset Guide".to_string(),
                slug: "password-reset".to_string(),
            },
            ArticleRef {
                title: "Account Security".to_string(),
                slug: "security".to_string(),
            },
        ];

        let message = format_bot_message(answer, &sources);

        assert!(message.contains(answer));
        assert!(message.contains("**Sources:**"));
        assert!(message.contains("- Password Reset Guide"));
        assert!(message.contains("- Account Security"));
        assert!(message.contains("If you need to speak with a human"));
    }

    #[test]
    fn test_format_bot_message_without_sources() {
        let answer = "I'm not sure about that.";
        let sources = vec![];

        let message = format_bot_message(answer, &sources);

        assert!(message.contains(answer));
        assert!(!message.contains("**Sources:**"));
        assert!(message.contains("If you need to speak with a human"));
    }

    #[test]
    fn test_format_bot_message_always_includes_human_hint() {
        let answer = "Here's some info.";
        let sources = vec![];

        let message = format_bot_message(answer, &sources);

        assert!(message.contains("human"));
    }
}
