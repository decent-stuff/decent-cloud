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
struct EmailErrorResponse {
    #[serde(default)]
    errors: Vec<String>,
}

struct EmailParams<'a> {
    to_email: &'a str,
    to_name: &'a str,
    from_email: &'a str,
    from_name: &'a str,
    subject: &'a str,
    body: &'a str,
    is_html: bool,
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

    async fn send_email_api(&self, params: EmailParams<'_>) -> Result<()> {
        let content_type = if params.is_html { "text/html" } else { "text/plain" };

        let request = EmailRequest {
            personalizations: vec![EmailPersonalization {
                to: vec![EmailAddress {
                    email: params.to_email.to_string(),
                    name: params.to_name.to_string(),
                }],
                dkim_domain: self.dkim_domain.clone(),
                dkim_selector: self.dkim_selector.clone(),
                dkim_private_key: self.dkim_private_key.clone(),
            }],
            from: EmailAddress {
                email: params.from_email.to_string(),
                name: params.from_name.to_string(),
            },
            subject: params.subject.to_string(),
            content: vec![EmailContent {
                content_type: content_type.to_string(),
                value: params.body.to_string(),
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

        // MailChannels returns 202 Accepted with empty/minimal body on success
        if status.is_success() {
            return Ok(());
        }

        // On error, try to parse response body for error details
        let response_text = response
            .text()
            .await
            .context("Failed to read error response body")?;

        // Try to parse as JSON error response
        let error_msg =
            if let Ok(err_response) = serde_json::from_str::<EmailErrorResponse>(&response_text) {
                format!("{:?}", err_response.errors)
            } else {
                response_text
            };

        anyhow::bail!("Email send failed (status: {}): {}", status, error_msg)
    }

    /// Send email directly without database queue
    pub async fn send_email(
        &self,
        from_addr: &str,
        to_addr: &str,
        subject: impl Into<String>,
        body: &str,
        is_html: bool,
    ) -> Result<()> {
        let (to_email, to_name) =
            parse_email_address(to_addr).context("Failed to parse recipient address")?;
        let (from_email, from_name) =
            parse_email_address(from_addr).context("Failed to parse sender address")?;
        let subject = subject.into();

        self.send_email_api(EmailParams {
            to_email: &to_email,
            to_name: &to_name,
            from_email: &from_email,
            from_name: &from_name,
            subject: &subject,
            body,
            is_html,
        })
        .await
    }
}
