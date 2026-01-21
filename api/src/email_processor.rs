use crate::database::email::{calculate_backoff_secs, EmailType};
use crate::database::Database;
use crate::email_service::{EmailService, EmailServiceExt};
use std::sync::Arc;
use std::time::Duration;

/// Background service for processing queued emails with retry logic
pub struct EmailProcessor {
    database: Arc<Database>,
    email_service: Arc<EmailService>,
    interval: Duration,
    batch_size: i64,
    frontend_url: String,
}

impl EmailProcessor {
    pub fn new(
        database: Arc<Database>,
        email_service: Arc<EmailService>,
        interval_secs: u64,
        batch_size: i64,
    ) -> Self {
        let frontend_url =
            std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:59010".to_string());

        Self {
            database,
            email_service,
            interval: Duration::from_secs(interval_secs),
            batch_size,
            frontend_url,
        }
    }

    /// Run the email processor indefinitely
    pub async fn run(self) {
        let mut interval = tokio::time::interval(self.interval);

        // Process immediately on startup
        self.run_all_processors().await;

        loop {
            interval.tick().await;
            self.run_all_processors().await;
        }
    }

    async fn run_all_processors(&self) {
        if let Err(e) = self.process_expired_emails().await {
            tracing::error!("Expired email processing failed: {:#}", e);
        }
        if let Err(e) = self.process_batch().await {
            tracing::error!("Email processing failed: {:#}", e);
        }
        if let Err(e) = self.process_user_notifications().await {
            tracing::error!("User notification processing failed: {:#}", e);
        }
        if let Err(e) = self.process_sla_breaches().await {
            tracing::error!("SLA breach processing failed: {:#}", e);
        }
        if let Err(e) = self.process_pending_stripe_receipts().await {
            tracing::error!("Pending Stripe receipt processing failed: {:#}", e);
        }
    }

    /// Mark emails that exceeded the 7-day window as permanently failed
    async fn process_expired_emails(&self) -> anyhow::Result<()> {
        let expired = self
            .database
            .get_expired_pending_emails(self.batch_size)
            .await?;

        if expired.is_empty() {
            return Ok(());
        }

        let mut failed_count = 0;
        for email in expired {
            self.database
                .mark_email_permanently_failed(&email.id)
                .await?;
            tracing::warn!(
                "Email {} PERMANENTLY FAILED after 7 days (type: {}, attempts: {})",
                hex::encode(&email.id),
                email.email_type,
                email.attempts
            );
            failed_count += 1;
        }

        if failed_count > 0 {
            tracing::info!(
                "Marked {} emails as permanently failed (7-day window expired)",
                failed_count
            );
        }

        Ok(())
    }

    async fn process_batch(&self) -> anyhow::Result<()> {
        let pending = self.database.get_pending_emails(self.batch_size).await?;

        if pending.is_empty() {
            tracing::debug!("No pending emails to process");
            return Ok(());
        }

        let now = chrono::Utc::now().timestamp();
        let mut sent_count = 0;
        let mut retry_count = 0;
        let mut skipped_count = 0;

        for email in pending {
            // Calculate backoff: immediate, 1m, 2m, 4m, 8m, 16m, 32m, then 1h intervals
            let backoff_seconds = calculate_backoff_secs(email.attempts);

            // Check if enough time has passed since last attempt
            if let Some(last_attempt) = email.last_attempted_at {
                let elapsed = now - last_attempt;
                if elapsed < backoff_seconds {
                    tracing::debug!(
                        "Skipping email {} (backoff: {}s remaining)",
                        hex::encode(&email.id),
                        backoff_seconds - elapsed
                    );
                    skipped_count += 1;
                    continue;
                }
            }

            // Attempt to send
            match self.email_service.send_queued_email(&email).await {
                Ok(()) => {
                    self.database.mark_email_sent(&email.id).await?;
                    tracing::info!(
                        "Sent email to {} (subject: {}, type: {})",
                        email.to_addr,
                        email.subject,
                        email.email_type
                    );
                    sent_count += 1;
                }
                Err(e) => {
                    let error_msg = format!("{:#}", e);
                    self.database
                        .mark_email_failed(&email.id, &error_msg)
                        .await?;

                    tracing::warn!(
                        "Email {} failed (attempt {}, type: {}): {}",
                        hex::encode(&email.id),
                        email.attempts + 1,
                        email.email_type,
                        error_msg
                    );
                    retry_count += 1;
                }
            }
        }

        if sent_count > 0 || retry_count > 0 || skipped_count > 0 {
            tracing::info!(
                "Email batch processed: {} sent, {} retrying, {} skipped (backoff)",
                sent_count,
                retry_count,
                skipped_count
            );
        }

        Ok(())
    }

