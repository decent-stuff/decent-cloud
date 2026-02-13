//! Cloudflare DNS API client for gateway DNS management.
//!
//! Centralizes DNS management so agent hosts don't need Cloudflare credentials.

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Cloudflare DNS client for DNS record management.
pub struct CloudflareDns {
    client: Client,
    api_token: String,
    zone_id: String,
    domain: String,
    /// Gateway DNS prefix: "gw" for prod, "dev-gw" for dev (from CF_GW_PREFIX env var)
    gw_prefix: String,
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
    /// Returns None if required environment variables are not set or empty.
    pub fn from_env() -> Option<Arc<Self>> {
        let api_token = std::env::var("CF_API_TOKEN").ok().filter(|s| !s.is_empty())?;
        let zone_id = std::env::var("CF_ZONE_ID").ok().filter(|s| !s.is_empty())?;
        let domain = std::env::var("CF_DOMAIN").unwrap_or_else(|_| "decent-cloud.org".to_string());
        let gw_prefix =
            std::env::var("CF_GW_PREFIX").unwrap_or_else(|_| "gw".to_string());

        Some(Arc::new(Self {
            client: Client::new(),
            api_token,
            zone_id,
            domain,
            gw_prefix,
        }))
    }

    /// Get the base domain (e.g., "decent-cloud.org")
    pub fn domain(&self) -> &str {
        &self.domain
    }

    /// Get the gateway prefix (e.g., "gw" or "dev-gw")
    pub fn gw_prefix(&self) -> &str {
        &self.gw_prefix
    }

    /// Build the full gateway FQDN: {slug}.{dc_id}.{gw_prefix}.{domain}
    pub fn gateway_fqdn(&self, slug: &str, dc_id: &str) -> String {
        format!("{}.{}.{}.{}", slug, dc_id, self.gw_prefix, self.domain)
    }

