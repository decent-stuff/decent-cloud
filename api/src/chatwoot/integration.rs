//! Chatwoot integration hooks for provider and contract lifecycle events.

use super::ChatwootClient;
use crate::database::Database;
use anyhow::{Context, Result};

/// Create a Chatwoot agent for a provider (called on first offering creation).
pub async fn create_provider_agent(db: &Database, pubkey: &[u8]) -> Result<()> {
    let client = ChatwootClient::from_env()?;

    // Get provider's account info for name and email
    let account = db
        .get_account_with_keys_by_public_key(pubkey)
        .await
        .context("Failed to get provider account")?
        .context("Provider account not found")?;

    let name = account.display_name.as_deref().unwrap_or(&account.username);
    let email = account
        .email
        .as_ref()
        .context("Provider email required for Chatwoot agent")?;

    client
        .create_agent(email, name)
        .await
        .context("Failed to create Chatwoot agent")?;

    tracing::info!(
        "Created Chatwoot agent for provider {} (email: {})",
        hex::encode(pubkey),
        email
    );

    Ok(())
}

/// Create a Chatwoot conversation for a contract.
pub async fn create_contract_conversation(
    db: &Database,
    contract_id: &[u8],
    requester_pubkey: &[u8],
) -> Result<()> {
    let client = ChatwootClient::from_env()?;

    let inbox_id: u32 = std::env::var("CHATWOOT_INBOX_ID")
        .context("CHATWOOT_INBOX_ID not set")?
        .parse()
        .context("CHATWOOT_INBOX_ID must be a number")?;

    // Get requester's account info
    let account = db
        .get_account_with_keys_by_public_key(requester_pubkey)
        .await
        .context("Failed to get requester account")?
        .context("Requester account not found")?;

    let identifier = hex::encode(requester_pubkey);
    let name = account.display_name.as_deref().unwrap_or(&account.username);

    // Create or get contact
    let contact = client
        .create_contact(inbox_id, &identifier, name, account.email.as_deref())
        .await
        .context("Failed to create Chatwoot contact")?;

    // Create conversation
    let contract_id_hex = hex::encode(contract_id);
    client
        .create_conversation(inbox_id, contact.id, &contract_id_hex)
        .await
        .context("Failed to create Chatwoot conversation")?;

    tracing::info!(
        "Created Chatwoot conversation for contract {}",
        contract_id_hex
    );

    Ok(())
}

/// Check if Chatwoot integration is configured.
pub fn is_configured() -> bool {
    std::env::var("CHATWOOT_API_TOKEN").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_configured_when_missing() {
        std::env::remove_var("CHATWOOT_API_TOKEN");
        assert!(!is_configured());
    }

    #[test]
    fn test_is_configured_when_set() {
        std::env::set_var("CHATWOOT_API_TOKEN", "test_token");
        assert!(is_configured());
        std::env::remove_var("CHATWOOT_API_TOKEN");
    }
}
