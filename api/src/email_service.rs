use crate::database::email::EmailQueueEntry;
use anyhow::Result;
pub use email_utils::EmailService;

/// Extension trait for EmailService to work with database queue entries
pub trait EmailServiceExt {
    async fn send_queued_email(&self, email: &EmailQueueEntry) -> Result<()>;
}

impl EmailServiceExt for EmailService {
    async fn send_queued_email(&self, email: &EmailQueueEntry) -> Result<()> {
        let is_html = email.is_html;
        self.send_email(
            &email.from_addr,
            &email.to_addr,
            &email.subject,
            &email.body,
            is_html,
        )
        .await
    }
}