    /// Validate dc_id: 2-20 chars, [a-z0-9-], no leading/trailing hyphen.
    pub fn validate_dc_id(dc_id: &str) -> Result<()> {
        if dc_id.len() < 2 || dc_id.len() > 20 {
            bail!("Invalid dc_id: must be 2-20 characters, got {}", dc_id.len());
        }
        if !dc_id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            bail!("Invalid dc_id: must contain only [a-z0-9-]");
        }
        if dc_id.starts_with('-') || dc_id.ends_with('-') {
            bail!("Invalid dc_id: must not start or end with a hyphen");
        }
        Ok(())
    }

    /// Create an A record for a gateway slug.
    /// Record name format: {slug}.{dc_id}.{gw_prefix} (Cloudflare appends zone domain)
    pub async fn create_gateway_record(
        &self,
        slug: &str,
        dc_id: &str,
        public_ip: &str,
    ) -> Result<()> {
        // Validate inputs
        if slug.len() != 6
            || !slug
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        {
            bail!("Invalid slug format: must be 6 lowercase alphanumeric characters");
        }

        Self::validate_dc_id(dc_id)?;

        // Build the subdomain record name (without domain suffix)
        // e.g., "k7m2p4.a3x9f2b1.dev-gw" (Cloudflare appends the zone domain)
        let record_name = format!("{}.{}.{}", slug, dc_id, self.gw_prefix);

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
    pub async fn delete_gateway_record(&self, slug: &str, dc_id: &str) -> Result<()> {
        let full_name = self.gateway_fqdn(slug, dc_id);

        // First, find the record ID
        let record_id = self.find_record_id(&full_name, "A").await?;

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
    async fn find_record_id(&self, name: &str, record_type: &str) -> Result<Option<String>> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records?type={}&name={}",
            self.zone_id, record_type, name
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

    /// Create or update a TXT record.
    /// Record name should be the full subdomain (e.g., "selector._domainkey" for DKIM).
    pub async fn create_txt_record(&self, name: &str, content: &str) -> Result<()> {
        let full_name = format!("{}.{}", name, self.domain);

        // Check if record already exists
        if let Some(record_id) = self.find_record_id(&full_name, "TXT").await? {
            // Update existing record
            let url = format!(
                "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
                self.zone_id, record_id
            );

            let record = CreateDnsRecord {
                record_type: "TXT".to_string(),
                name: name.to_string(),
                content: content.to_string(),
                ttl: 3600,
                proxied: false,
            };

            let response = self
                .client
                .put(&url)
                .header("Authorization", format!("Bearer {}", self.api_token))
                .header("Content-Type", "application/json")
                .json(&record)
                .send()
                .await
                .context("Failed to send Cloudflare update request")?;

            let status = response.status();
            let body = response
                .text()
                .await
                .context("Failed to read Cloudflare response")?;

            if !status.is_success() {
                bail!("Cloudflare API error ({}): {}", status, body);
            }

            tracing::info!("Updated DNS TXT record: {}", full_name);
            return Ok(());
        }

        // Create new record
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
            self.zone_id
        );

        let record = CreateDnsRecord {
            record_type: "TXT".to_string(),
            name: name.to_string(),
            content: content.to_string(),
            ttl: 3600,
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

        tracing::info!("Created DNS TXT record: {}", full_name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_dns_record_serialization() {
        let record = CreateDnsRecord {
            record_type: "TXT".to_string(),
            name: "selector._domainkey".to_string(),
            content: "v=DKIM1; k=ed25519; p=ABC123".to_string(),
            ttl: 3600,
            proxied: false,
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("\"type\":\"TXT\""));
        assert!(json.contains("\"name\":\"selector._domainkey\""));
        assert!(json.contains("\"content\":\"v=DKIM1; k=ed25519; p=ABC123\""));
        assert!(json.contains("\"ttl\":3600"));
        assert!(json.contains("\"proxied\":false"));
    }

    #[test]
    fn test_gateway_fqdn() {
        let dns = CloudflareDns {
            client: Client::new(),
            api_token: String::new(),
            zone_id: String::new(),
            domain: "decent-cloud.org".to_string(),
            gw_prefix: "dev-gw".to_string(),
        };
        assert_eq!(
            dns.gateway_fqdn("k7m2p4", "a3x9f2b1"),
            "k7m2p4.a3x9f2b1.dev-gw.decent-cloud.org"
        );

        let dns_prod = CloudflareDns {
            client: Client::new(),
            api_token: String::new(),
            zone_id: String::new(),
            domain: "decent-cloud.org".to_string(),
            gw_prefix: "gw".to_string(),
        };
        assert_eq!(
            dns_prod.gateway_fqdn("k7m2p4", "a3x9f2b1"),
            "k7m2p4.a3x9f2b1.gw.decent-cloud.org"
        );
    }

    #[test]
    fn test_validate_dc_id_valid() {
        assert!(CloudflareDns::validate_dc_id("ab").is_ok());
        assert!(CloudflareDns::validate_dc_id("a3x9f2b1").is_ok());
        assert!(CloudflareDns::validate_dc_id("dc-lk").is_ok());
        assert!(CloudflareDns::validate_dc_id("us-east-1").is_ok());
        assert!(CloudflareDns::validate_dc_id("a1234567890123456789").is_ok()); // 20 chars
    }

    #[test]
    fn test_validate_dc_id_too_short() {
        let err = CloudflareDns::validate_dc_id("a").unwrap_err();
        assert!(err.to_string().contains("2-20 characters"));
    }

    #[test]
    fn test_validate_dc_id_too_long() {
        let err = CloudflareDns::validate_dc_id("a12345678901234567890").unwrap_err(); // 21 chars
        assert!(err.to_string().contains("2-20 characters"));
    }

    #[test]
    fn test_validate_dc_id_leading_hyphen() {
        let err = CloudflareDns::validate_dc_id("-abc").unwrap_err();
        assert!(err.to_string().contains("hyphen"));
    }

    #[test]
    fn test_validate_dc_id_trailing_hyphen() {
        let err = CloudflareDns::validate_dc_id("abc-").unwrap_err();
        assert!(err.to_string().contains("hyphen"));
    }

    #[test]
    fn test_validate_dc_id_uppercase_rejected() {
        let err = CloudflareDns::validate_dc_id("DC-LK").unwrap_err();
        assert!(err.to_string().contains("[a-z0-9-]"));
    }

    #[test]
    fn test_validate_dc_id_underscore_rejected() {
        let err = CloudflareDns::validate_dc_id("dc_lk").unwrap_err();
        assert!(err.to_string().contains("[a-z0-9-]"));
    }
}
