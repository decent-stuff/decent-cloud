//! Notifications for rental contract events (new rental requests, etc.)

use crate::database::contracts::Contract;
use crate::database::Database;
use anyhow::{Context, Result};
use email_utils::EmailService;
use std::net::Ipv4Addr;
use std::sync::Arc;

/// Connection info derived from contract gateway fields and instance details.
/// Prefers gateway access; falls back to public IP; never exposes private IPs.
struct ConnectionInfo {
    ssh_command: String,
    host_display: String,
}

/// Derive SSH login username from the operating system name.
/// Ubuntu cloud images use `ubuntu`; most others default to `root`.
fn ssh_username(os: Option<&str>) -> &'static str {
    match os.map(|s| s.to_lowercase()).as_deref() {
        Some(os) if os.contains("ubuntu") => "ubuntu",
        Some(os) if os.contains("fedora") => "fedora",
        Some(os) if os.contains("centos") => "centos",
        Some(os) if os.contains("alma") => "almalinux",
        Some(os) if os.contains("rocky") => "rocky",
        _ => "root",
    }
}

fn derive_connection_info(contract: &Contract, instance_details: &str) -> ConnectionInfo {
    let user = ssh_username(contract.operating_system.as_deref());

    // Prefer gateway (reverse proxy with public subdomain)
    if let (Some(subdomain), Some(port)) = (&contract.gateway_subdomain, contract.gateway_ssh_port)
    {
        return ConnectionInfo {
            ssh_command: format!("ssh -p {} {}@{}", port, user, subdomain),
            host_display: format!("{} (port {})", subdomain, port),
        };
    }

    // Fall back to public IP from instance details
    if let Ok(details) = serde_json::from_str::<serde_json::Value>(instance_details) {
        // Prefer explicit public_ip field
        if let Some(ip) = details.get("public_ip").and_then(|v| v.as_str()) {
            return ConnectionInfo {
                ssh_command: format!("ssh {}@{}", user, ip),
                host_display: ip.to_string(),
            };
        }
        // Use ip_address only if it's public
        if let Some(ip) = details.get("ip_address").and_then(|v| v.as_str()) {
            if !is_private_ipv4(ip) {
                return ConnectionInfo {
                    ssh_command: format!("ssh {}@{}", user, ip),
                    host_display: ip.to_string(),
                };
            }
        }
    }

    ConnectionInfo {
        ssh_command: "See your dashboard for connection details".to_string(),
        host_display: "Pending — check your dashboard".to_string(),
    }
}

/// Returns true if the IP is RFC1918 private (10.x, 172.16-31.x, 192.168.x).
fn is_private_ipv4(ip: &str) -> bool {
    ip.parse::<Ipv4Addr>()
        .map(|addr| addr.is_private())
        .unwrap_or(false)
}

