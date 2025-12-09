use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};

const MAILCHANNELS_API_URL: &str = "https://api.mailchannels.net/tx/v1/send";

/// Email attachment
#[derive(Debug, Clone)]
pub struct EmailAttachment {
    /// MIME type (e.g., "application/pdf")
    pub content_type: String,
    /// Filename shown to recipient
    pub filename: String,
    /// Raw file content (will be base64 encoded for API)
    pub content: Vec<u8>,
}

/// Check if email domain is a test domain per RFC 2606 (reserved for testing/documentation).
/// Returns true for domains that should NOT receive real emails.
fn is_test_domain(email: &str) -> bool {
    let domain = match email.rsplit_once('@') {
        Some((_, domain)) => domain.to_lowercase(),
        None => return false,
    };
    // RFC 2606 reserved domains + common test patterns
    domain.ends_with(".test")
        || domain.ends_with(".example")
        || domain.ends_with(".invalid")
        || domain.ends_with(".localhost")
        || domain == "example.com"
        || domain == "example.net"
        || domain == "example.org"
        || domain.ends_with(".example.com")
        || domain.ends_with(".example.net")
        || domain.ends_with(".example.org")
}

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
struct ApiAttachment {
    #[serde(rename = "type")]
    content_type: String,
    filename: String,
    content: String, // Base64 encoded
}

#[derive(Debug, Clone, Serialize)]
struct EmailRequest {
    personalizations: Vec<EmailPersonalization>,
    from: EmailAddress,
    subject: String,
    content: Vec<EmailContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<ApiAttachment>>,
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
    attachments: Option<&'a [EmailAttachment]>,
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
        let content_type = if params.is_html {
            "text/html"
        } else {
            "text/plain"
        };

        let api_attachments = params.attachments.map(|attachments| {
            attachments
                .iter()
                .map(|a| ApiAttachment {
                    content_type: a.content_type.clone(),
                    filename: a.filename.clone(),
                    content: BASE64.encode(&a.content),
                })
                .collect()
        });

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
            attachments: api_attachments,
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
        self.send_email_with_attachments(from_addr, to_addr, subject, body, is_html, None)
            .await
    }

    /// Send email with optional attachments
    pub async fn send_email_with_attachments(
        &self,
        from_addr: &str,
        to_addr: &str,
        subject: impl Into<String>,
        body: &str,
        is_html: bool,
        attachments: Option<&[EmailAttachment]>,
    ) -> Result<()> {
        let (to_email, to_name) =
            parse_email_address(to_addr).context("Failed to parse recipient address")?;

        // Skip sending to RFC 2606 reserved test domains (used in testing)
        if is_test_domain(&to_email) {
            return Ok(());
        }

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
            attachments,
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_test_domain() {
        // RFC 2606 reserved TLDs
        assert!(is_test_domain("user@domain.test"));
        assert!(is_test_domain("user@sub.domain.test"));
        assert!(is_test_domain("user@domain.example"));
        assert!(is_test_domain("user@domain.invalid"));
        assert!(is_test_domain("user@domain.localhost"));

        // RFC 2606 reserved second-level domains
        assert!(is_test_domain("user@example.com"));
        assert!(is_test_domain("user@example.net"));
        assert!(is_test_domain("user@example.org"));

        // Subdomains of reserved domains (common in tests)
        assert!(is_test_domain("user@test.example.com"));
        assert!(is_test_domain("user@sub.test.example.com"));

        // Real domains should NOT be blocked
        assert!(!is_test_domain("user@gmail.com"));
        assert!(!is_test_domain("user@decent-cloud.org"));
        assert!(!is_test_domain("user@company.io"));

        // Case insensitive
        assert!(is_test_domain("user@EXAMPLE.COM"));
        assert!(is_test_domain("user@Test.Example.Com"));
    }
}