    /// Process user notifications for failed email deliveries
    async fn process_user_notifications(&self) -> anyhow::Result<()> {
        // Notify users about first-time failures (will retry)
        let retry_notifications = self
            .database
            .get_emails_needing_retry_notification(self.batch_size)
            .await?;

        for email in retry_notifications {
            if let Some(ref account_id) = email.related_account_id {
                if let Err(e) = self.send_retry_notification(account_id, &email).await {
                    tracing::warn!(
                        "Failed to send retry notification for email {}: {}",
                        hex::encode(&email.id),
                        e
                    );
                    continue;
                }
            }
            self.database.mark_retry_notified(&email.id).await?;
        }

        // Notify users about permanent failures (gave up after 7 days)
        let gave_up_notifications = self
            .database
            .get_emails_needing_gave_up_notification(self.batch_size)
            .await?;

        for email in gave_up_notifications {
            if let Some(ref account_id) = email.related_account_id {
                if let Err(e) = self.send_gave_up_notification(account_id, &email).await {
                    tracing::warn!(
                        "Failed to send gave-up notification for email {}: {}",
                        hex::encode(&email.id),
                        e
                    );
                    continue;
                }
            }
            self.database.mark_gave_up_notified(&email.id).await?;
        }

        Ok(())
    }

    /// Send notification to user that email delivery failed but will be retried
    async fn send_retry_notification(
        &self,
        account_id: &[u8],
        failed_email: &crate::database::email::EmailQueueEntry,
    ) -> anyhow::Result<()> {
        let user_email = self.get_account_email(account_id).await?;
        let Some(user_email) = user_email else {
            tracing::debug!("No verified email for account, skipping retry notification");
            return Ok(());
        };

        let from_addr = std::env::var("SMTP_FROM_ADDR")
            .unwrap_or_else(|_| "noreply@decent-cloud.org".to_string());

        let subject = "Email delivery issue - we're retrying";
        let body = format!(
            r#"<html>
<body style="font-family: system-ui, -apple-system, sans-serif; line-height: 1.6; color: #333;">
    <div style="max-width: 600px; margin: 0 auto; padding: 20px;">
        <h2 style="color: #f59e0b;">Email Delivery Issue</h2>
        <p>We encountered an issue sending an email notification on your behalf.</p>

        <div style="background: #fef3c7; padding: 15px; border-radius: 8px; margin: 20px 0; border-left: 4px solid #f59e0b;">
            <p style="margin: 0;"><strong>Subject:</strong> {}</p>
            <p style="margin: 5px 0 0 0;"><strong>Recipient:</strong> {}</p>
        </div>

        <p>Don't worry - we'll keep trying to deliver this email for up to 7 days. No action is needed from you.</p>

        <p style="color: #6b7280; font-size: 12px; margin-top: 30px;">
            This is an automated notification from your account.
        </p>
    </div>
</body>
</html>"#,
            failed_email.subject, failed_email.to_addr
        );

        // Queue without related_account_id to avoid infinite loops
        self.database
            .queue_email(
                &user_email,
                &from_addr,
                subject,
                &body,
                true,
                EmailType::General,
            )
            .await?;

        tracing::info!(
            "Queued retry notification for account {} about email {}",
            hex::encode(account_id),
            hex::encode(&failed_email.id)
        );

        Ok(())
    }

