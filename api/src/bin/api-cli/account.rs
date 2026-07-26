//! Account subcommand: create/get/update-email/ssh-keys.
use crate::api_cli::{Identity, SignedClient};
use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};
#[derive(Subcommand)]
pub(crate) enum AccountAction {
    /// Create a new account
    Create {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// Username for the account
        #[arg(long)]
        username: String,
        /// Email address
        #[arg(long)]
        email: String,
    },
    /// Get account information
    Get {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
    },
    /// Update account email
    UpdateEmail {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// New email address
        #[arg(long)]
        email: String,
    },
    /// Add SSH key to account
    AddSshKey {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
        /// SSH public key (e.g., "ssh-ed25519 AAAA...")
        #[arg(long)]
        key: String,
        /// Label for the key
        #[arg(long)]
        label: Option<String>,
    },
    /// List SSH keys for account
    ListSshKeys {
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
    },
}
// =============================================================================
// Account handlers
// =============================================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RegisterAccountRequest {
    username: String,
    email: String,
    public_key: String,
}

#[derive(Debug, Serialize)]
struct UpdateAccountEmailRequest {
    email: String,
}

#[derive(Debug, Serialize)]
struct AddExternalKeyRequest {
    key_type: String,
    key_data: String,
    key_fingerprint: Option<String>,
    label: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AccountWithKeys {
    username: String,
    email: Option<String>,
    email_verified: Option<bool>,
    created_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct AccountExternalKey {
    id: i64,
    key_type: String,
    key_data: String,
    label: Option<String>,
}

pub(crate) async fn handle_account_action(action: AccountAction, api_url: &str) -> Result<()> {
    match action {
        AccountAction::Create {
            identity,
            username,
            email,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let request = RegisterAccountRequest {
                username: username.clone(),
                email: email.clone(),
                public_key: id.public_key_hex.clone(),
            };

            let account: AccountWithKeys = client.post_api("/accounts", &request).await?;
            println!("Account created:");
            println!("  Username: {}", account.username);
            println!(
                "  Email: {}",
                account.email.unwrap_or_else(|| "N/A".to_string())
            );
            println!("  Public Key: {}", id.public_key_hex);
        }
        AccountAction::Get { identity } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            // Search by public key
            let path = format!("/accounts?publicKey={}", id.public_key_hex);
            let account: AccountWithKeys = client.get_api(&path).await?;
            println!("Account:");
            println!("  Username: {}", account.username);
            println!(
                "  Email: {}",
                account.email.unwrap_or_else(|| "N/A".to_string())
            );
            println!(
                "  Email verified: {}",
                account.email_verified.unwrap_or(false)
            );
            if let Some(created) = account.created_at {
                if let Some(dt) = chrono::DateTime::from_timestamp(created, 0) {
                    println!("  Created: {}", dt.format("%Y-%m-%d %H:%M:%S"));
                }
            }
        }
        AccountAction::UpdateEmail { identity, email } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            // First get the account to find username
            let path = format!("/accounts?publicKey={}", id.public_key_hex);
            let account: AccountWithKeys = client.get_api(&path).await?;

            let request = UpdateAccountEmailRequest {
                email: email.clone(),
            };
            let path = format!("/accounts/{}/email", account.username);
            let _: AccountWithKeys = client.put_api(&path, &request).await?;
            println!("Email updated to: {}", email);
        }
        AccountAction::AddSshKey {
            identity,
            key,
            label,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            // First get the account to find username
            let path = format!("/accounts?publicKey={}", id.public_key_hex);
            let account: AccountWithKeys = client.get_api(&path).await?;

            let request = AddExternalKeyRequest {
                key_type: "ssh".to_string(),
                key_data: key.clone(),
                key_fingerprint: None,
                label,
            };
            let path = format!("/accounts/{}/external-keys", account.username);
            let _: String = client.post_api(&path, &request).await?;
            println!("SSH key added successfully");
        }
        AccountAction::ListSshKeys { identity } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            // First get the account to find username
            let path = format!("/accounts?publicKey={}", id.public_key_hex);
            let account: AccountWithKeys = client.get_api(&path).await?;

            let path = format!("/accounts/{}/external-keys", account.username);
            let keys: Vec<AccountExternalKey> = client.get_api(&path).await?;

            if keys.is_empty() {
                println!("No SSH keys found.");
            } else {
                println!("\nSSH Keys:");
                println!("{}", "=".repeat(80));
                for key in &keys {
                    if key.key_type == "ssh" {
                        println!("  ID: {}", key.id);
                        println!("  Label: {}", key.label.as_deref().unwrap_or("N/A"));
                        println!(
                            "  Key: {}...",
                            &key.key_data.chars().take(50).collect::<String>()
                        );
                        println!("{}", "-".repeat(80));
                    }
                }
            }
        }
    }
    Ok(())
}

