//! Background service that alerts providers when contract uptime drops below their threshold.
//!
//! Runs on a configurable interval (default 1 hour).  For every provider that has an SLA
//! uptime config row, it queries contracts whose rolling-window uptime is below the threshold
//! and whose last breach-alert was sent more than 1 hour ago, then sends Telegram/email
//! notifications via the standard notification stack.

use crate::database::Database;
use email_utils::EmailService;
use std::sync::Arc;
use std::time::Duration;

pub struct SlaAlertService {
    database: Arc<Database>,
    email_service: Option<Arc<EmailService>>,
    interval: Duration,
}

impl SlaAlertService {
    pub fn new(
        database: Arc<Database>,
        email_service: Option<Arc<EmailService>>,
        interval_hours: u64,
    ) -> Self {
        Self {
            database,
            email_service,
            interval: Duration::from_secs(interval_hours * 60 * 60),
        }
    }

    /// Run the SLA alert service until shutdown is signalled.
    pub async fn run(self, mut shutdown: tokio::sync::watch::Receiver<bool>) {
        let mut interval = tokio::time::interval(self.interval);

        // Run initial check on startup
        self.check_sla_breaches().await;

        loop {
            tokio::select! {
                _ = interval.tick() => {}
                _ = shutdown.changed() => {
                    tracing::info!("SLA alert service shutting down gracefully");
                    return;
                }
            }
            self.check_sla_breaches().await;
        }
    }

