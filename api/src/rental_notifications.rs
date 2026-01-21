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
    let provider_pubkey =
        hex::decode(&contract.provider_pubkey).context("Invalid provider pubkey hex")?;

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
                    if let Err(e) = db
                        .increment_notification_usage(provider_id, "telegram")
                        .await
                    {
                        tracing::error!(
                            "Failed to increment telegram notification usage for {}: {:#}",
                            provider_id,
                            e
                        );
                    }
                    channels_sent.push("telegram");
                }
                Err(e) => errors.push(format!("telegram: {}", e)),
            }
        }
    }

    // Send to Email if enabled
    if config.notify_email {
        match send_email_rental_notification(
            db,
            email_service,
            &provider_pubkey,
            contract,
            &summary,
        )
        .await
        {
            Ok(()) => {
                if let Err(e) = db.increment_notification_usage(provider_id, "email").await {
                    tracing::error!(
                        "Failed to increment email notification usage for {}: {:#}",
                        provider_id,
                        e
                    );
                }
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
        amount_display,
        duration,
        &contract.contract_id[..32]
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
        .get_account(&account_id)
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

/// Notify user (requester) when their VM has been provisioned.
/// Uses the user's notification preferences (telegram, email).
pub async fn notify_user_provisioned(
    db: &Database,
    email_service: Option<&Arc<EmailService>>,
    contract: &Contract,
    instance_details: &str,
) -> Result<()> {
    let requester_pubkey =
        hex::decode(&contract.requester_pubkey).context("Invalid requester pubkey hex")?;

    // Get user notification config
    let config = db
        .get_user_notification_config(&requester_pubkey)
        .await
        .context("Failed to get user notification config")?;

    let Some(config) = config else {
        tracing::debug!(
            "No notification config for user {}, skipping provisioned notification",
            contract.requester_pubkey
        );
        // Fall back to email if no config but user has email
        return send_provisioned_email_fallback(
            db,
            email_service,
            &requester_pubkey,
            contract,
            instance_details,
        )
        .await;
    };

    let user_id = &contract.requester_pubkey;
    let mut channels_sent = Vec::new();
    let mut errors = Vec::new();

    // Send to Telegram if enabled
    if config.notify_telegram {
        if let Some(chat_id) = &config.telegram_chat_id {
            match send_telegram_provisioned_notification(chat_id, contract, instance_details).await
            {
                Ok(()) => {
                    if let Err(e) = db.increment_notification_usage(user_id, "telegram").await {
                        tracing::error!(
                            "Failed to increment telegram notification usage for {}: {:#}",
                            user_id,
                            e
                        );
                    }
                    channels_sent.push("telegram");
                }
                Err(e) => errors.push(format!("telegram: {}", e)),
            }
        }
    }

    // Send to Email if enabled
    if config.notify_email {
        match send_email_provisioned_notification(
            db,
            email_service,
            &requester_pubkey,
            contract,
            instance_details,
        )
        .await
        {
            Ok(()) => {
                if let Err(e) = db.increment_notification_usage(user_id, "email").await {
                    tracing::error!(
                        "Failed to increment email notification usage for {}: {:#}",
                        user_id,
                        e
                    );
                }
                channels_sent.push("email");
            }
            Err(e) => errors.push(format!("email: {}", e)),
        }
    }

    if channels_sent.is_empty() && errors.is_empty() {
        tracing::debug!(
            "No notification channels enabled for user {}, trying email fallback",
            contract.requester_pubkey
        );
        // Try email fallback
        return send_provisioned_email_fallback(
            db,
            email_service,
            &requester_pubkey,
            contract,
            instance_details,
        )
        .await;
    } else {
        tracing::info!(
            "Provisioned notification for contract {} - sent: [{}], errors: [{}]",
            &contract.contract_id[..16],
            channels_sent.join(", "),
            errors.join(", ")
        );
    }

    Ok(())
}

async fn send_telegram_provisioned_notification(
    chat_id: &str,
    contract: &Contract,
    instance_details: &str,
) -> Result<()> {
    use crate::notifications::telegram::TelegramClient;

    if !TelegramClient::is_configured() {
        anyhow::bail!("TELEGRAM_BOT_TOKEN not configured");
    }

    let telegram = TelegramClient::from_env()?;

    // Parse instance details to extract IP
    let ip_info = if let Ok(details) = serde_json::from_str::<serde_json::Value>(instance_details) {
        let ip = details
            .get("ip_address")
            .and_then(|v| v.as_str())
            .unwrap_or("N/A");
        format!("IP: `{}`", ip)
    } else {
        format!("Details: {}", instance_details)
    };

    let message = format!(
        "*Your VM is Ready!*\n\n\
        Contract: `{}`\n\
        {}\n\n\
        Connect with: `ssh root@<ip>`\n\n\
        View details in your dashboard.",
        &contract.contract_id[..32],
        ip_info
    );

    telegram.send_message(chat_id, &message).await?;

    tracing::info!(
        "Telegram provisioned notification sent to chat_id: {} for contract {}",
        chat_id,
        &contract.contract_id[..16]
    );
    Ok(())
}

async fn send_email_provisioned_notification(
    db: &Database,
    email_service: Option<&Arc<EmailService>>,
    requester_pubkey: &[u8],
    contract: &Contract,
    instance_details: &str,
) -> Result<()> {
    let email_svc = email_service.ok_or_else(|| {
        anyhow::anyhow!("Email service not configured (missing MAILCHANNELS_API_KEY)")
    })?;

    // Look up user email
    let account_id = db
        .get_account_id_by_public_key(requester_pubkey)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No account found for requester pubkey"))?;

    let account = db
        .get_account(&account_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Account not found"))?;

    let email_addr = account
        .email
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No email address on user account"))?;

    // Parse instance details for email
    let (ip_address, ipv6_address) =
        if let Ok(details) = serde_json::from_str::<serde_json::Value>(instance_details) {
            let ip = details
                .get("ip_address")
                .and_then(|v| v.as_str())
                .unwrap_or("N/A");
            let ipv6 = details
                .get("ipv6_address")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            (ip.to_string(), ipv6)
        } else {
            ("See dashboard".to_string(), None)
        };

    let ipv6_line = ipv6_address
        .map(|v6| format!("\nIPv6: {}", v6))
        .unwrap_or_default();

    let email_body = format!(
        "Great news! Your virtual machine has been provisioned and is ready to use.\n\n\
        Contract ID: {}\n\
        IP Address: {}{}\n\n\
        Connect using SSH:\n\
        ssh root@{}\n\n\
        Use the SSH key you provided during the rental request.\n\n\
        View your rental details: https://decent-cloud.org/dashboard/rentals?contract={}\n\n\
        If you have any issues, please contact the provider through the platform.",
        contract.contract_id, ip_address, ipv6_line, ip_address, contract.contract_id
    );

    let from_addr =
        std::env::var("EMAIL_FROM_ADDR").unwrap_or_else(|_| "noreply@decent-cloud.org".to_string());

    email_svc
        .send_email(
            &from_addr,
            email_addr,
            "Your VM is Ready - Decent Cloud",
            &email_body,
            false,
        )
        .await
        .context("Failed to send email")?;

    tracing::info!(
        "Email provisioned notification sent to {} for contract {}",
        email_addr,
        &contract.contract_id[..16]
    );
    Ok(())
}

/// Fallback: send email if user has email on account but no notification config
async fn send_provisioned_email_fallback(
    db: &Database,
    email_service: Option<&Arc<EmailService>>,
    requester_pubkey: &[u8],
    contract: &Contract,
    instance_details: &str,
) -> Result<()> {
    // Try to send email as fallback
    match send_email_provisioned_notification(
        db,
        email_service,
        requester_pubkey,
        contract,
        instance_details,
    )
    .await
    {
        Ok(()) => {
            tracing::info!(
                "Email provisioned notification (fallback) sent for contract {}",
                &contract.contract_id[..16]
            );
        }
        Err(e) => {
            tracing::debug!(
                "Could not send email fallback for contract {}: {}",
                &contract.contract_id[..16],
                e
            );
        }
    }
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
