//! Notify subcommand: test email/telegram.
use crate::handle_test_email;
use anyhow::{Context, Result};
use clap::Subcommand;
use std::env;
#[derive(Subcommand)]
pub(crate) enum NotifyAction {
    /// Send test email
    Email {
        /// Recipient email address
        #[arg(long)]
        to: String,
        /// Test DKIM signing
        #[arg(long)]
        with_dkim: bool,
    },
    /// Send test Telegram notification
    Telegram {
        /// Chat ID
        #[arg(long)]
        chat_id: String,
        /// Message text
        #[arg(long)]
        message: String,
    },
}
// =============================================================================
// Notify handlers
// =============================================================================

pub(crate) async fn handle_notify_action(action: NotifyAction) -> Result<()> {
    match action {
        NotifyAction::Email { to, with_dkim } => handle_test_email(&to, with_dkim).await,
        NotifyAction::Telegram { chat_id, message } => {
            let bot_token = env::var("TELEGRAM_BOT_TOKEN").context("TELEGRAM_BOT_TOKEN not set")?;

            let http = api::http_util::http_client();
            let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);

            let params = serde_json::json!({
                "chat_id": chat_id,
                "text": message,
            });

            let response = http.post(&url).json(&params).send().await?;

            if response.status().is_success() {
                println!("Telegram message sent successfully to chat {}", chat_id);
            } else {
                let text = response.text().await?;
                anyhow::bail!("Failed to send Telegram message: {}", text);
            }
            Ok(())
        }
    }
}

