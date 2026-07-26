//! Admin subcommand: grant/revoke/list.
use crate::connect_db;
use anyhow::Result;
use clap::Subcommand;
#[derive(Subcommand)]
pub(crate) enum AdminAction {
    /// Grant admin access to a user
    Grant { username: String },
    /// Revoke admin access from a user
    Revoke { username: String },
    /// List all admin accounts
    List,
}
// =============================================================================
// Admin handlers (existing)
// =============================================================================

pub(crate) async fn handle_admin_action(action: AdminAction) -> Result<()> {
    let db = connect_db().await?;

    match action {
        AdminAction::Grant { username } => {
            db.set_admin_status(&username, true).await?;
            println!("✓ Admin access granted to: {}", username);
        }
        AdminAction::Revoke { username } => {
            db.set_admin_status(&username, false).await?;
            println!("✓ Admin access revoked from: {}", username);
        }
        AdminAction::List => {
            let admins = db.list_admins().await?;
            if admins.is_empty() {
                println!("No admin accounts found.");
            } else {
                println!("\nAdmin Accounts:");
                println!("{}", "=".repeat(80));
                println!("{:<20} {:<40} {:<20}", "Username", "Email", "Created At");
                println!("{}", "-".repeat(80));
                for admin in &admins {
                    let email = admin.email.as_deref().unwrap_or("N/A");
                    let created = chrono::DateTime::from_timestamp(admin.created_at, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| "Invalid".to_string());
                    println!("{:<20} {:<40} {:<20}", admin.username, email, created);
                }
                println!("{}", "=".repeat(80));
                println!("Total: {} admin account(s)", admins.len());
            }
        }
    }
    Ok(())
}