    /// Send notification to user that we permanently gave up on email delivery
    async fn send_gave_up_notification(
        &self,
        account_id: &[u8],
        failed_email: &crate::database::email::EmailQueueEntry,
    ) -> anyhow::Result<()> {
        let user_email = self.get_account_email(account_id).await?;
        let Some(user_email) = user_email else {
            tracing::debug!("No verified email for account, skipping gave-up notification");
            return Ok(());
        };

        let from_addr = std::env::var("SMTP_FROM_ADDR")
            .unwrap_or_else(|_| "noreply@decent-cloud.org".to_string());

        let subject = "Email delivery failed permanently";
        let body = format!(
            r#"<html>
<body style="font-family: system-ui, -apple-system, sans-serif; line-height: 1.6; color: #333;">
    <div style="max-width: 600px; margin: 0 auto; padding: 20px;">
        <h2 style="color: #dc2626;">Email Delivery Failed</h2>
        <p>After trying for 7 days, we were unable to deliver an email notification on your behalf.</p>

        <div style="background: #fee2e2; padding: 15px; border-radius: 8px; margin: 20px 0; border-left: 4px solid #dc2626;">
            <p style="margin: 0;"><strong>Subject:</strong> {}</p>
            <p style="margin: 5px 0 0 0;"><strong>Recipient:</strong> {}</p>
            <p style="margin: 5px 0 0 0;"><strong>Last error:</strong> {}</p>
        </div>

        <p>The recipient may need to check their email address or spam settings. You may want to contact them through another channel.</p>

        <p style="color: #6b7280; font-size: 12px; margin-top: 30px;">
            This is an automated notification from your account.
        </p>
    </div>
</body>
</html>"#,
            failed_email.subject,
            failed_email.to_addr,
            failed_email
                .last_error
                .as_deref()
                .unwrap_or("Unknown error")
        );

        // Queue without related_account_id to avoid infinite loops
        self.database
            .queue_email(
                &user_email,
                &from_addr,
                subject,
                &body,
                true,
                EmailType::General,
            )
            .await?;

        tracing::info!(
            "Queued gave-up notification for account {} about email {}",
            hex::encode(account_id),
            hex::encode(&failed_email.id)
        );

        Ok(())
    }

    /// Process pending Stripe receipts - try to get Stripe invoice and send receipt
    async fn process_pending_stripe_receipts(&self) -> anyhow::Result<()> {
        let pending = self
            .database
            .get_pending_stripe_receipts(self.batch_size)
            .await?;

        if pending.is_empty() {
            return Ok(());
        }

        let stripe_client = match crate::stripe_client::StripeClient::new() {
            Ok(c) => Some(c),
            Err(e) => {
                tracing::warn!(
                    "Stripe not configured, will use Typst for pending receipts: {}",
                    e
                );
                None
            }
        };

        for receipt in pending {
            let contract_id_hex = hex::encode(&receipt.contract_id);

            // Check if receipt was already sent (e.g., via invoice.paid webhook)
            if self
                .database
                .cancel_pending_stripe_receipt_if_sent(&receipt.contract_id)
                .await?
            {
                tracing::debug!(
                    "Pending receipt for contract {} already sent, removed from queue",
                    contract_id_hex
                );
                continue;
            }

            // Try to get Stripe invoice
            let has_stripe_invoice = if let Some(ref client) = stripe_client {
                match client.find_invoice_by_contract_id(&contract_id_hex).await {
                    Ok(Some(invoice_id)) => {
                        // Update contract with invoice ID
                        if let Err(e) = self
                            .database
                            .update_stripe_invoice_id(&receipt.contract_id, &invoice_id)
                            .await
                        {
                            tracing::warn!(
                                "Failed to update stripe_invoice_id for contract {}: {}",
                                contract_id_hex,
                                e
                            );
                        } else {
                            tracing::info!(
                                "Found Stripe invoice {} for contract {}",
                                invoice_id,
                                contract_id_hex
                            );
                        }
                        true
                    }
                    Ok(None) => {
                        tracing::debug!(
                            "No Stripe invoice yet for contract {} (attempt {})",
                            contract_id_hex,
                            receipt.attempts + 1
                        );
                        false
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to search for Stripe invoice for contract {}: {}",
                            contract_id_hex,
                            e
                        );
                        false
                    }
                }
            } else {
                false
            };

            // If we have Stripe invoice or max attempts reached, send receipt
            let should_send = has_stripe_invoice || receipt.attempts >= 4; // 5th attempt = max

            if should_send {
                // Send receipt (will use Stripe PDF if available, otherwise Typst)
                match crate::receipts::send_payment_receipt(
                    self.database.as_ref(),
                    &receipt.contract_id,
                    Some(&self.email_service),
                )
                .await
                {
                    Ok(0) => {
                        tracing::debug!(
                            "Receipt already sent for contract {}, removing from pending",
                            contract_id_hex
                        );
                    }
                    Ok(receipt_num) => {
                        if has_stripe_invoice {
                            tracing::info!(
                                "Sent receipt #{} with Stripe invoice for contract {}",
                                receipt_num,
                                contract_id_hex
                            );
                        } else {
                            tracing::info!(
                                "Sent receipt #{} with Typst invoice for contract {} (max attempts reached)",
                                receipt_num,
                                contract_id_hex
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to send receipt for contract {}: {}",
                            contract_id_hex,
                            e
                        );
                    }
                }

                // Remove from pending queue
                self.database
                    .remove_pending_stripe_receipt(&receipt.contract_id)
                    .await?;
            } else {
                // Schedule next retry
                if !self
                    .database
                    .update_pending_stripe_receipt_retry(&receipt.contract_id)
                    .await?
                {
                    // Max attempts reached but update failed - remove anyway
                    self.database
                        .remove_pending_stripe_receipt(&receipt.contract_id)
                        .await?;
                }
            }
        }

        Ok(())
    }

