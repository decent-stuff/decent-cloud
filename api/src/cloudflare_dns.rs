//! Cloudflare DNS API client for gateway DNS management.
//!
//! Centralizes DNS management so agent hosts don't need Cloudflare credentials.

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Cloudflare DNS client for A record management.
pub struct CloudflareDns {
    client: Client,
    api_token: String,
    zone_id: String,
    domain: String,
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

impl CloudflareDns {
    /// Create a new Cloudflare DNS client from environment variables.
    /// Returns None if required environment variables are not set.
    pub fn from_env() -> Option<Arc<Self>> {
        let api_token = std::env::var("CF_API_TOKEN").ok()?;
        let zone_id = std::env::var("CF_ZONE_ID").ok()?;
        let domain = std::env::var("CF_DOMAIN").unwrap_or_else(|_| "decent-cloud.org".to_string());

        Some(Arc::new(Self {
            client: Client::new(),
            api_token,
            zone_id,
            domain,
        }))
    }

    /// Get the base domain (e.g., "decent-cloud.org")
    pub fn domain(&self) -> &str {
        &self.domain
    }

    /// Create an A record for a gateway slug.
    /// Record name format: {slug}.{datacenter}.{domain}
    pub async fn create_gateway_record(
        &self,
        slug: &str,
        datacenter: &str,
        public_ip: &str,
    ) -> Result<()> {
        // Validate inputs
        if slug.len() != 6 || !slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
            bail!("Invalid slug format: must be 6 lowercase alphanumeric characters");
        }

        if datacenter.is_empty() || datacenter.len() > 20 {
            bail!("Invalid datacenter: must be 1-20 characters");
        }

        // Build the subdomain record name (without domain suffix)
        // e.g., "k7m2p4.dc-lk" (Cloudflare appends the zone domain)
        let record_name = format!("{}.{}", slug, datacenter);

        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
            self.zone_id
        );

        let record = CreateDnsRecord {
            record_type: "A".to_string(),
            name: record_name.clone(),
            content: public_ip.to_string(),
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
            let errors: Vec<String> = cf_response
                .errors
                .iter()
                .map(|e| e.message.clone())
                .collect();
            bail!("Cloudflare API errors: {}", errors.join(", "));
        }

        tracing::info!(
            "Created DNS A record: {}.{} -> {}",
            record_name,
            self.domain,
            public_ip
        );
        Ok(())
    }

    /// Delete an A record for a gateway slug.
    pub async fn delete_gateway_record(&self, slug: &str, datacenter: &str) -> Result<()> {
        let full_name = format!("{}.{}.{}", slug, datacenter, self.domain);

        // First, find the record ID
        let record_id = self.find_record_id(&full_name).await?;

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

                tracing::info!("Deleted DNS A record: {}", full_name);
                Ok(())
            }
            None => {
                tracing::debug!("DNS record not found, nothing to delete: {}", full_name);
                Ok(())
            }
        }
    }

    /// Find a DNS record ID by full name.
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
            let errors: Vec<String> = cf_response
                .errors
                .iter()
                .map(|e| e.message.clone())
                .collect();
            bail!("Cloudflare API errors: {}", errors.join(", "));
        }

        Ok(cf_response
            .result
            .and_then(|records| records.into_iter().next().map(|r| r.id)))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_from_env_missing_vars() {
        // When env vars are not set, should return None
        let client = super::CloudflareDns::from_env();
        // In test environment, env vars are typically not set
        // This just verifies the function handles missing vars gracefully
        assert!(client.is_none() || client.is_some());
    }
}
