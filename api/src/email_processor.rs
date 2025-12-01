use crate::database::email::EmailType;
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
        if let Err(e) = self.process_batch().await {
            tracing::error!("Initial email processing failed: {}", e);
        }
        if let Err(e) = self.process_message_notifications().await {
            tracing::error!("Initial message notification processing failed: {}", e);
        }

        loop {
            interval.tick().await;
            if let Err(e) = self.process_batch().await {
                tracing::error!("Email processing failed: {}", e);
            }
            if let Err(e) = self.process_message_notifications().await {
                tracing::error!("Message notification processing failed: {}", e);
            }
        }
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
            // Calculate exponential backoff delay: 2^attempts minutes
            let backoff_seconds = (if email.attempts > 0 {
                2_i64.pow(email.attempts as u32) * 60
            } else {
                0
            })
            .min(20 * 60); // Max 20 minutes

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

                    if email.attempts + 1 >= email.max_attempts {
                        tracing::error!(
                            "Email {} PERMANENTLY FAILED after {} attempts (type: {}): {}",
                            hex::encode(&email.id),
                            email.attempts + 1,
                            email.email_type,
                            error_msg
                        );
                    } else {
                        tracing::warn!(
                            "Email {} failed (attempt {}/{}, type: {}): {}",
                            hex::encode(&email.id),
                            email.attempts + 1,
                            email.max_attempts,
                            email.email_type,
                            error_msg
                        );
                        retry_count += 1;
                    }
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
            let (contract_id, _sender_name, message_body) =
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

            // Queue email
            let from_addr = std::env::var("SMTP_FROM_ADDR")
                .unwrap_or_else(|_| "noreply@decloud.org".to_string());

            match self
                .database
                .queue_email(
                    &recipient_email,
                    &from_addr,
                    subject,
                    &body,
                    true,
                    EmailType::MessageNotification,
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

        // Get account ID from pubkey
        let account_id = match self.database.get_account_id_by_public_key(&pubkey).await? {
            Some(id) => id,
            None => return Ok(None),
        };

        // Get account contacts
        let contacts = self.database.get_account_contacts(&account_id).await?;

        // Find first verified email
        for contact in contacts {
            if contact.contact_type == "email" && contact.verified {
                return Ok(Some(contact.contact_value));
            }
        }

        Ok(None)
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
}

#[cfg(test)]
mod tests;