    /// Process SLA breaches and send alert emails to providers
    async fn process_sla_breaches(&self) -> anyhow::Result<()> {
        let breaches = self.database.get_sla_breaches().await?;

        if breaches.is_empty() {
            tracing::debug!("No SLA breaches to process");
            return Ok(());
        }

        let mut alerted_count = 0;

        for breach in breaches {
            // Mark as breached first
            self.database.mark_sla_breached(breach.message_id).await?;

            // Get provider email
            let provider_email = match self
                .database
                .get_account_with_keys_by_public_key(&breach.provider_pubkey)
                .await?
            {
                Some(acc) if acc.email_verified && acc.email.is_some() => acc.email.unwrap(),
                _ => {
                    tracing::debug!(
                        "No verified email for provider {}, skipping SLA alert",
                        hex::encode(&breach.provider_pubkey)
                    );
                    self.database.mark_sla_alert_sent(breach.message_id).await?;
                    continue;
                }
            };

            // Calculate how long the customer has been waiting
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            let wait_hours = (now - breach.created_at) / 3600;

            // Queue SLA alert email
            let from_addr = std::env::var("SMTP_FROM_ADDR")
                .unwrap_or_else(|_| "noreply@decent-cloud.org".to_string());

            let subject = "Action Required: Customer awaiting response";
            let contract_url = format!(
                "{}/dashboard/rentals/{}",
                self.frontend_url, breach.contract_id
            );
            let support_url =
                std::env::var("CHATWOOT_FRONTEND_URL").expect("CHATWOOT_FRONTEND_URL must be set");

            let body = format!(
                r#"<html>
<body style="font-family: system-ui, -apple-system, sans-serif; line-height: 1.6; color: #333;">
    <div style="max-width: 600px; margin: 0 auto; padding: 20px;">
        <h2 style="color: #dc2626;">Response Time Alert</h2>
        <p>A customer has been waiting for your response for over <strong>{} hours</strong>.</p>

        <div style="background: #fee2e2; padding: 15px; border-radius: 8px; margin: 20px 0; border-left: 4px solid #dc2626;">
            <p style="margin: 0;"><strong>Contract ID:</strong> {}</p>
            <p style="margin: 5px 0 0 0;"><strong>Waiting since:</strong> {} hours ago</p>
        </div>

        <p>Quick response times build customer trust and improve your provider rating.</p>

        <p>
            <a href="{}" style="display: inline-block; background: #2563eb; color: white; padding: 12px 24px; text-decoration: none; border-radius: 6px; font-weight: 500; margin-right: 10px;">
                View in Dashboard
            </a>
            <a href="{}" style="display: inline-block; background: #059669; color: white; padding: 12px 24px; text-decoration: none; border-radius: 6px; font-weight: 500;">
                Open Support Portal
            </a>
        </p>

        <p style="color: #6b7280; font-size: 12px; margin-top: 30px;">
            You're receiving this alert because your SLA response time threshold has been exceeded.
        </p>
    </div>
</body>
</html>"#,
                wait_hours, &breach.contract_id, wait_hours, contract_url, support_url
            );

            match self
                .database
                .queue_email(
                    &provider_email,
                    &from_addr,
                    subject,
                    &body,
                    true,
                    EmailType::General,
                )
                .await
            {
                Ok(_) => {
                    self.database.mark_sla_alert_sent(breach.message_id).await?;
                    tracing::info!(
                        "Queued SLA breach alert to provider {} for contract {}",
                        hex::encode(&breach.provider_pubkey),
                        &breach.contract_id
                    );
                    alerted_count += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to queue SLA alert for contract {}: {}",
                        &breach.contract_id,
                        e
                    );
                }
            }
        }

        if alerted_count > 0 {
            tracing::info!("Processed {} SLA breach alerts", alerted_count);
        }

        Ok(())
    }

    /// Get verified email for an account
    async fn get_account_email(&self, account_id: &[u8]) -> anyhow::Result<Option<String>> {
        let account = match self.database.get_account(account_id).await? {
            Some(acc) => acc,
            None => return Ok(None),
        };

        // Return email only if verified
        if account.email_verified {
            Ok(account.email)
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests;
