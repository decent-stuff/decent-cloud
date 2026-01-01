//! Cloudflare DNS API client for gateway DNS management.

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Cloudflare API client for DNS management.
pub struct CloudflareClient {
    client: Client,
    api_token: String,
    zone_id: String,
}

#[derive(Debug, Deserialize)]
struct CloudflareResponse<T> {
    success: bool,
    result: Option<T>,
    errors: Vec<CloudflareError>,
}

#[derive(Debug, Deserialize)]
struct CloudflareError {
    #[allow(dead_code)]
    code: i64,
    message: String,
}

#[derive(Debug, Deserialize)]
struct DnsRecord {
    id: String,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    content: String,
}

#[derive(Debug, Serialize)]
struct CreateDnsRecord {
    #[serde(rename = "type")]
    record_type: String,
    name: String,
    content: String,
    ttl: u32,
    proxied: bool,
}

impl CloudflareClient {
    /// Create a new Cloudflare client.
    pub fn new(api_token: &str, zone_id: &str) -> Self {
        Self {
            client: Client::new(),
            api_token: api_token.to_string(),
            zone_id: zone_id.to_string(),
        }
    }

    /// Create an A record pointing to the gateway's public IP.
    pub async fn create_a_record(&self, name: &str, ip: &str) -> Result<()> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
            self.zone_id
        );

        let record = CreateDnsRecord {
            record_type: "A".to_string(),
            name: name.to_string(),
            content: ip.to_string(),
            ttl: 300,
            proxied: false,
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json")
            .json(&record)
            .send()
            .await
            .context("Failed to send Cloudflare API request")?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("Failed to read Cloudflare response")?;

        if !status.is_success() {
            bail!("Cloudflare API error ({}): {}", status, body);
        }

        let cf_response: CloudflareResponse<DnsRecord> =
            serde_json::from_str(&body).context("Failed to parse Cloudflare response")?;

        if !cf_response.success {
            let errors: Vec<String> = cf_response.errors.iter().map(|e| e.message.clone()).collect();
            bail!("Cloudflare API errors: {}", errors.join(", "));
        }

        tracing::debug!("Created DNS A record: {} -> {}", name, ip);
        Ok(())
    }

    /// Delete an A record by name.
    pub async fn delete_a_record(&self, name: &str) -> Result<()> {
        // First, find the record ID
        let record_id = self.find_record_id(name).await?;

        match record_id {
            Some(id) => {
                let url = format!(
                    "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
                    self.zone_id, id
                );

                let response = self
                    .client
                    .delete(&url)
                    .header("Authorization", format!("Bearer {}", self.api_token))
                    .send()
                    .await
                    .context("Failed to send Cloudflare delete request")?;

                let status = response.status();
                if !status.is_success() {
                    let body = response.text().await.unwrap_or_default();
                    bail!("Cloudflare delete error ({}): {}", status, body);
                }

                tracing::debug!("Deleted DNS A record: {}", name);
                Ok(())
            }
            None => {
                tracing::debug!("DNS record not found, nothing to delete: {}", name);
                Ok(())
            }
        }
    }

    /// Find a DNS record ID by name.
    async fn find_record_id(&self, name: &str) -> Result<Option<String>> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records?type=A&name={}",
            self.zone_id, name
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await
            .context("Failed to query Cloudflare DNS records")?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("Failed to read Cloudflare response")?;

        if !status.is_success() {
            bail!("Cloudflare API error ({}): {}", status, body);
        }

        let cf_response: CloudflareResponse<Vec<DnsRecord>> =
            serde_json::from_str(&body).context("Failed to parse Cloudflare response")?;

        if !cf_response.success {
            let errors: Vec<String> = cf_response.errors.iter().map(|e| e.message.clone()).collect();
            bail!("Cloudflare API errors: {}", errors.join(", "));
        }

        Ok(cf_response.result.and_then(|records| records.into_iter().next().map(|r| r.id)))
    }
}

#[cfg(test)]
mod tests {
    // Cloudflare API tests would require mocking or integration test setup
    // Unit tests verify the struct can be created
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = CloudflareClient::new("test_token", "test_zone");
        assert_eq!(client.api_token, "test_token");
        assert_eq!(client.zone_id, "test_zone");
    }
}
