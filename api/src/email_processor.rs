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
            tracing::error!("Expired email processing failed: {}", e);
        }
        if let Err(e) = self.process_batch().await {
            tracing::error!("Email processing failed: {}", e);
        }
        if let Err(e) = self.process_message_notifications().await {
            tracing::error!("Message notification processing failed: {}", e);
        }
        if let Err(e) = self.process_user_notifications().await {
            tracing::error!("User notification processing failed: {}", e);
        }
        if let Err(e) = self.process_sla_breaches().await {
            tracing::error!("SLA breach processing failed: {}", e);
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

    /// Process pending message notifications
    async fn process_message_notifications(&self) -> anyhow::Result<()> {
        let pending = self
            .database
            .get_pending_message_notifications(self.batch_size)
            .await?;

        if pending.is_empty() {
            tracing::debug!("No pending message notifications to process");
            return Ok(());
        }

        let mut sent_count = 0;
        let mut skipped_count = 0;

        for notification in pending {
            let recipient_pubkey_hex = &notification.recipient_pubkey;

            // Check if message is already read - skip if so
            match self
                .database
                .is_message_read(&notification.message_id, recipient_pubkey_hex)
                .await
            {
                Ok(true) => {
                    // Message already read, skip notification
                    if let Err(e) = self
                        .database
                        .mark_notification_skipped(&notification.id)
                        .await
                    {
                        tracing::warn!(
                            "Failed to mark notification {} as skipped: {}",
                            hex::encode(&notification.id),
                            e
                        );
                    }
                    skipped_count += 1;
                    continue;
                }
                Ok(false) => {
                    // Message not read, continue processing
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to check read status for message {}: {}",
                        hex::encode(&notification.message_id),
                        e
                    );
                    continue;
                }
            }

            // Get recipient email address
            let recipient_email = match self.get_recipient_email(recipient_pubkey_hex).await {
                Ok(Some(email)) => email,
                Ok(None) => {
                    tracing::debug!(
                        "No verified email for recipient {}, skipping notification",
                        recipient_pubkey_hex
                    );
                    if let Err(e) = self
                        .database
                        .mark_notification_skipped(&notification.id)
                        .await
                    {
                        tracing::warn!(
                            "Failed to mark notification {} as skipped: {}",
                            hex::encode(&notification.id),
                            e
                        );
                    }
                    skipped_count += 1;
                    continue;
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to get email for recipient {}: {}",
                        recipient_pubkey_hex,
                        e
                    );
                    continue;
                }
            };

            // Get message details for email content
            let (contract_id, sender_pubkey, message_body) =
                match self.get_message_details(&notification.message_id).await {
                    Ok(details) => details,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to get message details for {}: {}",
                            hex::encode(&notification.message_id),
                            e
                        );
                        continue;
                    }
                };

            // Get sender's account ID for failure notifications
            let sender_account_id = match self.get_account_id_by_pubkey(&sender_pubkey).await {
                Ok(id) => id,
                Err(e) => {
                    tracing::debug!("Could not get sender account ID: {}", e);
                    None
                }
            };

            // Generate email content
            let subject = "New message in your rental contract";
            let message_preview = if message_body.len() > 200 {
                format!("{}...", &message_body[..200])
            } else {
                message_body.clone()
            };

            let contract_id_hex = hex::encode(&contract_id);
            let view_url = format!(
                "{}/dashboard/rentals/{}/messages",
                self.frontend_url, contract_id_hex
            );

            let body = format!(
                r#"<html>
<body style="font-family: system-ui, -apple-system, sans-serif; line-height: 1.6; color: #333;">
    <div style="max-width: 600px; margin: 0 auto; padding: 20px;">
        <h2 style="color: #2563eb;">New Message</h2>
        <p>You have received a new message regarding your rental contract.</p>

        <div style="background: #f3f4f6; padding: 15px; border-radius: 8px; margin: 20px 0;">
            <p style="margin: 0; color: #6b7280; font-size: 14px;">Message preview:</p>
            <p style="margin: 10px 0 0 0;">{}</p>
        </div>

        <p>
            <a href="{}" style="display: inline-block; background: #2563eb; color: white; padding: 12px 24px; text-decoration: none; border-radius: 6px; font-weight: 500;">
                View Full Message
            </a>
        </p>

        <p style="color: #6b7280; font-size: 12px; margin-top: 30px;">
            You're receiving this email because you're part of a rental contract.
            To stop receiving these notifications, please contact support.
        </p>
    </div>
</body>
</html>"#,
                message_preview, view_url
            );

            // Queue email with sender's account for failure notifications
            let from_addr = std::env::var("SMTP_FROM_ADDR")
                .unwrap_or_else(|_| "noreply@decloud.org".to_string());

            match self
                .database
                .queue_email_with_account(
                    &recipient_email,
                    &from_addr,
                    subject,
                    &body,
                    true,
                    EmailType::MessageNotification,
                    sender_account_id.as_deref(),
                )
                .await
            {
                Ok(_) => {
                    // Mark notification as sent
                    if let Err(e) = self.database.mark_notification_sent(&notification.id).await {
                        tracing::warn!(
                            "Failed to mark notification {} as sent: {}",
                            hex::encode(&notification.id),
                            e
                        );
                    } else {
                        tracing::info!(
                            "Queued message notification email to {} for message {}",
                            recipient_email,
                            hex::encode(&notification.message_id)
                        );
                        sent_count += 1;
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to queue email for notification {}: {}",
                        hex::encode(&notification.id),
                        e
                    );
                }
            }
        }

        if sent_count > 0 || skipped_count > 0 {
            tracing::info!(
                "Message notifications processed: {} queued, {} skipped",
                sent_count,
                skipped_count
            );
        }

        Ok(())
    }

    /// Get verified email address for a recipient pubkey
    async fn get_recipient_email(&self, pubkey_hex: &str) -> anyhow::Result<Option<String>> {
        // Decode pubkey
        let pubkey = hex::decode(pubkey_hex)?;

        // Get account by pubkey
        let account = match self
            .database
            .get_account_with_keys_by_public_key(&pubkey)
            .await?
        {
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

    /// Get message details for email content (contract_id, sender_name, message_body)
    async fn get_message_details(
        &self,
        message_id: &[u8],
    ) -> anyhow::Result<(Vec<u8>, String, String)> {
        // Get message with thread info
        let message = sqlx::query!(
            r#"SELECT m.body, m.sender_pubkey, m.thread_id, mt.contract_id
               FROM messages m
               JOIN message_threads mt ON m.thread_id = mt.id
               WHERE m.id = ?"#,
            message_id
        )
        .fetch_one(&self.database.pool)
        .await?;

        let sender_name = message.sender_pubkey;
        let body = message.body;
        let contract_id = message.contract_id;

        Ok((contract_id, sender_name, body))
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

        let from_addr =
            std::env::var("SMTP_FROM_ADDR").unwrap_or_else(|_| "noreply@decloud.org".to_string());

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

        let from_addr =
            std::env::var("SMTP_FROM_ADDR").unwrap_or_else(|_| "noreply@decloud.org".to_string());

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
                .unwrap_or_else(|_| "noreply@decloud.org".to_string());

            let subject = "Action Required: Customer awaiting response";
            let contract_url = format!(
                "{}/dashboard/rentals/{}/messages",
                self.frontend_url, breach.contract_id
            );
            let support_url = std::env::var("CHATWOOT_FRONTEND_URL")
                .expect("CHATWOOT_FRONTEND_URL must be set");

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
        if account.email_verified != 0 {
            Ok(account.email)
        } else {
            Ok(None)
        }
    }

    /// Get account ID from pubkey hex string
    async fn get_account_id_by_pubkey(&self, pubkey_hex: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let pubkey = hex::decode(pubkey_hex)?;
        self.database.get_account_id_by_public_key(&pubkey).await
    }
}

#[cfg(test)]
mod tests;