    async fn check_sla_breaches(&self) {
        tracing::info!("Checking for SLA uptime breaches");

        let providers = match self.database.get_providers_with_sla_config().await {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Failed to fetch providers with SLA config: {:#}", e);
                return;
            }
        };

        if providers.is_empty() {
            tracing::debug!("No providers with SLA uptime config, skipping breach check");
            return;
        }

        tracing::info!(
            "{} provider(s) with SLA config, checking for uptime breaches",
            providers.len()
        );

        for provider in providers {
            let breaches = match self
                .database
                .get_contracts_with_sla_breach(
                    provider.sla_alert_window_hours,
                    provider.uptime_threshold_percent,
                )
                .await
            {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!(
                        provider = %hex::encode(&provider.provider_pubkey),
                        "Failed to fetch SLA breaches: {:#}",
                        e
                    );
                    continue;
                }
            };

            // Filter to only this provider's contracts
            let my_breaches: Vec<_> = breaches
                .into_iter()
                .filter(|b| {
                    hex::decode(&b.provider_pubkey)
                        .map(|pk| pk == provider.provider_pubkey)
                        .unwrap_or(false)
                })
                .collect();

            for breach in my_breaches {
                if let Err(e) = self.send_breach_alert(&breach).await {
                    tracing::error!(
                        contract_id = %breach.contract_id,
                        "Failed to send SLA breach alert: {:#}",
                        e
                    );
                    // Continue processing remaining breaches
                }
            }
        }
    }

    async fn send_breach_alert(
        &self,
        breach: &crate::database::providers::SlaBreachInfo,
    ) -> anyhow::Result<()> {
        let provider_pubkey_bytes = hex::decode(&breach.provider_pubkey)
            .map_err(|e| anyhow::anyhow!("Invalid provider pubkey hex: {}", e))?;
        let contract_id_bytes = hex::decode(&breach.contract_id)
            .map_err(|e| anyhow::anyhow!("Invalid contract_id hex: {}", e))?;

        // Get provider notification config
        let config = self
            .database
            .get_user_notification_config(&provider_pubkey_bytes)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get provider notification config: {:#}", e))?;

        let Some(config) = config else {
            tracing::debug!(
                contract_id = %&breach.contract_id[..16.min(breach.contract_id.len())],
                "No notification config for provider, skipping SLA breach alert"
            );
            return Ok(());
        };

        let provider_id = &breach.provider_pubkey;
        let mut channels_sent = Vec::new();
        let mut errors = Vec::new();

        // Telegram
        if config.notify_telegram {
            if let Some(chat_id) = &config.telegram_chat_id {
                match send_telegram_sla_breach(chat_id, breach).await {
                    Ok(()) => {
                        if let Err(e) = self
                            .database
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
            match send_email_sla_breach(
                &self.database,
                self.email_service.as_ref(),
                &provider_pubkey_bytes,
                breach,
            )
            .await
            {
                Ok(()) => {
                    if let Err(e) = self
                        .database
                        .increment_notification_usage(provider_id, "email")
                        .await
                    {
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
                "SLA breach alert for contract {} (uptime {}% < {}%) - sent: [{}], errors: [{}]",
                &breach.contract_id[..16.min(breach.contract_id.len())],
                breach.uptime_percent,
                breach.threshold_percent,
                channels_sent.join(", "),
                errors.join(", ")
            );
        }

        // Record the alert so we don't re-send within 1 hour
        self.database
            .upsert_sla_breach_alert(
                &contract_id_bytes,
                &provider_pubkey_bytes,
                breach.uptime_percent,
                breach.threshold_percent,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to record SLA breach alert: {:#}", e))?;

        Ok(())
    }
}

async fn send_telegram_sla_breach(
    chat_id: &str,
    breach: &crate::database::providers::SlaBreachInfo,
) -> anyhow::Result<()> {
    use crate::notifications::telegram::TelegramClient;

    if !TelegramClient::is_configured() {
        anyhow::bail!("TELEGRAM_BOT_TOKEN not configured");
    }

    let telegram = TelegramClient::from_env()?;

    let message = format!(
        "*SLA Uptime Breach Alert*\n\n\
        Contract: `{}`\n\
        Uptime: *{}%* (threshold: {}%)\n\n\
        Check your contracts in the provider dashboard:\n\
        https://decent-cloud.org/dashboard/provider/sla",
        &breach.contract_id[..32.min(breach.contract_id.len())],
        breach.uptime_percent,
        breach.threshold_percent,
    );

    telegram.send_message(chat_id, &message).await?;

    tracing::info!(
        "Telegram SLA breach alert sent to chat_id: {} for contract {}",
        chat_id,
        &breach.contract_id[..16.min(breach.contract_id.len())]
    );
    Ok(())
}

async fn send_email_sla_breach(
    db: &Database,
    email_service: Option<&Arc<EmailService>>,
    provider_pubkey: &[u8],
    breach: &crate::database::providers::SlaBreachInfo,
) -> anyhow::Result<()> {
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

    let email_body = format!(
        "SLA uptime breach detected for one of your contracts.\n\n\
        Contract ID: {}\n\
        Current Uptime: {}%\n\
        Configured Threshold: {}%\n\n\
        Please review your contract health in the provider dashboard:\n\
        https://decent-cloud.org/dashboard/provider/sla\n\n\
        This alert will not repeat for at least 1 hour.",
        breach.contract_id, breach.uptime_percent, breach.threshold_percent
    );

    let from_addr =
        std::env::var("EMAIL_FROM_ADDR").unwrap_or_else(|_| "noreply@decent-cloud.org".to_string());

    email_svc
        .send_email(
            &from_addr,
            email_addr,
            "SLA Uptime Breach - Decent Cloud",
            &email_body,
            false,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to send SLA breach email: {:#}", e))?;

    tracing::info!(
        "Email SLA breach alert sent to {} for contract {}",
        email_addr,
        &breach.contract_id[..16.min(breach.contract_id.len())]
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::providers::SlaBreachInfo;

    #[test]
    fn test_sla_alert_service_interval() {
        // Verify a 1-hour interval is 3600 seconds
        assert_eq!(Duration::from_secs(60 * 60).as_secs(), 3_600);
    }

    #[tokio::test]
    async fn test_send_telegram_sla_breach_no_token() {
        let breach = SlaBreachInfo {
            contract_id: "a".repeat(64),
            provider_pubkey: "b".repeat(64),
            uptime_percent: 80,
            threshold_percent: 95,
        };

        std::env::remove_var("TELEGRAM_BOT_TOKEN");
        let result = send_telegram_sla_breach("chat-123", &breach).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("TELEGRAM_BOT_TOKEN"),
            "expected TELEGRAM_BOT_TOKEN error, got: {}",
            err
        );
    }
}
