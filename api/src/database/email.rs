use super::types::Database;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EmailQueueEntry {
    pub id: Vec<u8>,
    pub to_addr: String,
    pub from_addr: String,
    pub subject: String,
    pub body: String,
    pub is_html: i64,
    pub status: String,
    pub attempts: i64,
    pub max_attempts: i64,
    pub last_error: Option<String>,
    pub created_at: i64,
    pub last_attempted_at: Option<i64>,
    pub sent_at: Option<i64>,
}

impl Database {
    pub async fn queue_email(
        &self,
        to_addr: &str,
        from_addr: &str,
        subject: &str,
        body: &str,
        is_html: bool,
    ) -> Result<Vec<u8>> {
        let id = uuid::Uuid::new_v4().as_bytes().to_vec();
        let is_html_int = if is_html { 1 } else { 0 };
        let created_at = chrono::Utc::now().timestamp();

        sqlx::query!(
            r#"INSERT INTO email_queue
               (id, to_addr, from_addr, subject, body, is_html, status, attempts, max_attempts, created_at)
               VALUES (?, ?, ?, ?, ?, ?, 'pending', 0, 3, ?)"#,
            id,
            to_addr,
            from_addr,
            subject,
            body,
            is_html_int,
            created_at
        )
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    pub async fn get_pending_emails(&self, limit: i64) -> Result<Vec<EmailQueueEntry>> {
        let emails = sqlx::query_as!(
            EmailQueueEntry,
            r#"SELECT id, to_addr, from_addr, subject, body, is_html,
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

    pub async fn get_email_by_id(&self, id: &[u8]) -> Result<Option<EmailQueueEntry>> {
        let email = sqlx::query_as!(
            EmailQueueEntry,
            r#"SELECT id, to_addr, from_addr, subject, body, is_html,
                      status, attempts, max_attempts, last_error, created_at, last_attempted_at, sent_at
               FROM email_queue
               WHERE id = ?"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(email)
    }
}

#[cfg(test)]
mod tests;
