use crate::database::Database;
use crate::email_service::EmailService;
use std::sync::Arc;
use std::time::Duration;

/// Background service for processing queued emails with retry logic
pub struct EmailProcessor {
    database: Arc<Database>,
    email_service: Arc<EmailService>,
    interval: Duration,
    batch_size: i64,
}

impl EmailProcessor {
    pub fn new(
        database: Arc<Database>,
        email_service: Arc<EmailService>,
        interval_secs: u64,
        batch_size: i64,
    ) -> Self {
        Self {
            database,
            email_service,
            interval: Duration::from_secs(interval_secs),
            batch_size,
        }
    }

    /// Run the email processor indefinitely
    pub async fn run(self) {
        let mut interval = tokio::time::interval(self.interval);

        // Process immediately on startup
        if let Err(e) = self.process_batch().await {
            tracing::error!("Initial email processing failed: {}", e);
        }

        loop {
            interval.tick().await;
            if let Err(e) = self.process_batch().await {
                tracing::error!("Email processing failed: {}", e);
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
            let backoff_seconds = if email.attempts > 0 {
                2_i64.pow(email.attempts as u32) * 60
            } else {
                0
            };

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
                        "Sent email to {} (subject: {})",
                        email.to_addr,
                        email.subject
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
                            "Email {} permanently failed after {} attempts: {}",
                            hex::encode(&email.id),
                            email.attempts + 1,
                            error_msg
                        );
                    } else {
                        tracing::warn!(
                            "Email {} failed (attempt {}/{}): {}",
                            hex::encode(&email.id),
                            email.attempts + 1,
                            email.max_attempts,
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
}

#[cfg(test)]
mod tests;
