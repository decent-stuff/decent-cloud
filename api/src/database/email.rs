use super::types::Database;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Email type for retry policy configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailType {
    /// Critical: Account recovery emails - 24 attempts over ~8 hours
    Recovery,
    /// Important: Welcome emails - 12 attempts over ~4 hours
    Welcome,
    /// General: Other notifications - 6 attempts over ~2 hours
    General,
    /// Message notifications - 6 attempts over ~2 hours
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

    pub fn max_attempts(&self) -> i64 {
        match self {
            EmailType::Recovery => 24,           // Critical: many retries
            EmailType::Welcome => 12,            // Important: moderate retries
            EmailType::General => 6,             // Normal: fewer retries
            EmailType::MessageNotification => 6, // Normal: fewer retries
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
    pub async fn queue_email(
        &self,
        to_addr: &str,
        from_addr: &str,
        subject: &str,
        body: &str,
        is_html: bool,
        email_type: EmailType,
    ) -> Result<Vec<u8>> {
        let id = uuid::Uuid::new_v4().as_bytes().to_vec();
        let is_html_int = if is_html { 1 } else { 0 };
        let created_at = chrono::Utc::now().timestamp();
        let email_type_str = email_type.as_str();
        let max_attempts = email_type.max_attempts();

        sqlx::query!(
            r#"INSERT INTO email_queue
               (id, to_addr, from_addr, subject, body, is_html, email_type, status, attempts, max_attempts, created_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, 'pending', 0, ?, ?)"#,
            id,
            to_addr,
            from_addr,
            subject,
            body,
            is_html_int,
            email_type_str,
            max_attempts,
            created_at
        )
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    pub async fn get_pending_emails(&self, limit: i64) -> Result<Vec<EmailQueueEntry>> {
        let emails = sqlx::query_as!(
            EmailQueueEntry,
            r#"SELECT id, to_addr, from_addr, subject, body, is_html, email_type,
                      status, attempts, max_attempts, last_error, created_at, last_attempted_at, sent_at
               FROM email_queue
               WHERE status = 'pending' AND attempts < max_attempts
               ORDER BY created_at ASC
               LIMIT ?"#,
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
                      status, attempts, max_attempts, last_error, created_at, last_attempted_at, sent_at
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
        sqlx::query!(
            "UPDATE email_queue SET status = 'pending', attempts = 0, last_error = NULL WHERE id = ? AND status = 'failed'",
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Reset a single email for retry - reset attempts to 0, status to pending, clear last_error
    /// Returns true if an email was found and reset
    pub async fn reset_email_for_retry(&self, id: &[u8]) -> Result<bool> {
        let result = sqlx::query!(
            "UPDATE email_queue SET status = 'pending', attempts = 0, last_error = NULL WHERE id = ?",
            id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Reset all failed emails to pending status for bulk retry
    /// Returns the count of emails reset
    pub async fn retry_all_failed_emails(&self) -> Result<u64> {
        let result = sqlx::query!(
            "UPDATE email_queue SET status = 'pending', attempts = 0, last_error = NULL WHERE status = 'failed'"
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

        // Mark as permanently failed if max attempts reached
        sqlx::query!(
            "UPDATE email_queue SET status = 'failed' WHERE id = ? AND attempts >= max_attempts",
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
