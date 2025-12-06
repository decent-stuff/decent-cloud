//! AI Bot webhook handler for answering customer questions

use super::llm::{generate_answer, ArticleRef};
use super::notifications::{dispatch_notification, SupportNotification};
use super::search::search_articles_semantic;
use crate::chatwoot::ChatwootClient;
use crate::database::Database;
use anyhow::Result;
use email_utils::EmailService;
use std::sync::Arc;

/// Contract info needed for bot handling
struct ContractInfo {
    provider_pubkey: Vec<u8>,
    portal_slug: Option<String>,
}

/// Get contract info (provider pubkey and portal slug) from contract_id
async fn get_contract_info(db: &Database, contract_id: &str) -> Result<Option<ContractInfo>> {
    let contract_id_bytes = match hex::decode(contract_id) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::warn!("Invalid contract_id hex '{}': {}", contract_id, e);
            return Ok(None);
        }
    };

    let contract = match db.get_contract(&contract_id_bytes).await? {
        Some(c) => c,
        None => {
            tracing::warn!("Contract not found: {}", contract_id);
            return Ok(None);
        }
    };

    let provider_pubkey_bytes = match hex::decode(&contract.provider_pubkey) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::warn!("Invalid provider pubkey hex: {}", e);
            return Ok(None);
        }
    };

    let notification_config = db
        .get_user_notification_config(&provider_pubkey_bytes)
        .await?;

    Ok(Some(ContractInfo {
        provider_pubkey: provider_pubkey_bytes,
        portal_slug: notification_config.and_then(|c| c.chatwoot_portal_slug),
    }))
}

/// Truncate a message to max_len characters, adding "..." if truncated
fn truncate_message(msg: &str, max_len: usize) -> String {
    if msg.len() <= max_len {
        msg.to_string()
    } else {
        format!("{}...", &msg[..max_len.saturating_sub(3)])
    }
}

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
    email_service: Option<&Arc<EmailService>>,
    conversation_id: u64,
    contract_id: Option<&str>,
    message_content: &str,
) -> Result<()> {
    tracing::debug!(
        "handle_customer_message: conversation={}, contract={:?}",
        conversation_id,
        contract_id
    );

    // Get contract info (provider pubkey and portal slug)
    let contract_info = match contract_id {
        Some(cid) => get_contract_info(db, cid).await?,
        None => None,
    };

    // Extract portal_slug, falling back to default
    let portal_slug = contract_info
        .as_ref()
        .and_then(|c| c.portal_slug.clone())
        .or_else(|| std::env::var("CHATWOOT_DEFAULT_PORTAL_SLUG").ok().filter(|s| !s.is_empty()));

    let portal_slug = match portal_slug {
        Some(slug) => {
            if contract_info.as_ref().and_then(|c| c.portal_slug.as_ref()).is_none() {
                tracing::debug!(
                    "Using default portal '{}' for conversation {}",
                    slug,
                    conversation_id
                );
            }
            slug
        }
        None => {
            tracing::warn!(
                "No portal slug available (contract: {:?}, no default configured), escalating conversation {}",
                contract_id,
                conversation_id
            );
            chatwoot
                .update_conversation_status(conversation_id, "open")
                .await?;
            return Ok(());
        }
    };

    tracing::debug!(
        "Fetching help center articles from portal '{}' for conversation {}",
        portal_slug,
        conversation_id
    );

    // 3. Fetch articles from Help Center
    let articles = match chatwoot.fetch_help_center_articles(&portal_slug).await {
        Ok(articles) => articles,
        Err(e) => {
            tracing::error!(
                "Failed to fetch help center articles for conversation {}: {}",
                conversation_id,
                e
            );
            chatwoot
                .send_message(
                    conversation_id,
                    "I'm experiencing technical difficulties. Let me connect you with a human agent.",
                )
                .await?;
            chatwoot
                .update_conversation_status(conversation_id, "open")
                .await?;
            return Ok(());
        }
    };

    if articles.is_empty() {
        tracing::warn!(
            "No articles found in portal '{}', escalating conversation {}",
            portal_slug,
            conversation_id
        );
        chatwoot
            .send_message(
                conversation_id,
                "I don't have enough information to help with that. Let me connect you with a human agent.",
            )
            .await?;
        chatwoot
            .update_conversation_status(conversation_id, "open")
            .await?;
        return Ok(());
    }

    tracing::debug!(
        "Found {} articles, searching for relevant content",
        articles.len()
    );

    // 4. Search articles (semantic if configured, else keyword)
    let scored_articles = search_articles_semantic(message_content, &articles).await;

    tracing::debug!(
        "Found {} relevant articles, generating AI response",
        scored_articles.len()
    );

    // 5. Generate answer
    let bot_response = generate_answer(message_content, &scored_articles).await?;

    // 6. Respond or escalate
    if bot_response.should_escalate {
        tracing::info!(
            "Bot escalating conversation {} (confidence: {:.2})",
            conversation_id,
            bot_response.confidence
        );
        // Send escalation message if there's one
        if !bot_response.answer.is_empty() {
            chatwoot
                .send_message(conversation_id, &bot_response.answer)
                .await?;
        }
        chatwoot
            .update_conversation_status(conversation_id, "open")
            .await?;

        // Notify provider about escalation
        if let Some(info) = &contract_info {
            let chatwoot_base_url =
                std::env::var("CHATWOOT_BASE_URL").unwrap_or_else(|_| "https://chat.example.com".to_string());
            let notification = SupportNotification::new(
                info.provider_pubkey.clone(),
                conversation_id as i64,
                contract_id.unwrap_or("unknown").to_string(),
                format!("Customer needs assistance: {}", truncate_message(message_content, 100)),
                &chatwoot_base_url,
            );
            if let Err(e) = dispatch_notification(db, email_service, &notification).await {
                tracing::error!("Failed to dispatch escalation notification: {}", e);
            }
        }
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

    #[test]
    fn test_truncate_message_short() {
        assert_eq!(truncate_message("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_message_exact() {
        assert_eq!(truncate_message("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_message_long() {
        assert_eq!(truncate_message("hello world", 8), "hello...");
    }
}
