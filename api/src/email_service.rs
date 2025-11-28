use crate::database::email::EmailQueueEntry;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

const MAILCHANNELS_API_URL: &str = "https://api.mailchannels.net/tx/v1/send";

/// Parse RFC 2822 email address format: "Name <email@example.com>" or "email@example.com"
fn parse_email_address(addr: &str) -> Result<(String, String)> {
    let trimmed = addr.trim();

    if let Some(start) = trimmed.find('<') {
        if let Some(end) = trimmed.find('>') {
            let name = trimmed[..start].trim().to_string();
            let email = trimmed[start + 1..end].trim().to_string();
            Ok((email, name))
        } else {
            bail!("Invalid email address format: missing closing '>'");
        }
    } else {
        // Just an email address without name
        Ok((trimmed.to_string(), trimmed.to_string()))
    }
}

#[derive(Debug, Clone, Serialize)]
struct EmailAddress {
    email: String,
    name: String,
}

#[derive(Debug, Clone, Serialize)]
struct EmailPersonalization {
    to: Vec<EmailAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dkim_domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dkim_selector: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dkim_private_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct EmailContent {
    #[serde(rename = "type")]
    content_type: String,
    value: String,
}

#[derive(Debug, Clone, Serialize)]
struct EmailRequest {
    personalizations: Vec<EmailPersonalization>,
    from: EmailAddress,
    subject: String,
    content: Vec<EmailContent>,
}

#[derive(Debug, Clone, Deserialize)]
struct EmailResponse {
    success: bool,
    #[serde(default)]
    errors: Vec<String>,
}

pub struct EmailService {
    api_key: String,
    client: reqwest::Client,
    dkim_domain: Option<String>,
    dkim_selector: Option<String>,
    dkim_private_key: Option<String>,
}

impl EmailService {
    pub fn new(
        api_key: String,
        dkim_domain: Option<String>,
        dkim_selector: Option<String>,
        dkim_private_key: Option<String>,
    ) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
            dkim_domain,
            dkim_selector,
            dkim_private_key,
        }
    }

    async fn send_email_api(
        &self,
        to_email: &str,
        to_name: &str,
        from_email: &str,
        from_name: &str,
        subject: &str,
        body: &str,
        is_html: bool,
    ) -> Result<()> {
        let content_type = if is_html { "text/html" } else { "text/plain" };

        let request = EmailRequest {
            personalizations: vec![EmailPersonalization {
                to: vec![EmailAddress {
                    email: to_email.to_string(),
                    name: to_name.to_string(),
                }],
                dkim_domain: self.dkim_domain.clone(),
                dkim_selector: self.dkim_selector.clone(),
                dkim_private_key: self.dkim_private_key.clone(),
            }],
            from: EmailAddress {
                email: from_email.to_string(),
                name: from_name.to_string(),
            },
            subject: subject.to_string(),
            content: vec![EmailContent {
                content_type: content_type.to_string(),
                value: body.to_string(),
            }],
        };

        let response = self
            .client
            .post(MAILCHANNELS_API_URL)
            .header("X-Api-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send email request")?;

        let status = response.status();
        let response_body: EmailResponse = response
            .json()
            .await
            .context("Failed to parse email response")?;

        if !response_body.success || !status.is_success() {
            anyhow::bail!(
                "Email send failed (status: {}): {:?}",
                status,
                response_body.errors
            );
        }

        Ok(())
    }

    pub async fn send_queued_email(&self, email: &EmailQueueEntry) -> Result<()> {
        let (to_email, to_name) =
            parse_email_address(&email.to_addr).context("Failed to parse recipient address")?;
        let (from_email, from_name) =
            parse_email_address(&email.from_addr).context("Failed to parse sender address")?;

        let is_html = email.is_html != 0;
        self.send_email_api(
            &to_email,
            &to_name,
            &from_email,
            &from_name,
            &email.subject,
            &email.body,
            is_html,
        )
        .await
    }
}

#[cfg(test)]
mod tests;
