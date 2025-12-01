use super::types::Database;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Maximum time window for email retries: 7 days in seconds
pub const EMAIL_RETRY_WINDOW_SECS: i64 = 7 * 24 * 60 * 60;

/// Calculate backoff delay for a given attempt number.
/// Schedule: immediate, 1min, 2min, 4min, 8min, 16min, 32min, then 1h intervals
pub fn calculate_backoff_secs(attempts: i64) -> i64 {
    match attempts {
        0 => 0,       // immediate
        1 => 60,      // 1 min
        2 => 2 * 60,  // 2 min
        3 => 4 * 60,  // 4 min
        4 => 8 * 60,  // 8 min
        5 => 16 * 60, // 16 min
        6 => 32 * 60, // 32 min
        _ => 60 * 60, // 1 hour for all subsequent attempts
    }
}

/// Email type categorization (all types now retry for 7 days)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailType {
    /// Account recovery emails
    Recovery,
    /// Welcome emails
    Welcome,
    /// General notifications
    General,
    /// Message notifications (tracks sender for failure notification)
    MessageNotification,
}

impl EmailType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EmailType::Recovery => "recovery",
            EmailType::Welcome => "welcome",
            EmailType::General => "general",
            EmailType::MessageNotification => "message_notification",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, poem_openapi::Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct EmailQueueEntry {
    #[oai(skip)]
    #[serde(skip)]
    pub id: Vec<u8>,
    pub to_addr: String,
    pub from_addr: String,
    pub subject: String,
    pub body: String,
    pub is_html: i64,
    pub email_type: String,
    pub status: String,
    pub attempts: i64,
    pub max_attempts: i64,
    pub last_error: Option<String>,
    pub created_at: i64,
    pub last_attempted_at: Option<i64>,
    pub sent_at: Option<i64>,
    /// Account to notify on failure (e.g., message sender)
    #[oai(skip)]
    #[serde(skip)]
    pub related_account_id: Option<Vec<u8>>,
    /// Whether we've notified the user about retry
    pub user_notified_retry: i64,
    /// Whether we've notified the user about permanent failure
    pub user_notified_gave_up: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, poem_openapi::Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct EmailStats {
    pub pending: i64,
    pub sent: i64,
    pub failed: i64,
    pub total: i64,
}

impl Database {
    /// Queue an email for delivery. All emails retry for 7 days before permanent failure.
    /// `related_account_id` is optional - used to notify the account owner on failure.
    pub async fn queue_email(
        &self,
        to_addr: &str,
        from_addr: &str,
        subject: &str,
        body: &str,
        is_html: bool,
        email_type: EmailType,
    ) -> Result<Vec<u8>> {
        self.queue_email_with_account(to_addr, from_addr, subject, body, is_html, email_type, None)
            .await
    }

    /// Queue an email with an associated account for failure notifications.
    #[allow(clippy::too_many_arguments)]
    pub async fn queue_email_with_account(
        &self,
        to_addr: &str,
        from_addr: &str,
        subject: &str,
        body: &str,
        is_html: bool,
        email_type: EmailType,
        related_account_id: Option<&[u8]>,
    ) -> Result<Vec<u8>> {
        let id = uuid::Uuid::new_v4().as_bytes().to_vec();
        let is_html_int = if is_html { 1 } else { 0 };
        let created_at = chrono::Utc::now().timestamp();
        let email_type_str = email_type.as_str();

        sqlx::query!(
            r#"INSERT INTO email_queue
               (id, to_addr, from_addr, subject, body, is_html, email_type, status, attempts, max_attempts, created_at, related_account_id)
               VALUES (?, ?, ?, ?, ?, ?, ?, 'pending', 0, 999, ?, ?)"#,
            id,
            to_addr,
            from_addr,
            subject,
            body,
            is_html_int,
            email_type_str,
            created_at,
            related_account_id
        )
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// Get pending emails that are still within the 7-day retry window
    pub async fn get_pending_emails(&self, limit: i64) -> Result<Vec<EmailQueueEntry>> {
        let cutoff = chrono::Utc::now().timestamp() - EMAIL_RETRY_WINDOW_SECS;

        let emails = sqlx::query_as!(
            EmailQueueEntry,
            r#"SELECT id, to_addr, from_addr, subject, body, is_html, email_type,
                      status, attempts, max_attempts, last_error, created_at, last_attempted_at, sent_at,
                      related_account_id, user_notified_retry, user_notified_gave_up
               FROM email_queue
               WHERE status = 'pending' AND created_at >= ?
               ORDER BY created_at ASC
               LIMIT ?"#,
            cutoff,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(emails)
    }

    /// Get failed emails for admin review
    pub async fn get_failed_emails(&self, limit: i64) -> Result<Vec<EmailQueueEntry>> {
        let emails = sqlx::query_as!(
            EmailQueueEntry,
            r#"SELECT id, to_addr, from_addr, subject, body, is_html, email_type,
                      status, attempts, max_attempts, last_error, created_at, last_attempted_at, sent_at,
                      related_account_id, user_notified_retry, user_notified_gave_up
               FROM email_queue
               WHERE status = 'failed'
               ORDER BY created_at DESC
               LIMIT ?"#,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(emails)
    }

    /// Retry a failed email by resetting its status and attempts
    pub async fn retry_failed_email(&self, id: &[u8]) -> Result<()> {
        // Reset status, attempts, and also update created_at to restart the 7-day window
        let now = chrono::Utc::now().timestamp();
        sqlx::query!(
            "UPDATE email_queue SET status = 'pending', attempts = 0, last_error = NULL, created_at = ?, user_notified_retry = 0, user_notified_gave_up = 0 WHERE id = ? AND status = 'failed'",
            now,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Reset a single email for retry - reset attempts to 0, status to pending, clear last_error
    /// Also resets created_at to restart the 7-day window
    /// Returns true if an email was found and reset
    pub async fn reset_email_for_retry(&self, id: &[u8]) -> Result<bool> {
        let now = chrono::Utc::now().timestamp();
        let result = sqlx::query!(
            "UPDATE email_queue SET status = 'pending', attempts = 0, last_error = NULL, created_at = ?, user_notified_retry = 0, user_notified_gave_up = 0 WHERE id = ?",
            now,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Reset all failed emails to pending status for bulk retry
    /// Returns the count of emails reset
    pub async fn retry_all_failed_emails(&self) -> Result<u64> {
        let now = chrono::Utc::now().timestamp();
        let result = sqlx::query!(
            "UPDATE email_queue SET status = 'pending', attempts = 0, last_error = NULL, created_at = ?, user_notified_retry = 0, user_notified_gave_up = 0 WHERE status = 'failed'",
            now
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Get email queue statistics
    pub async fn get_email_stats(&self) -> Result<EmailStats> {
        let pending =
            sqlx::query_scalar!("SELECT COUNT(*) FROM email_queue WHERE status = 'pending'")
                .fetch_one(&self.pool)
                .await?;

        let sent = sqlx::query_scalar!("SELECT COUNT(*) FROM email_queue WHERE status = 'sent'")
            .fetch_one(&self.pool)
            .await?;

        let failed =
            sqlx::query_scalar!("SELECT COUNT(*) FROM email_queue WHERE status = 'failed'")
                .fetch_one(&self.pool)
                .await?;

        let total = sqlx::query_scalar!("SELECT COUNT(*) FROM email_queue")
            .fetch_one(&self.pool)
            .await?;

        Ok(EmailStats {
            pending,
            sent,
            failed,
            total,
        })
    }

    pub async fn mark_email_sent(&self, id: &[u8]) -> Result<()> {
        let sent_at = chrono::Utc::now().timestamp();

        sqlx::query!(
            "UPDATE email_queue SET status = 'sent', sent_at = ? WHERE id = ?",
            sent_at,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark an email attempt as failed (increments attempts, stores error)
    /// Does NOT mark as permanently failed - that's handled by time-based expiration
    pub async fn mark_email_failed(&self, id: &[u8], error: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        sqlx::query!(
            "UPDATE email_queue SET attempts = attempts + 1, last_error = ?, last_attempted_at = ? WHERE id = ?",
            error,
            now,
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark an email as permanently failed (7-day window expired)
    pub async fn mark_email_permanently_failed(&self, id: &[u8]) -> Result<()> {
        sqlx::query!(
            "UPDATE email_queue SET status = 'failed' WHERE id = ? AND status = 'pending'",
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get pending emails that have exceeded the 7-day retry window
    pub async fn get_expired_pending_emails(&self, limit: i64) -> Result<Vec<EmailQueueEntry>> {
        let cutoff = chrono::Utc::now().timestamp() - EMAIL_RETRY_WINDOW_SECS;

        let emails = sqlx::query_as!(
            EmailQueueEntry,
            r#"SELECT id, to_addr, from_addr, subject, body, is_html, email_type,
                      status, attempts, max_attempts, last_error, created_at, last_attempted_at, sent_at,
                      related_account_id, user_notified_retry, user_notified_gave_up
               FROM email_queue
               WHERE status = 'pending' AND created_at < ?
               ORDER BY created_at ASC
               LIMIT ?"#,
            cutoff,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(emails)
    }

    /// Get emails that failed at least once, have a related account, and haven't been notified about retry
    pub async fn get_emails_needing_retry_notification(
        &self,
        limit: i64,
    ) -> Result<Vec<EmailQueueEntry>> {
        let emails = sqlx::query_as!(
            EmailQueueEntry,
            r#"SELECT id, to_addr, from_addr, subject, body, is_html, email_type,
                      status, attempts, max_attempts, last_error, created_at, last_attempted_at, sent_at,
                      related_account_id, user_notified_retry, user_notified_gave_up
               FROM email_queue
               WHERE status = 'pending' AND attempts > 0 AND related_account_id IS NOT NULL AND user_notified_retry = 0
               ORDER BY last_attempted_at ASC
               LIMIT ?"#,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(emails)
    }

    /// Get emails that permanently failed, have a related account, and haven't been notified
    pub async fn get_emails_needing_gave_up_notification(
        &self,
        limit: i64,
    ) -> Result<Vec<EmailQueueEntry>> {
        let emails = sqlx::query_as!(
            EmailQueueEntry,
            r#"SELECT id, to_addr, from_addr, subject, body, is_html, email_type,
                      status, attempts, max_attempts, last_error, created_at, last_attempted_at, sent_at,
                      related_account_id, user_notified_retry, user_notified_gave_up
               FROM email_queue
               WHERE status = 'failed' AND related_account_id IS NOT NULL AND user_notified_gave_up = 0
               ORDER BY last_attempted_at ASC
               LIMIT ?"#,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(emails)
    }

    /// Mark that we've notified the user about email retry
    pub async fn mark_retry_notified(&self, id: &[u8]) -> Result<()> {
        sqlx::query!(
            "UPDATE email_queue SET user_notified_retry = 1 WHERE id = ?",
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark that we've notified the user about permanent failure
    pub async fn mark_gave_up_notified(&self, id: &[u8]) -> Result<()> {
        sqlx::query!(
            "UPDATE email_queue SET user_notified_gave_up = 1 WHERE id = ?",
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Queue an email safely - never fails, only logs errors
    /// Returns true if email was queued, false if skipped or failed
    pub async fn queue_email_safe(
        &self,
        to_addr: Option<&str>,
        from_addr: &str,
        subject: &str,
        body: &str,
        is_html: bool,
        email_type: EmailType,
    ) -> bool {
        let Some(to) = to_addr else {
            tracing::debug!("Email not queued: no recipient address provided");
            return false;
        };

        match self
            .queue_email(to, from_addr, subject, body, is_html, email_type)
            .await
        {
            Ok(id) => {
                tracing::info!(
                    "Email queued: id={} to={} subject={} type={}",
                    hex::encode(&id),
                    to,
                    subject,
                    email_type.as_str()
                );
                true
            }
            Err(e) => {
                tracing::warn!("Failed to queue email to {}: {:#}", to, e);
                false
            }
        }
    }
}

#[cfg(test)]
mod tests;
