//! Identity subcommand: keypair generation/import/list/show/delete.
use crate::api_cli::Identity;
use anyhow::Result;
use clap::Subcommand;
#[derive(Subcommand)]
pub(crate) enum IdentityAction {
    /// Generate a new keypair
    Generate {
        /// Name for the identity
        #[arg(long)]
        name: String,
    },
    /// Import an existing keypair from file
    Import {
        /// Name for the identity
        #[arg(long)]
        name: String,
        /// Path to secret key file (hex or PEM format)
        #[arg(long)]
        secret_key: String,
    },
    /// List all saved identities
    List,
    /// Show public key for an identity
    Show {
        /// Name of the identity
        name: String,
    },
    /// Delete an identity
    Delete {
        /// Name of the identity
        name: String,
    },
}
// =============================================================================
// Identity handlers
// =============================================================================

pub(crate) async fn handle_identity_action(action: IdentityAction) -> Result<()> {
    match action {
        IdentityAction::Generate { name } => {
            let identity = Identity::generate(&name)?;
            println!("Generated identity: {}", name);
            println!("  Public key: {}", identity.public_key_hex);
            println!("  Stored at: {}", Identity::path(&name)?.display());
        }
        IdentityAction::Import { name, secret_key } => {
            let identity = Identity::import(&name, &secret_key)?;
            println!("Imported identity: {}", name);
            println!("  Public key: {}", identity.public_key_hex);
            println!("  Stored at: {}", Identity::path(&name)?.display());
        }
        IdentityAction::List => {
            let identities = Identity::list()?;
            if identities.is_empty() {
                println!("No identities found.");
                println!("Use 'api-cli identity generate --name <name>' to create one.");
            } else {
                println!("\nSaved Identities:");
                println!("{}", "=".repeat(100));
                println!("{:<20} {:<66} {:<20}", "Name", "Public Key", "Created At");
                println!("{}", "-".repeat(100));
                for id in &identities {
                    println!(
                        "{:<20} {:<66} {:<20}",
                        id.name,
                        id.public_key_hex,
                        &id.created_at[..19]
                    );
                }
                println!("{}", "=".repeat(100));
                println!("Total: {} identity(ies)", identities.len());
            }
        }
        IdentityAction::Show { name } => {
            let identity = Identity::load(&name)?;
            println!("Identity: {}", identity.name);
            println!("  Public key: {}", identity.public_key_hex);
            println!("  Created: {}", identity.created_at);
        }
        IdentityAction::Delete { name } => {
            Identity::delete(&name)?;
            println!("Deleted identity: {}", name);
        }
    }
    Ok(())
}

