use super::types::Database;
use anyhow::Result;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(skip_serializing_if_is_none)]
pub struct MessageThread {
    #[ts(type = "string")]
    #[serde(skip)]
    #[oai(skip)]
    pub id: Vec<u8>,
    #[ts(type = "string")]
    pub thread_id: String,
    #[ts(type = "string")]
    pub contract_id: String,
    pub subject: String,
    #[ts(type = "number")]
    pub created_at_ns: i64,
    #[ts(type = "number")]
    pub last_message_at_ns: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(skip_serializing_if_is_none)]
pub struct Message {
    #[ts(type = "string")]
    #[serde(skip)]
    #[oai(skip)]
    pub id: Vec<u8>,
    #[ts(type = "string")]
    pub message_id: String,
    #[ts(type = "string")]
    pub thread_id: String,
    pub sender_pubkey: String,
    pub sender_role: String,
    pub body: String,
    #[ts(type = "number")]
    pub created_at_ns: i64,
    #[ts(type = "boolean")]
    pub is_read: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MessageThreadParticipant {
    pub thread_id: Vec<u8>,
    pub pubkey: String,
    pub role: String,
    pub joined_at_ns: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MessageNotification {
    pub id: Vec<u8>,
    pub message_id: Vec<u8>,
    pub recipient_pubkey: String,
    pub status: String,
    pub created_at_ns: i64,
    pub sent_at_ns: Option<i64>,
}

impl Database {
    /// Create a new message thread for a contract
    pub async fn create_thread(
        &self,
        contract_id: &[u8],
        subject: &str,
        requester_pubkey: &str,
        provider_pubkey: &str,
    ) -> Result<Vec<u8>> {
        let thread_id = uuid::Uuid::new_v4().as_bytes().to_vec();
        let created_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        let mut tx = self.pool.begin().await?;

        // Create thread
        sqlx::query!(
            "INSERT INTO message_threads (id, contract_id, subject, created_at_ns, last_message_at_ns, status) VALUES (?, ?, ?, ?, ?, 'open')",
            thread_id,
            contract_id,
            subject,
            created_at_ns,
            created_at_ns
        )
        .execute(&mut *tx)
        .await?;

        // Add participants
        sqlx::query!(
            "INSERT INTO message_thread_participants (thread_id, pubkey, role, joined_at_ns) VALUES (?, ?, 'requester', ?)",
            thread_id,
            requester_pubkey,
            created_at_ns
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "INSERT INTO message_thread_participants (thread_id, pubkey, role, joined_at_ns) VALUES (?, ?, 'provider', ?)",
            thread_id,
            provider_pubkey,
            created_at_ns
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(thread_id)
    }

    /// Get thread by contract ID
    pub async fn get_thread_by_contract(
        &self,
        contract_id: &[u8],
    ) -> Result<Option<MessageThread>> {
        let thread = sqlx::query!(
            r#"SELECT id, contract_id, subject, created_at_ns as "created_at_ns!", last_message_at_ns as "last_message_at_ns!", status as "status!"
               FROM message_threads WHERE contract_id = ?"#,
            contract_id
        )
        .fetch_optional(&self.pool)
        .await?
        .map(|row| MessageThread {
            id: row.id.clone(),
            thread_id: hex::encode(&row.id),
            contract_id: hex::encode(&row.contract_id),
            subject: row.subject,
            created_at_ns: row.created_at_ns,
            last_message_at_ns: row.last_message_at_ns,
            status: row.status,
        });

        Ok(thread)
    }

    /// Create a message in a thread
    pub async fn create_message(
        &self,
        thread_id: &[u8],
        sender_pubkey: &str,
        sender_role: &str,
        body: &str,
    ) -> Result<Vec<u8>> {
        let message_id = uuid::Uuid::new_v4().as_bytes().to_vec();
        let created_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        let mut tx = self.pool.begin().await?;

        // Create message
        sqlx::query!(
            "INSERT INTO messages (id, thread_id, sender_pubkey, sender_role, body, created_at_ns) VALUES (?, ?, ?, ?, ?, ?)",
            message_id,
            thread_id,
            sender_pubkey,
            sender_role,
            body,
            created_at_ns
        )
        .execute(&mut *tx)
        .await?;

        // Update thread last_message_at_ns
        sqlx::query!(
            "UPDATE message_threads SET last_message_at_ns = ? WHERE id = ?",
            created_at_ns,
            thread_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(message_id)
    }

    /// Get messages for a thread with read status for a specific reader
    pub async fn get_messages_for_thread(
        &self,
        thread_id: &[u8],
        reader_pubkey: &str,
    ) -> Result<Vec<Message>> {
        let messages = sqlx::query!(
            r#"SELECT m.id, m.thread_id, m.sender_pubkey as "sender_pubkey!", m.sender_role as "sender_role!", m.body as "body!", m.created_at_ns as "created_at_ns!",
               COALESCE((SELECT 1 FROM message_read_receipts WHERE message_id = m.id AND reader_pubkey = ?), 0) as "is_read: i64"
               FROM messages m
               WHERE m.thread_id = ?
               ORDER BY m.created_at_ns ASC"#,
            reader_pubkey,
            thread_id
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| Message {
            id: row.id.clone(),
            message_id: hex::encode(&row.id),
            thread_id: hex::encode(&row.thread_id),
            sender_pubkey: row.sender_pubkey,
            sender_role: row.sender_role,
            body: row.body,
            created_at_ns: row.created_at_ns,
            is_read: row.is_read.unwrap_or(0) != 0,
        })
        .collect();

        Ok(messages)
    }

    /// Mark a message as read by a specific reader
    pub async fn mark_message_read(&self, message_id: &[u8], reader_pubkey: &str) -> Result<()> {
        let read_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        sqlx::query!(
            "INSERT INTO message_read_receipts (message_id, reader_pubkey, read_at_ns) VALUES (?, ?, ?)
             ON CONFLICT(message_id, reader_pubkey) DO NOTHING",
            message_id,
            reader_pubkey,
            read_at_ns
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get unread message count for a user
    pub async fn get_unread_count(&self, pubkey: &str) -> Result<i64> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(DISTINCT m.id) as "count!"
               FROM messages m
               INNER JOIN message_thread_participants mtp ON m.thread_id = mtp.thread_id
               LEFT JOIN message_read_receipts mrr ON m.id = mrr.message_id AND mrr.reader_pubkey = ?
               WHERE mtp.pubkey = ? AND m.sender_pubkey != ? AND mrr.message_id IS NULL"#,
            pubkey,
            pubkey,
            pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    /// Get all threads for a user
    pub async fn get_threads_for_user(&self, pubkey: &str) -> Result<Vec<MessageThread>> {
        let threads = sqlx::query!(
            r#"SELECT mt.id, mt.contract_id, mt.subject as "subject!", mt.created_at_ns as "created_at_ns!", mt.last_message_at_ns as "last_message_at_ns!", mt.status as "status!"
               FROM message_threads mt
               INNER JOIN message_thread_participants mtp ON mt.id = mtp.thread_id
               WHERE mtp.pubkey = ?
               ORDER BY mt.last_message_at_ns DESC"#,
            pubkey
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| MessageThread {
            id: row.id.clone(),
            thread_id: hex::encode(&row.id),
            contract_id: hex::encode(&row.contract_id),
            subject: row.subject,
            created_at_ns: row.created_at_ns,
            last_message_at_ns: row.last_message_at_ns,
            status: row.status,
        })
        .collect();

        Ok(threads)
    }

    /// Queue a message notification for email delivery
    pub async fn queue_message_notification(
        &self,
        message_id: &[u8],
        recipient_pubkey: &str,
    ) -> Result<Vec<u8>> {
        let notification_id = uuid::Uuid::new_v4().as_bytes().to_vec();
        let created_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        sqlx::query!(
            "INSERT INTO message_notifications (id, message_id, recipient_pubkey, status, created_at_ns) VALUES (?, ?, ?, 'pending', ?)",
            notification_id,
            message_id,
            recipient_pubkey,
            created_at_ns
        )
        .execute(&self.pool)
        .await?;

        Ok(notification_id)
    }
}

#[cfg(test)]
mod tests;
