//! Chatwoot integration hooks for provider and contract lifecycle events.

use super::ChatwootClient;
use super::ChatwootPlatformClient;
use crate::database::email::EmailType;
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

/// Generate a URL-safe slug from a provider name.
fn generate_portal_slug(name: &str, pubkey: &[u8]) -> String {
    // Take first 8 chars of pubkey hex for uniqueness
    let pubkey_suffix = hex::encode(&pubkey[..4]);
    let base = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>();
    // Remove consecutive dashes and trim
    let mut prev_dash = false;
    let cleaned: String = base
        .chars()
        .filter(|&c| {
            if c == '-' {
                if prev_dash {
                    return false;
                }
                prev_dash = true;
            } else {
                prev_dash = false;
            }
            true
        })
        .collect();
    let trimmed = cleaned.trim_matches('-');
    // Limit length and append pubkey suffix
    let max_base = 40;
    let truncated = if trimmed.len() > max_base {
        &trimmed[..max_base]
    } else {
        trimmed
    };
    format!("{}-{}", truncated, pubkey_suffix)
}

/// Create or link a Chatwoot agent for a provider using Platform API.
/// Also creates dedicated inbox, team, and Help Center portal.
/// If user exists in Chatwoot, links them. If not, creates new user.
/// Stores user_id in database for future password resets.
/// Returns the generated password for display to the user.
pub async fn create_provider_agent(db: &Database, pubkey: &[u8]) -> Result<String> {
    let platform_client = ChatwootPlatformClient::from_env()?;
    let account_client = ChatwootClient::from_env()?;
    let api_token = std::env::var("CHATWOOT_API_TOKEN").context("CHATWOOT_API_TOKEN not set")?;

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

    // Ensure Support Agent custom role exists (has Help Center access)
    let custom_role_id = platform_client
        .ensure_support_agent_role(&api_token)
        .await
        .context("Failed to ensure Support Agent role exists")?;

    // Generate password for the user
    let password = generate_secure_password();

    // Create or find user via Platform API (Chatwoot handles find-or-create internally)
    let user = platform_client
        .create_user(email, name, &password)
        .await
        .context("Failed to create Chatwoot user")?;

    // Always update password to ensure it's set (create doesn't update existing user's password)
    platform_client
        .update_user_password(user.id, &password)
        .await
        .context("Failed to set Chatwoot user password")?;

    // Add user to account with Support Agent role (ignore error if already added)
    if let Err(e) = platform_client
        .add_user_to_account(user.id, custom_role_id)
        .await
    {
        // 422 typically means user is already in account
        let err_str = format!("{:#}", e);
        if !err_str.contains("422") {
            return Err(e).context("Failed to add user to Chatwoot account");
        }
        tracing::info!("User {} already in Chatwoot account", email);
    }

    // Store user_id in database for future password resets
    let account_id_bytes = hex::decode(&account.id).context("Invalid account ID")?;
    db.set_chatwoot_user_id(&account_id_bytes, user.id)
        .await
        .context("Failed to store Chatwoot user ID")?;

    // Create dedicated inbox for the provider
    let inbox_name = format!("{} Support", name);
    let inbox = account_client
        .create_inbox(&inbox_name)
        .await
        .context("Failed to create Chatwoot inbox for provider")?;
    tracing::info!(
        "Created Chatwoot inbox '{}' (id={}) for provider {}",
        inbox.name,
        inbox.id,
        hex::encode(pubkey)
    );

    // Create dedicated team for the provider
    let team_name = format!("{} Team", name);
    let team_desc = format!("Support team for {}", name);
    let team = account_client
        .create_team(&team_name, &team_desc)
        .await
        .context("Failed to create Chatwoot team for provider")?;
    tracing::info!(
        "Created Chatwoot team '{}' (id={}) for provider {}",
        team.name,
        team.id,
        hex::encode(pubkey)
    );

    // Add the provider agent to their team
    account_client
        .add_agents_to_team(team.id, &[user.id])
        .await
        .context("Failed to add provider agent to team")?;

    // Create dedicated Help Center portal
    let portal_slug = generate_portal_slug(name, pubkey);
    let portal_name = format!("{} Help Center", name);
    let portal = account_client
        .create_portal(&portal_name, &portal_slug)
        .await
        .context("Failed to create Chatwoot portal")?;
    tracing::info!(
        "Created Chatwoot portal '{}' (slug={}) for provider {}",
        portal.name,
        portal.slug,
        hex::encode(pubkey)
    );

    // Store Chatwoot resource IDs in database
    db.set_provider_chatwoot_resources(pubkey, inbox.id, team.id, &portal.slug)
        .await
        .context("Failed to store Chatwoot resource IDs")?;

    tracing::info!(
        "Completed Chatwoot onboarding for provider {} (email: {}, user_id: {}, inbox: {}, team: {}, portal: {})",
        hex::encode(pubkey),
        email,
        user.id,
        inbox.id,
        team.id,
        portal.slug
    );

    // Send welcome email
    let support_url = std::env::var("CHATWOOT_FRONTEND_URL")
        .unwrap_or_else(|_| "https://support.decent-cloud.org".to_string());
    db.queue_email_safe(
        Some(email),
        "noreply@decent-cloud.org",
        "Your Provider Support Portal is Ready",
        &format!(
            r#"Hello {name},

Your Decent Cloud provider support portal has been created with dedicated resources:

- A private inbox for customer support tickets
- A team workspace for your support agents
- A Help Center portal for your knowledge base articles

SUPPORT PORTAL ACCESS
---------------------
Web: {support_url}

MOBILE APP
----------
iOS: https://apps.apple.com/app/chatwoot/id1495796682
Android: https://play.google.com/store/apps/details?id=com.chatwoot.app

Server URL: {support_url}

You will receive a separate email from the support system to set your password.

Best regards,
The Decent Cloud Team"#
        ),
        false,
        EmailType::General,
    )
    .await;

    Ok(password)
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

    #[test]
    fn test_generate_portal_slug_basic() {
        let pubkey = [0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78, 0x9a];
        let slug = generate_portal_slug("Acme Hosting", &pubkey);
        assert_eq!(slug, "acme-hosting-abcdef12");
    }

    #[test]
    fn test_generate_portal_slug_special_chars() {
        let pubkey = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
        let slug = generate_portal_slug("Provider #1 (Best!)", &pubkey);
        assert_eq!(slug, "provider-1-best-11223344");
    }

    #[test]
    fn test_generate_portal_slug_long_name() {
        let pubkey = [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11];
        let long_name = "A".repeat(100);
        let slug = generate_portal_slug(&long_name, &pubkey);
        // 40 chars base + dash + 8 hex chars = 49 chars max
        assert!(slug.len() <= 49);
        assert!(slug.ends_with("-aabbccdd"));
    }
}
