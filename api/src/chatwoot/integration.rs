//! Chatwoot integration hooks for provider and contract lifecycle events.

use super::{ChatwootClient, ChatwootPlatformClient};
use crate::database::Database;
use anyhow::{Context, Result};
use rand::Rng;

/// Generate a secure random password meeting Chatwoot requirements.
/// Must contain: uppercase, lowercase, number, and special character.
pub fn generate_secure_password() -> String {
    let mut rng = rand::thread_rng();

    // Character sets
    let uppercase = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let lowercase = b"abcdefghijklmnopqrstuvwxyz";
    let numbers = b"0123456789";
    let special = b"!@#$%^&*";

    // Ensure at least one of each required type
    let mut password = vec![
        uppercase[rng.gen_range(0..uppercase.len())] as char,
        lowercase[rng.gen_range(0..lowercase.len())] as char,
        numbers[rng.gen_range(0..numbers.len())] as char,
        special[rng.gen_range(0..special.len())] as char,
    ];

    // Fill remaining 12 characters with random mix
    let all_chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*";
    for _ in 0..12 {
        password.push(all_chars[rng.gen_range(0..all_chars.len())] as char);
    }

    // Shuffle to avoid predictable pattern
    use rand::seq::SliceRandom;
    password.shuffle(&mut rng);

    password.into_iter().collect()
}

/// Create a Chatwoot agent for a provider using Platform API.
/// Stores user_id in database for future password resets.
pub async fn create_provider_agent(db: &Database, pubkey: &[u8]) -> Result<()> {
    let client = ChatwootPlatformClient::from_env()?;

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

    // Generate initial password (user will reset via email)
    let initial_password = generate_secure_password();

    // Create user via Platform API
    let user = client
        .create_user(email, name, &initial_password)
        .await
        .context("Failed to create Chatwoot user")?;

    // Add user to account as agent
    client
        .add_user_to_account(user.id)
        .await
        .context("Failed to add user to Chatwoot account")?;

    // Store user_id in database for future password resets
    let account_id_bytes = hex::decode(&account.id).context("Invalid account ID")?;
    db.set_chatwoot_user_id(&account_id_bytes, user.id)
        .await
        .context("Failed to store Chatwoot user ID")?;

    tracing::info!(
        "Created Chatwoot agent for provider {} (email: {}, user_id: {})",
        hex::encode(pubkey),
        email,
        user.id
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

/// Check if Chatwoot Platform API integration is configured.
pub fn is_configured() -> bool {
    ChatwootPlatformClient::is_configured()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_configured_when_missing() {
        std::env::remove_var("CHATWOOT_PLATFORM_API_TOKEN");
        assert!(!is_configured());
    }

    #[test]
    fn test_is_configured_when_set() {
        std::env::set_var("CHATWOOT_PLATFORM_API_TOKEN", "test_token");
        std::env::set_var("CHATWOOT_BASE_URL", "https://test.chatwoot.com");
        std::env::set_var("CHATWOOT_ACCOUNT_ID", "1");
        assert!(is_configured());
        std::env::remove_var("CHATWOOT_PLATFORM_API_TOKEN");
        std::env::remove_var("CHATWOOT_BASE_URL");
        std::env::remove_var("CHATWOOT_ACCOUNT_ID");
    }

    #[test]
    fn test_generate_secure_password_meets_requirements() {
        let password = generate_secure_password();

        // Check length (16 chars)
        assert_eq!(password.len(), 16);

        // Check contains uppercase
        assert!(password.chars().any(|c| c.is_ascii_uppercase()));

        // Check contains lowercase
        assert!(password.chars().any(|c| c.is_ascii_lowercase()));

        // Check contains digit
        assert!(password.chars().any(|c| c.is_ascii_digit()));

        // Check contains special char
        assert!(password.chars().any(|c| "!@#$%^&*".contains(c)));
    }

    #[test]
    fn test_generate_secure_password_is_random() {
        let p1 = generate_secure_password();
        let p2 = generate_secure_password();
        assert_ne!(p1, p2);
    }
}