/// Notify provider about a new rental request after payment succeeds.
/// Uses the provider's notification preferences (telegram, email, SMS).
pub async fn notify_provider_new_rental(
    db: &Database,
    email_service: Option<&Arc<EmailService>>,
    contract: &Contract,
) -> Result<()> {
    let provider_pubkey =
        hex::decode(&contract.provider_pubkey).context("Invalid provider pubkey hex")?;

    // Insert in-app notification for the provider
    let amount_display = format_amount(contract.payment_amount_e9s, &contract.currency);
    let duration_display = contract
        .duration_hours
        .map(format_duration)
        .unwrap_or_else(|| "Unknown".to_string());
    if let Err(e) = db
        .insert_user_notification(
            &provider_pubkey,
            "rental_request",
            "New Rental Request",
            &format!(
                "A tenant requested {} for {}. Contract: {}...",
                amount_display,
                duration_display,
                &contract.contract_id[..16]
            ),
            Some(&contract.contract_id),
            None,
        )
        .await
    {
        tracing::error!(
            "Failed to insert in-app notification for provider {}: {:#}",
            contract.provider_pubkey,
            e
        );
    }

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

    // Insert in-app notification for the requester
    if let Err(e) = db
        .insert_user_notification(
            &requester_pubkey,
            "contract_provisioned",
            "Your VM is Ready",
            &format!(
                "Your virtual machine for contract {}... is provisioned and ready to use.",
                &contract.contract_id[..16]
            ),
            Some(&contract.contract_id),
            None,
        )
        .await
    {
        tracing::error!(
            "Failed to insert in-app notification for requester {}: {:#}",
            contract.requester_pubkey,
            e
        );
    }

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

    let conn = derive_connection_info(contract, instance_details);

    let message = format!(
        "*Your VM is Ready!*\n\n\
        Contract: `{contract_id}`\n\
        Host: `{host}`\n\n\
        Connect: `{ssh_cmd}`\n\n\
        If you generated a key, use:\n\
        `chmod 600 ~/Downloads/id_ed25519_decent_cloud`\n\
        `{ssh_cmd} -o IdentitiesOnly=yes -i ~/Downloads/id_ed25519_decent_cloud`\n\n\
        View details in your dashboard.",
        contract_id = &contract.contract_id[..32],
        host = conn.host_display,
        ssh_cmd = conn.ssh_command,
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

    let conn = derive_connection_info(contract, instance_details);

    let email_body = format!(
        "Great news! Your virtual machine has been provisioned and is ready to use.\n\n\
        Contract ID: {contract_id}\n\
        Host: {host}\n\n\
        Connect using SSH:\n\
        {ssh_cmd}\n\n\
        If you generated a new SSH key during rental, use:\n\
        chmod 600 ~/Downloads/id_ed25519_decent_cloud\n\
        {ssh_cmd} -o IdentitiesOnly=yes -i ~/Downloads/id_ed25519_decent_cloud\n\n\
        View your rental details: https://decent-cloud.org/dashboard/rentals?contract={contract_id}\n\n\
        If you have any issues, please contact the provider through the platform.",
        contract_id = contract.contract_id,
        host = conn.host_display,
        ssh_cmd = conn.ssh_command,
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

/// Notify provider about a password reset event for their contract.
///
/// If `completed` is `false`, the reset was *requested* by the tenant (dc-agent will handle it
/// automatically). If `completed` is `true`, dc-agent has finished the reset and the tenant's
/// new password is available in the dashboard.
pub async fn notify_provider_password_reset(
    db: &Database,
    email_service: Option<&Arc<EmailService>>,
    contract: &Contract,
    completed: bool,
) -> Result<()> {
    let provider_pubkey =
        hex::decode(&contract.provider_pubkey).context("Invalid provider pubkey hex")?;

    let config = db
        .get_user_notification_config(&provider_pubkey)
        .await
        .context("Failed to get provider notification config")?;

    let Some(config) = config else {
        tracing::debug!(
            "No notification config for provider {}, skipping password reset notification",
            contract.provider_pubkey
        );
        return Ok(());
    };

    let provider_id = &contract.provider_pubkey;
    let mut channels_sent = Vec::new();
    let mut errors = Vec::new();

    if config.notify_telegram {
        if let Some(chat_id) = &config.telegram_chat_id {
            match send_telegram_password_reset_notification(chat_id, contract, completed).await {
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

    if config.notify_email {
        match send_email_password_reset_notification(
            db,
            email_service,
            &provider_pubkey,
            contract,
            completed,
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

    let event = if completed { "completed" } else { "requested" };
    if !channels_sent.is_empty() || !errors.is_empty() {
        tracing::info!(
            "Password reset {} notification for contract {} - sent: [{}], errors: [{}]",
            event,
            &contract.contract_id[..16],
            channels_sent.join(", "),
            errors.join(", ")
        );
    }

    Ok(())
}

async fn send_telegram_password_reset_notification(
    chat_id: &str,
    contract: &Contract,
    completed: bool,
) -> Result<()> {
    use crate::notifications::telegram::TelegramClient;

    if !TelegramClient::is_configured() {
        anyhow::bail!("TELEGRAM_BOT_TOKEN not configured");
    }

    let telegram = TelegramClient::from_env()?;

    let message = if completed {
        format!(
            "*Password Reset Completed*\n\n\
            Contract: `{}`\n\n\
            The agent has reset the password. The tenant can retrieve it from their dashboard.",
            &contract.contract_id[..32]
        )
    } else {
        format!(
            "*Password Reset Requested*\n\n\
            Contract: `{}`\n\n\
            Your agent will process this automatically.",
            &contract.contract_id[..32]
        )
    };

    telegram.send_message(chat_id, &message).await?;

    tracing::info!(
        "Telegram password reset {} notification sent to chat_id: {} for contract {}",
        if completed { "completed" } else { "requested" },
        chat_id,
        &contract.contract_id[..16]
    );
    Ok(())
}

async fn send_email_password_reset_notification(
    db: &Database,
    email_service: Option<&Arc<EmailService>>,
    provider_pubkey: &[u8],
    contract: &Contract,
    completed: bool,
) -> Result<()> {
    let email_svc = email_service.ok_or_else(|| {
        anyhow::anyhow!("Email service not configured (missing MAILCHANNELS_API_KEY)")
    })?;

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

    let (subject, email_body) = if completed {
        (
            "Password Reset Completed - Decent Cloud",
            format!(
                "The agent has completed a password reset for contract {}.\n\n\
                The tenant can retrieve their new password from the dashboard.",
                contract.contract_id
            ),
        )
    } else {
        (
            "Password Reset Requested - Decent Cloud",
            format!(
                "A tenant has requested a password reset for contract {}. Your agent will process this automatically.",
                contract.contract_id
            ),
        )
    };

    let from_addr =
        std::env::var("EMAIL_FROM_ADDR").unwrap_or_else(|_| "noreply@decent-cloud.org".to_string());

    email_svc
        .send_email(&from_addr, email_addr, subject, &email_body, false)
        .await
        .context("Failed to send email")?;

    tracing::info!(
        "Email password reset {} notification sent to {} for contract {}",
        if completed { "completed" } else { "requested" },
        email_addr,
        &contract.contract_id[..16]
    );
    Ok(())
}

/// Notify the tenant (requester) that their password reset has been completed by dc-agent.
/// The new credentials are available in their dashboard.
pub async fn notify_tenant_password_reset_complete(
    db: &Database,
    email_service: Option<&Arc<EmailService>>,
    contract: &Contract,
) -> Result<()> {
    let requester_pubkey =
        hex::decode(&contract.requester_pubkey).context("Invalid requester pubkey hex")?;

    // Insert in-app notification for the tenant
    if let Err(e) = db
        .insert_user_notification(
            &requester_pubkey,
            "password_reset_complete",
            "Password Reset Complete",
            &format!(
                "Your password has been reset for contract {}.... Retrieve new credentials from the dashboard.",
                &contract.contract_id[..16]
            ),
            Some(&contract.contract_id),
            None,
        )
        .await
    {
        tracing::error!(
            "Failed to insert in-app notification for tenant {}: {:#}",
            contract.requester_pubkey,
            e
        );
    }

    let config = db
        .get_user_notification_config(&requester_pubkey)
        .await
        .context("Failed to get tenant notification config")?;

    let Some(config) = config else {
        tracing::debug!(
            "No notification config for tenant {}, skipping password reset complete notification",
            contract.requester_pubkey
        );
        return Ok(());
    };

    let tenant_id = &contract.requester_pubkey;
    let mut channels_sent = Vec::new();
    let mut errors = Vec::new();

    if config.notify_telegram {
        if let Some(chat_id) = &config.telegram_chat_id {
            match send_telegram_tenant_password_reset_complete(chat_id, contract).await {
                Ok(()) => {
                    if let Err(e) = db.increment_notification_usage(tenant_id, "telegram").await {
                        tracing::error!(
                            "Failed to increment telegram notification usage for {}: {:#}",
                            tenant_id,
                            e
                        );
                    }
                    channels_sent.push("telegram");
                }
                Err(e) => errors.push(format!("telegram: {}", e)),
            }
        }
    }

    if config.notify_email {
        match send_email_tenant_password_reset_complete(
            db,
            email_service,
            &requester_pubkey,
            contract,
        )
        .await
        {
            Ok(()) => {
                if let Err(e) = db.increment_notification_usage(tenant_id, "email").await {
                    tracing::error!(
                        "Failed to increment email notification usage for {}: {:#}",
                        tenant_id,
                        e
                    );
                }
                channels_sent.push("email");
            }
            Err(e) => errors.push(format!("email: {}", e)),
        }
    }

    if !channels_sent.is_empty() || !errors.is_empty() {
        tracing::info!(
            "Password reset complete notification for tenant on contract {} - sent: [{}], errors: [{}]",
            &contract.contract_id[..16],
            channels_sent.join(", "),
            errors.join(", ")
        );
    }

    Ok(())
}

async fn send_telegram_tenant_password_reset_complete(
    chat_id: &str,
    contract: &Contract,
) -> Result<()> {
    use crate::notifications::telegram::TelegramClient;

    if !TelegramClient::is_configured() {
        anyhow::bail!("TELEGRAM_BOT_TOKEN not configured");
    }

    let telegram = TelegramClient::from_env()?;

    let message = format!(
        "*Password Reset Complete*\n\n\
        Your password has been reset for contract `{}`.\n\n\
        Log in to your dashboard to retrieve the new credentials:\n\
        https://decent-cloud.org/dashboard/rentals?contract={}",
        &contract.contract_id[..32],
        contract.contract_id
    );

    telegram.send_message(chat_id, &message).await?;

    tracing::info!(
        "Telegram password reset complete notification sent to tenant chat_id: {} for contract {}",
        chat_id,
        &contract.contract_id[..16]
    );
    Ok(())
}

async fn send_email_tenant_password_reset_complete(
    db: &Database,
    email_service: Option<&Arc<EmailService>>,
    requester_pubkey: &[u8],
    contract: &Contract,
) -> Result<()> {
    let email_svc = email_service.ok_or_else(|| {
        anyhow::anyhow!("Email service not configured (missing MAILCHANNELS_API_KEY)")
    })?;

    let account_id = db
        .get_account_id_by_public_key(requester_pubkey)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No account found for tenant pubkey"))?;

    let account = db
        .get_account(&account_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Account not found"))?;

    let email_addr = account
        .email
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No email address on tenant account"))?;

    let email_body = format!(
        "Your password has been reset for contract {}.\n\n\
        Log in to your dashboard to retrieve the new credentials:\n\
        https://decent-cloud.org/dashboard/rentals?contract={}\n\n\
        If you did not request this reset, please contact support.",
        contract.contract_id, contract.contract_id
    );

    let from_addr =
        std::env::var("EMAIL_FROM_ADDR").unwrap_or_else(|_| "noreply@decent-cloud.org".to_string());

    email_svc
        .send_email(
            &from_addr,
            email_addr,
            "Password Reset Complete - Decent Cloud",
            &email_body,
            false,
        )
        .await
        .context("Failed to send email")?;

    tracing::info!(
        "Email password reset complete notification sent to tenant {} for contract {}",
        email_addr,
        &contract.contract_id[..16]
    );
    Ok(())
}

/// Notify offering owner when a buyer's recipe script fails during provisioning.
pub async fn notify_offering_owner_recipe_failure(
    db: &Database,
    email_service: Option<&Arc<EmailService>>,
    contract_id: &[u8],
    exit_code: i32,
    log_excerpt: &str,
) -> Result<()> {
    let contract_id_hex = hex::encode(contract_id);
    let contract = db
        .get_contract(contract_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Contract not found: {}", contract_id_hex))?;

    // The offering owner is the provider
    let provider_pubkey =
        hex::decode(&contract.provider_pubkey).context("Invalid provider pubkey hex")?;

    let config = db
        .get_user_notification_config(&provider_pubkey)
        .await
        .context("Failed to get provider notification config")?;

    let Some(config) = config else {
        tracing::debug!(
            "No notification config for provider {}, skipping recipe failure notification",
            contract.provider_pubkey
        );
        return Ok(());
    };

    let provider_id = &contract.provider_pubkey;
    let mut channels_sent = Vec::new();
    let mut errors = Vec::new();

    let log_truncated = truncate_log(log_excerpt, 500);

    // Telegram
    if config.notify_telegram {
        if let Some(chat_id) = &config.telegram_chat_id {
            match send_telegram_recipe_failure(chat_id, &contract_id_hex, exit_code, &log_truncated)
                .await
            {
                Ok(()) => {
                    if let Err(e) = db
                        .increment_notification_usage(provider_id, "telegram")
                        .await
                    {
                        tracing::error!(
                            "Failed to increment telegram usage for {}: {:#}",
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

    // Email
    if config.notify_email {
        match send_email_recipe_failure(
            db,
            email_service,
            &provider_pubkey,
            &contract_id_hex,
            exit_code,
            &log_truncated,
        )
        .await
        {
            Ok(()) => {
                if let Err(e) = db.increment_notification_usage(provider_id, "email").await {
                    tracing::error!(
                        "Failed to increment email usage for {}: {:#}",
                        provider_id,
                        e
                    );
                }
                channels_sent.push("email");
            }
            Err(e) => errors.push(format!("email: {}", e)),
        }
    }

    if !channels_sent.is_empty() || !errors.is_empty() {
        tracing::info!(
            "Recipe failure notification for contract {} - sent: [{}], errors: [{}]",
            &contract_id_hex[..16],
            channels_sent.join(", "),
            errors.join(", ")
        );
    }

    Ok(())
}

async fn send_telegram_recipe_failure(
    chat_id: &str,
    contract_id: &str,
    exit_code: i32,
    log_excerpt: &str,
) -> Result<()> {
    use crate::notifications::telegram::TelegramClient;

    if !TelegramClient::is_configured() {
        anyhow::bail!("TELEGRAM_BOT_TOKEN not configured");
    }

    let telegram = TelegramClient::from_env()?;

    let message = format!(
        "*Recipe Script Failed*\n\n\
        Contract: `{}`\n\
        Exit code: {}\n\n\
        ```\n{}\n```\n\n\
        Check the contract details in your dashboard.",
        &contract_id[..32],
        exit_code,
        log_excerpt
    );

    telegram.send_message(chat_id, &message).await?;
    Ok(())
}

async fn send_email_recipe_failure(
    db: &Database,
    email_service: Option<&Arc<EmailService>>,
    provider_pubkey: &[u8],
    contract_id: &str,
    exit_code: i32,
    log_excerpt: &str,
) -> Result<()> {
    let email_svc = email_service.ok_or_else(|| {
        anyhow::anyhow!("Email service not configured (missing MAILCHANNELS_API_KEY)")
    })?;

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
        .ok_or_else(|| anyhow::anyhow!("No email on provider account"))?;

    let email_body = format!(
        "A recipe script failed during VM provisioning.\n\n\
        Contract ID: {}\n\
        Exit Code: {}\n\n\
        Script Output:\n{}\n\n\
        Please check the contract details in your dashboard.",
        contract_id, exit_code, log_excerpt
    );

    let from_addr =
        std::env::var("EMAIL_FROM_ADDR").unwrap_or_else(|_| "noreply@decent-cloud.org".to_string());

    email_svc
        .send_email(
            &from_addr,
            email_addr,
            "Recipe Script Failed - Decent Cloud",
            &email_body,
            false,
        )
        .await
        .context("Failed to send recipe failure email")?;

    Ok(())
}

/// Truncate a log string to `max_len` characters, appending "...(truncated)" if needed.
fn truncate_log(log: &str, max_len: usize) -> String {
    if log.len() > max_len {
        format!("{}...(truncated)", &log[..max_len])
    } else {
        log.to_string()
    }
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

    #[test]
    fn test_truncate_log_short() {
        let short = "short log";
        let result = truncate_log(short, 500);
        assert_eq!(result, "short log");
    }

    #[test]
    fn test_truncate_log_long() {
        let long = "x".repeat(600);
        let result = truncate_log(&long, 500);
        assert_eq!(result.len(), 500 + "...(truncated)".len());
        assert!(result.ends_with("...(truncated)"));
        assert!(result.starts_with(&"x".repeat(500)));
    }

    #[test]
    fn test_truncate_log_exact_boundary() {
        let exact = "y".repeat(500);
        let result = truncate_log(&exact, 500);
        assert_eq!(result, exact);
    }

    /// Verify that the Telegram password-reset helper returns a clear error
    /// when TELEGRAM_BOT_TOKEN is absent — covering the `bail!` early-exit path.
    #[tokio::test]
    async fn test_send_telegram_password_reset_no_token() {
        use crate::database::contracts::Contract;

        let contract = Contract {
            contract_id: "a".repeat(64),
            requester_pubkey: "b".repeat(64),
            requester_ssh_pubkey: String::new(),
            requester_contact: String::new(),
            provider_pubkey: "c".repeat(64),
            offering_id: "offering-1".to_string(),
            region_name: None,
            instance_config: None,
            payment_amount_e9s: 1_000_000_000,
            start_timestamp_ns: None,
            end_timestamp_ns: None,
            duration_hours: Some(24),
            original_duration_hours: None,
            request_memo: String::new(),
            created_at_ns: 0,
            status: "active".to_string(),
            provisioning_instance_details: None,
            provisioning_completed_at_ns: None,
            payment_method: "stripe".to_string(),
            stripe_payment_intent_id: None,
            stripe_customer_id: None,
            icpay_transaction_id: None,
            payment_status: "succeeded".to_string(),
            currency: "USD".to_string(),
            refund_amount_e9s: None,
            stripe_refund_id: None,
            refund_created_at_ns: None,
            status_updated_at_ns: None,
            icpay_payment_id: None,
            icpay_refund_id: None,
            total_released_e9s: None,
            last_release_at_ns: None,
            tax_amount_e9s: None,
            tax_rate_percent: None,
            tax_type: None,
            tax_jurisdiction: None,
            customer_tax_id: None,
            reverse_charge: None,
            buyer_address: None,
            stripe_invoice_id: None,
            receipt_number: None,
            receipt_sent_at_ns: None,
            stripe_subscription_id: None,
            subscription_status: None,
            current_period_end_ns: None,
            cancel_at_period_end: false,
            auto_renew: false,
            gateway_slug: None,
            gateway_subdomain: None,
            gateway_ssh_port: None,
            gateway_port_range_start: None,
            gateway_port_range_end: None,
            password_reset_requested_at_ns: None,
            ssh_key_rotation_requested_at_ns: None,
            offering_name: None,
            operating_system: None,
        };

        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        let result = send_telegram_password_reset_notification("chat-123", &contract, false).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("TELEGRAM_BOT_TOKEN"),
            "expected TELEGRAM_BOT_TOKEN error, got: {}",
            err
        );
    }

    /// Verify that the completed=true path also bails out without TELEGRAM_BOT_TOKEN,
    /// confirming the `completed` parameter is threaded through.
    #[tokio::test]
    async fn test_send_telegram_password_reset_completed_no_token() {
        use crate::database::contracts::Contract;

        let contract = Contract {
            contract_id: "a".repeat(64),
            requester_pubkey: "b".repeat(64),
            requester_ssh_pubkey: String::new(),
            requester_contact: String::new(),
            provider_pubkey: "c".repeat(64),
            offering_id: "offering-1".to_string(),
            region_name: None,
            instance_config: None,
            payment_amount_e9s: 1_000_000_000,
            start_timestamp_ns: None,
            end_timestamp_ns: None,
            duration_hours: Some(24),
            original_duration_hours: None,
            request_memo: String::new(),
            created_at_ns: 0,
            status: "active".to_string(),
            provisioning_instance_details: None,
            provisioning_completed_at_ns: None,
            payment_method: "stripe".to_string(),
            stripe_payment_intent_id: None,
            stripe_customer_id: None,
            icpay_transaction_id: None,
            payment_status: "succeeded".to_string(),
            currency: "USD".to_string(),
            refund_amount_e9s: None,
            stripe_refund_id: None,
            refund_created_at_ns: None,
            status_updated_at_ns: None,
            icpay_payment_id: None,
            icpay_refund_id: None,
            total_released_e9s: None,
            last_release_at_ns: None,
            tax_amount_e9s: None,
            tax_rate_percent: None,
            tax_type: None,
            tax_jurisdiction: None,
            customer_tax_id: None,
            reverse_charge: None,
            buyer_address: None,
            stripe_invoice_id: None,
            receipt_number: None,
            receipt_sent_at_ns: None,
            stripe_subscription_id: None,
            subscription_status: None,
            current_period_end_ns: None,
            cancel_at_period_end: false,
            auto_renew: false,
            gateway_slug: None,
            gateway_subdomain: None,
            gateway_ssh_port: None,
            gateway_port_range_start: None,
            gateway_port_range_end: None,
            password_reset_requested_at_ns: None,
            ssh_key_rotation_requested_at_ns: None,
            offering_name: None,
            operating_system: None,
        };

        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        let result = send_telegram_password_reset_notification("chat-123", &contract, true).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("TELEGRAM_BOT_TOKEN"),
            "expected TELEGRAM_BOT_TOKEN error, got: {}",
            err
        );
    }

    /// Verify that `send_telegram_tenant_password_reset_complete` bails early with a
    /// descriptive error when TELEGRAM_BOT_TOKEN is absent, covering the fail-fast path
    /// in the new tenant notification helper.
    #[tokio::test]
    async fn test_send_telegram_tenant_password_reset_complete_no_token() {
        use crate::database::contracts::Contract;

        let contract = Contract {
            contract_id: "a".repeat(64),
            requester_pubkey: "b".repeat(64),
            requester_ssh_pubkey: String::new(),
            requester_contact: String::new(),
            provider_pubkey: "c".repeat(64),
            offering_id: "offering-1".to_string(),
            region_name: None,
            instance_config: None,
            payment_amount_e9s: 1_000_000_000,
            start_timestamp_ns: None,
            end_timestamp_ns: None,
            duration_hours: Some(24),
            original_duration_hours: None,
            request_memo: String::new(),
            created_at_ns: 0,
            status: "active".to_string(),
            provisioning_instance_details: None,
            provisioning_completed_at_ns: None,
            payment_method: "stripe".to_string(),
            stripe_payment_intent_id: None,
            stripe_customer_id: None,
            icpay_transaction_id: None,
            payment_status: "succeeded".to_string(),
            currency: "USD".to_string(),
            refund_amount_e9s: None,
            stripe_refund_id: None,
            refund_created_at_ns: None,
            status_updated_at_ns: None,
            icpay_payment_id: None,
            icpay_refund_id: None,
            total_released_e9s: None,
            last_release_at_ns: None,
            tax_amount_e9s: None,
            tax_rate_percent: None,
            tax_type: None,
            tax_jurisdiction: None,
            customer_tax_id: None,
            reverse_charge: None,
            buyer_address: None,
            stripe_invoice_id: None,
            receipt_number: None,
            receipt_sent_at_ns: None,
            stripe_subscription_id: None,
            subscription_status: None,
            current_period_end_ns: None,
            cancel_at_period_end: false,
            auto_renew: false,
            gateway_slug: None,
            gateway_subdomain: None,
            gateway_ssh_port: None,
            gateway_port_range_start: None,
            gateway_port_range_end: None,
            password_reset_requested_at_ns: None,
            ssh_key_rotation_requested_at_ns: None,
            offering_name: None,
            operating_system: None,
        };

        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        let result = send_telegram_tenant_password_reset_complete("chat-456", &contract).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("TELEGRAM_BOT_TOKEN"),
            "expected TELEGRAM_BOT_TOKEN error, got: {}",
            err
        );
    }

    #[test]
    fn test_is_private_ipv4() {
        assert!(is_private_ipv4("10.0.0.1"));
        assert!(is_private_ipv4("10.255.255.255"));
        assert!(is_private_ipv4("172.16.0.146"));
        assert!(is_private_ipv4("172.31.255.255"));
        assert!(is_private_ipv4("192.168.1.1"));
        assert!(!is_private_ipv4("172.15.0.1"));
        assert!(!is_private_ipv4("172.32.0.1"));
        assert!(!is_private_ipv4("8.8.8.8"));
        assert!(!is_private_ipv4("203.0.113.1"));
        assert!(!is_private_ipv4("not-an-ip"));
        assert!(!is_private_ipv4(""));
    }

    fn make_test_contract(
        gateway_subdomain: Option<&str>,
        gateway_ssh_port: Option<i32>,
    ) -> Contract {
        Contract {
            contract_id: "a".repeat(64),
            requester_pubkey: "b".repeat(64),
            requester_ssh_pubkey: String::new(),
            requester_contact: String::new(),
            provider_pubkey: "c".repeat(64),
            offering_id: "offering-1".to_string(),
            region_name: None,
            instance_config: None,
            payment_amount_e9s: 1_000_000_000,
            start_timestamp_ns: None,
            end_timestamp_ns: None,
            duration_hours: Some(24),
            original_duration_hours: None,
            request_memo: String::new(),
            created_at_ns: 0,
            status: "active".to_string(),
            provisioning_instance_details: None,
            provisioning_completed_at_ns: None,
            payment_method: "stripe".to_string(),
            stripe_payment_intent_id: None,
            stripe_customer_id: None,
            icpay_transaction_id: None,
            payment_status: "succeeded".to_string(),
            currency: "USD".to_string(),
            refund_amount_e9s: None,
            stripe_refund_id: None,
            refund_created_at_ns: None,
            status_updated_at_ns: None,
            icpay_payment_id: None,
            icpay_refund_id: None,
            total_released_e9s: None,
            last_release_at_ns: None,
            tax_amount_e9s: None,
            tax_rate_percent: None,
            tax_type: None,
            tax_jurisdiction: None,
            customer_tax_id: None,
            reverse_charge: None,
            buyer_address: None,
            stripe_invoice_id: None,
            receipt_number: None,
            receipt_sent_at_ns: None,
            stripe_subscription_id: None,
            subscription_status: None,
            current_period_end_ns: None,
            cancel_at_period_end: false,
            auto_renew: false,
            gateway_slug: None,
            gateway_subdomain: gateway_subdomain.map(|s| s.to_string()),
            gateway_ssh_port,
            gateway_port_range_start: None,
            gateway_port_range_end: None,
            password_reset_requested_at_ns: None,
            ssh_key_rotation_requested_at_ns: None,
            offering_name: None,
            operating_system: None,
        }
    }

    #[test]
    fn test_ssh_username() {
        assert_eq!(ssh_username(Some("Ubuntu 22.04")), "ubuntu");
        assert_eq!(ssh_username(Some("ubuntu-24.04-lts")), "ubuntu");
        assert_eq!(ssh_username(Some("Fedora 39")), "fedora");
        assert_eq!(ssh_username(Some("CentOS Stream 9")), "centos");
        assert_eq!(ssh_username(Some("AlmaLinux 9")), "almalinux");
        assert_eq!(ssh_username(Some("Rocky Linux 9")), "rocky");
        assert_eq!(ssh_username(Some("Debian 12")), "root");
        assert_eq!(ssh_username(None), "root");
        assert_eq!(ssh_username(Some("SomeUnknownOS")), "root");
    }

    #[test]
    fn test_derive_connection_info_prefers_gateway() {
        let mut contract =
            make_test_contract(Some("k7m2p4.dc-lk.dev-gw.decent-cloud.org"), Some(20000));
        contract.operating_system = Some("Ubuntu 22.04".to_string());
        let details = r#"{"ip_address": "10.0.0.5", "ssh_port": 22}"#;
        let conn = derive_connection_info(&contract, details);
        assert_eq!(
            conn.ssh_command,
            "ssh -p 20000 ubuntu@k7m2p4.dc-lk.dev-gw.decent-cloud.org"
        );
        assert!(conn
            .host_display
            .contains("k7m2p4.dc-lk.dev-gw.decent-cloud.org"));
        // Must NOT contain private IP
        assert!(!conn.ssh_command.contains("10.0.0.5"));
        assert!(!conn.host_display.contains("10.0.0.5"));
    }

    #[test]
    fn test_derive_connection_info_uses_root_for_debian() {
        let mut contract =
            make_test_contract(Some("k7m2p4.dc-lk.dev-gw.decent-cloud.org"), Some(20000));
        contract.operating_system = Some("Debian 12".to_string());
        let details = r#"{"ip_address": "10.0.0.5"}"#;
        let conn = derive_connection_info(&contract, details);
        assert!(
            conn.ssh_command.contains("root@"),
            "Debian should use root: {}",
            conn.ssh_command
        );
    }

    #[test]
    fn test_derive_connection_info_uses_public_ip_fallback() {
        let contract = make_test_contract(None, None);
        let details = r#"{"ip_address": "10.0.0.5", "public_ip": "49.12.34.56"}"#;
        let conn = derive_connection_info(&contract, details);
        assert_eq!(conn.ssh_command, "ssh root@49.12.34.56");
        assert_eq!(conn.host_display, "49.12.34.56");
    }

    #[test]
    fn test_derive_connection_info_uses_public_ip_address() {
        let contract = make_test_contract(None, None);
        let details = r#"{"ip_address": "203.0.113.1"}"#;
        let conn = derive_connection_info(&contract, details);
        assert_eq!(conn.ssh_command, "ssh root@203.0.113.1");
        assert_eq!(conn.host_display, "203.0.113.1");
    }

    #[test]
    fn test_derive_connection_info_hides_private_ip() {
        let contract = make_test_contract(None, None);
        let details = r#"{"ip_address": "172.16.0.146"}"#;
        let conn = derive_connection_info(&contract, details);
        assert!(
            !conn.ssh_command.contains("172.16"),
            "private IP must not appear in ssh_command: {}",
            conn.ssh_command
        );
        assert!(
            !conn.host_display.contains("172.16"),
            "private IP must not appear in host_display: {}",
            conn.host_display
        );
        assert!(conn.host_display.contains("dashboard"));
    }

    #[test]
    fn test_derive_connection_info_invalid_json() {
        let contract = make_test_contract(None, None);
        let conn = derive_connection_info(&contract, "not-json");
        assert!(conn.host_display.contains("dashboard"));
    }
}
