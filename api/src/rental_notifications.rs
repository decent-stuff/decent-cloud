//! Notifications for rental contract events (new rental requests, etc.)

use crate::database::contracts::Contract;
use crate::database::Database;
use anyhow::{Context, Result};
use email_utils::EmailService;
use std::sync::Arc;

/// Notify provider about a new rental request after payment succeeds.
/// Uses the provider's notification preferences (telegram, email, SMS).
pub async fn notify_provider_new_rental(
    db: &Database,
    email_service: Option<&Arc<EmailService>>,
    contract: &Contract,
) -> Result<()> {
    let provider_pubkey = hex::decode(&contract.provider_pubkey)
        .context("Invalid provider pubkey hex")?;

    // Get provider notification config
    let config = db
        .get_user_notification_config(&provider_pubkey)
        .await
        .context("Failed to get provider notification config")?;

    let Some(config) = config else {
        tracing::debug!(
            "No notification config for provider {}, skipping rental notification",
            contract.provider_pubkey
        );
        return Ok(());
    };

    let provider_id = &contract.provider_pubkey;
    let mut channels_sent = Vec::new();
    let mut errors = Vec::new();

    // Format notification message
    let amount_display = format_amount(contract.payment_amount_e9s, &contract.currency);
    let duration_display = contract
        .duration_hours
        .map(format_duration)
        .unwrap_or_else(|| "Unknown".to_string());

    let summary = format!(
        "New rental request: {} for {} (Contract: {}...)",
        amount_display,
        duration_display,
        &contract.contract_id[..16]
    );

    // Send to Telegram if enabled
    if config.notify_telegram {
        if let Some(chat_id) = &config.telegram_chat_id {
            match send_telegram_rental_notification(chat_id, contract, &summary).await {
                Ok(()) => {
                    db.increment_notification_usage(provider_id, "telegram")
                        .await
                        .ok();
                    channels_sent.push("telegram");
                }
                Err(e) => errors.push(format!("telegram: {}", e)),
            }
        }
    }

    // Send to Email if enabled
    if config.notify_email {
        match send_email_rental_notification(db, email_service, &provider_pubkey, contract, &summary).await {
            Ok(()) => {
                db.increment_notification_usage(provider_id, "email")
                    .await
                    .ok();
                channels_sent.push("email");
            }
            Err(e) => errors.push(format!("email: {}", e)),
        }
    }

    if channels_sent.is_empty() && errors.is_empty() {
        tracing::debug!(
            "No notification channels enabled for provider {}",
            contract.provider_pubkey
        );
    } else {
        tracing::info!(
            "Rental notification for contract {} - sent: [{}], errors: [{}]",
            &contract.contract_id[..16],
            channels_sent.join(", "),
            errors.join(", ")
        );
    }

    Ok(())
}

async fn send_telegram_rental_notification(
    chat_id: &str,
    contract: &Contract,
    _summary: &str,
) -> Result<()> {
    use crate::notifications::telegram::TelegramClient;

    if !TelegramClient::is_configured() {
        anyhow::bail!("TELEGRAM_BOT_TOKEN not configured");
    }

    let telegram = TelegramClient::from_env()?;

    let amount_display = format_amount(contract.payment_amount_e9s, &contract.currency);
    let duration = contract
        .duration_hours
        .map(format_duration)
        .unwrap_or_else(|| "Unknown".to_string());

    let message = format!(
        "*New Rental Request*\n\n\
        {} for {}\n\n\
        Contract: `{}`\n\n\
        Please review and accept/reject in your provider dashboard.",
        amount_display, duration, &contract.contract_id[..32]
    );

    telegram.send_message(chat_id, &message).await?;

    tracing::info!(
        "Telegram rental notification sent to chat_id: {} for contract {}",
        chat_id,
        &contract.contract_id[..16]
    );
    Ok(())
}

async fn send_email_rental_notification(
    db: &Database,
    email_service: Option<&Arc<EmailService>>,
    provider_pubkey: &[u8],
    contract: &Contract,
    _summary: &str,
) -> Result<()> {
    let email_svc = email_service.ok_or_else(|| {
        anyhow::anyhow!("Email service not configured (missing MAILCHANNELS_API_KEY)")
    })?;

    // Look up provider email
    let account_id = db
        .get_account_id_by_public_key(provider_pubkey)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No account found for provider pubkey"))?;

    let account = db
        .get_account_by_id(&account_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Account not found"))?;

    let email_addr = account
        .email
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No email address on provider account"))?;

    let amount_display = format_amount(contract.payment_amount_e9s, &contract.currency);
    let duration = contract
        .duration_hours
        .map(format_duration)
        .unwrap_or_else(|| "Unknown".to_string());

    let email_body = format!(
        "You have received a new rental request.\n\n\
        Amount: {}\n\
        Duration: {}\n\
        Contract ID: {}\n\n\
        Please log in to your provider dashboard to review and accept or reject this request.\n\n\
        Dashboard: https://decent-cloud.org/dashboard/provider/requests",
        amount_display, duration, contract.contract_id
    );

    let from_addr =
        std::env::var("EMAIL_FROM_ADDR").unwrap_or_else(|_| "noreply@decent-cloud.org".to_string());

    email_svc
        .send_email(
            &from_addr,
            email_addr,
            "New Rental Request Received",
            &email_body,
            false,
        )
        .await
        .context("Failed to send email")?;

    tracing::info!(
        "Email rental notification sent to {} for contract {}",
        email_addr,
        &contract.contract_id[..16]
    );
    Ok(())
}

/// Format amount from e9s to human-readable string
fn format_amount(amount_e9s: i64, currency: &str) -> String {
    let amount = amount_e9s as f64 / 1_000_000_000.0;
    format!("{:.2} {}", amount, currency)
}

/// Format duration hours to human-readable string
fn format_duration(hours: i64) -> String {
    if hours < 24 {
        format!("{} hours", hours)
    } else if hours < 168 {
        format!("{} days", hours / 24)
    } else if hours < 720 {
        format!("{} weeks", hours / 168)
    } else {
        format!("{} months", hours / 720)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_amount() {
        assert_eq!(format_amount(1_000_000_000, "USD"), "1.00 USD");
        assert_eq!(format_amount(10_500_000_000, "EUR"), "10.50 EUR");
        assert_eq!(format_amount(123_456_789_000, "USD"), "123.46 USD");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(1), "1 hours");
        assert_eq!(format_duration(24), "1 days");
        assert_eq!(format_duration(168), "1 weeks");
        assert_eq!(format_duration(720), "1 months");
        assert_eq!(format_duration(2160), "3 months");
    }
}
