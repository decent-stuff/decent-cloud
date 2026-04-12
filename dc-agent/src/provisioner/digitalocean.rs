use super::{
    HealthStatus, Instance, ProvisionRequest, Provisioner, RunningInstance, SetupVerification,
};
use crate::config::DigitalOceanConfig;
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing;

const DO_API_BASE: &str = "https://api.digitalocean.com/v2";
const DC_AGENT_TAG: &str = "dc-agent";

// ── DO API response types ───────────────────────────────────────────────────
// These are based on the DigitalOcean API v2 specification:
// https://docs.digitalocean.com/reference/api/api-reference/

#[derive(Debug, Deserialize)]
struct DropletsResponse {
    droplets: Vec<Droplet>,
    meta: Option<Meta>,
}

#[derive(Debug, Deserialize)]
struct DropletResponse {
    droplet: Droplet,
}

#[derive(Debug, Deserialize)]
struct Droplet {
    id: i64,
    name: String,
    status: String,
    memory: i64,
    vcpus: i32,
    disk: i64,
    locked: bool,
    created_at: String,
    #[serde(default)]
    networks: Networks,
    region: DoRegion,
    size_slug: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    image: Option<DoImage>,
    #[serde(default)]
    features: Vec<String>,
}

impl Droplet {
    fn public_ipv4(&self) -> Option<String> {
        self.networks
            .v4
            .iter()
            .find(|n| n.network_type == "public")
            .map(|n| n.ip_address.clone())
    }

    fn public_ipv6(&self) -> Option<String> {
        self.networks
            .v6
            .iter()
            .find(|n| n.network_type == "public")
            .map(|n| n.ip_address.clone())
    }
}

#[derive(Debug, Default, Deserialize)]
struct Networks {
    #[serde(default)]
    v4: Vec<NetworkV4>,
    #[serde(default)]
    v6: Vec<NetworkV6>,
}

#[derive(Debug, Deserialize)]
struct NetworkV4 {
    ip_address: String,
    netmask: String,
    gateway: String,
    #[serde(rename = "type")]
    network_type: String,
}

#[derive(Debug, Deserialize)]
struct NetworkV6 {
    ip_address: String,
    netmask: i32,
    gateway: String,
    #[serde(rename = "type")]
    network_type: String,
}

#[derive(Debug, Deserialize)]
struct DoRegion {
    name: String,
    slug: String,
}

#[derive(Debug, Deserialize)]
struct DoImage {
    id: i64,
    name: String,
    slug: Option<String>,
    distribution: String,
}

#[derive(Debug, Deserialize)]
struct SizesResponse {
    sizes: Vec<Size>,
}

#[derive(Debug, Deserialize)]
struct Size {
    slug: String,
    memory: i64,
    vcpus: i32,
    disk: i64,
    price_monthly: f64,
    price_hourly: f64,
    available: bool,
    #[serde(default)]
    regions: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RegionsResponse {
    regions: Vec<RegionDetail>,
}

#[derive(Debug, Deserialize)]
struct RegionDetail {
    name: String,
    slug: String,
    available: bool,
}

#[derive(Debug, Deserialize)]
struct ImagesResponse {
    images: Vec<ImageDetail>,
}

#[derive(Debug, Deserialize)]
struct ImageDetail {
    id: i64,
    name: String,
    slug: Option<String>,
    distribution: String,
    public: bool,
    available: bool,
}

#[derive(Debug, Deserialize)]
struct SshKeyResponse {
    ssh_key: SshKey,
}

#[derive(Debug, Deserialize)]
struct SshKey {
    id: i64,
    name: String,
    fingerprint: String,
}

#[derive(Debug, Deserialize)]
struct DoActionResponse {
    action: DoAction,
}

#[derive(Debug, Deserialize)]
struct DoAction {
    id: i64,
    status: String,
    #[serde(rename = "type")]
    action_type: String,
}

#[derive(Debug, Deserialize)]
struct Meta {
    total: i64,
}

#[derive(Debug, Deserialize)]
struct DoErrorResponse {
    id: String,
    message: String,
}

// ── Create droplet request ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct CreateDropletRequest {
    name: String,
    region: String,
    size: String,
    image: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    ssh_keys: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_data: Option<String>,
}

fn droplet_name(contract_id: &str) -> String {
    format!("dc-{}", contract_id)
}

fn extract_contract_id(name: &str) -> Option<String> {
    name.strip_prefix("dc-").map(String::from)
}

// ── DigitalOceanProvisioner ─────────────────────────────────────────────────

pub struct DigitalOceanProvisioner {
    config: DigitalOceanConfig,
    client: Client,
}

impl DigitalOceanProvisioner {
    pub fn new(config: DigitalOceanConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client for DigitalOcean API")?;
        Ok(Self { config, client })
    }

    fn request_builder(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", DO_API_BASE, path);
        self.client
            .request(method, &url)
            .bearer_auth(&self.config.api_token)
            .header("Content-Type", "application/json")
    }

    async fn handle_error(response: reqwest::Response) -> Result<()> {
        let status = response.status();
        if status.is_success() {
            return Ok(());
        }
        let body = response.text().await.unwrap_or_default();
        bail!(
            "DigitalOcean API error: status={}, body={}",
            status,
            body
        );
    }

    async fn get_droplet(&self, droplet_id: i64) -> Result<Option<Droplet>> {
        let resp = self
            .request_builder(reqwest::Method::GET, &format!("/v2/droplets/{}", droplet_id))
            .send()
            .await
            .with_context(|| format!("Failed to GET droplet {}", droplet_id))?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Failed to get droplet {}: status={}, body={}", droplet_id, status, body);
        }

        let droplet_resp: DropletResponse = resp
            .json()
            .await
            .context("Failed to parse droplet response")?;
        Ok(Some(droplet_resp.droplet))
    }

    async fn wait_for_droplet_active(&self, droplet_id: i64, max_retries: u32) -> Result<Droplet> {
        for attempt in 0..max_retries {
            let droplet = self
                .get_droplet(droplet_id)
                .await?
                .context("Droplet disappeared while waiting for active state")?;

            if droplet.status == "active" {
                tracing::info!(droplet_id, "Droplet is active");
                return Ok(droplet);
            }

            tracing::debug!(
                droplet_id,
                status = %droplet.status,
                attempt,
                "Waiting for droplet to become active"
            );
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
        bail!(
            "Droplet {} did not reach 'active' state within {} retries",
            droplet_id,
            max_retries
        );
    }

    async fn create_ssh_key(&self, name: &str, public_key: &str) -> Result<i64> {
        #[derive(Serialize)]
        struct CreateSshKeyRequest {
            name: String,
            public_key: String,
        }

        let resp = self
            .request_builder(reqwest::Method::POST, "/v2/account/keys")
            .json(&CreateSshKeyRequest {
                name: name.to_string(),
                public_key: public_key.to_string(),
            })
            .send()
            .await
            .context("Failed to create SSH key on DigitalOcean")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Failed to create SSH key: status={}, body={}", status, body);
        }

        let key_resp: SshKeyResponse = resp
            .json()
            .await
            .context("Failed to parse SSH key response")?;
        Ok(key_resp.ssh_key.id)
    }

    async fn delete_ssh_key(&self, key_id: i64) -> Result<()> {
        let resp = self
            .request_builder(reqwest::Method::DELETE, &format!("/v2/account/keys/{}", key_id))
            .send()
            .await
            .with_context(|| format!("Failed to delete SSH key {}", key_id))?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            tracing::warn!(key_id, "SSH key not found, assuming already deleted");
            return Ok(());
        }
        Self::handle_error(resp).await
    }

    fn resolve_size(&self, request: &ProvisionRequest) -> String {
        request
            .instance_config
            .as_ref()
            .and_then(|c| c.get("size"))
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| self.config.default_size.clone())
    }

    fn resolve_region(&self, request: &ProvisionRequest) -> String {
        request
            .instance_config
            .as_ref()
            .and_then(|c| c.get("region"))
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| self.config.default_region.clone())
    }

    fn resolve_image(&self, request: &ProvisionRequest) -> String {
        request
            .instance_config
            .as_ref()
            .and_then(|c| c.get("image"))
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| self.config.default_image.clone())
    }

    fn droplet_to_instance(&self, droplet: &Droplet) -> Instance {
        Instance {
            external_id: droplet.id.to_string(),
            ip_address: droplet.public_ipv4(),
            ipv6_address: droplet.public_ipv6(),
            public_ip: droplet.public_ipv4(),
            ssh_port: 22,
            root_password: None,
            additional_details: Some(serde_json::json!({
                "name": droplet.name,
                "size_slug": droplet.size_slug,
                "region": droplet.region.slug,
                "status": droplet.status,
                "vcpus": droplet.vcpus,
                "memory": droplet.memory,
                "disk": droplet.disk,
            })),
            gateway_slug: None,
            gateway_subdomain: None,
            gateway_ssh_port: None,
            gateway_port_range_start: None,
            gateway_port_range_end: None,
        }
    }
}

#[async_trait]
impl Provisioner for DigitalOceanProvisioner {
    async fn provision(&self, request: &ProvisionRequest) -> Result<Instance> {
        let name = droplet_name(&request.contract_id);
        let size = self.resolve_size(request);
        let region = self.resolve_region(request);
        let image = self.resolve_image(request);

        tracing::info!(
            contract_id = %request.contract_id,
            size = %size,
            region = %region,
            image = %image,
            "Provisioning DigitalOcean droplet"
        );

        let mut ssh_key_ids: Vec<i64> = Vec::new();
        let mut created_ssh_key_id: Option<i64> = None;

        if let Some(pubkey) = &request.requester_ssh_pubkey {
            match self
                .create_ssh_key(&format!("dc-{}", request.contract_id), pubkey)
                .await
            {
                Ok(key_id) => {
                    tracing::info!(key_id, "Created SSH key on DigitalOcean");
                    ssh_key_ids.push(key_id);
                    created_ssh_key_id = Some(key_id);
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to create SSH key, provisioning without SSH key");
                }
            }
        }

        let create_req = CreateDropletRequest {
            name: name.clone(),
            region: region.clone(),
            size: size.clone(),
            image: image.clone(),
            ssh_keys: if ssh_key_ids.is_empty() {
                None
            } else {
                Some(ssh_key_ids)
            },
            tags: Some(vec![DC_AGENT_TAG.to_string(), format!("dc-contract-{}", request.contract_id)]),
            user_data: None,
        };

        let resp = self
            .request_builder(reqwest::Method::POST, "/v2/droplets")
            .json(&create_req)
            .send()
            .await
            .context("Failed to create droplet")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if let Some(key_id) = created_ssh_key_id {
                let _ = self.delete_ssh_key(key_id).await;
            }
            bail!(
                "Failed to create droplet: status={}, body={}",
                status,
                body
            );
        }

        let droplet_resp: DropletResponse = resp
            .json()
            .await
            .context("Failed to parse create droplet response")?;

        let droplet_id = droplet_resp.droplet.id;
        tracing::info!(droplet_id, "Droplet created, waiting for active state");

        match self.wait_for_droplet_active(droplet_id, 60).await {
            Ok(droplet) => {
                let instance = self.droplet_to_instance(&droplet);
                tracing::info!(
                    droplet_id,
                    ip = ?instance.ip_address,
                    "Droplet provisioned successfully"
                );
                Ok(instance)
            }
            Err(e) => {
                tracing::error!(droplet_id, error = %e, "Droplet failed to become active, cleaning up");
                let _ = self
                    .request_builder(
                        reqwest::Method::DELETE,
                        &format!("/v2/droplets/{}", droplet_id),
                    )
                    .send()
                    .await;
                if let Some(key_id) = created_ssh_key_id {
                    let _ = self.delete_ssh_key(key_id).await;
                }
                Err(e)
            }
        }
    }

    async fn terminate(&self, external_id: &str) -> Result<()> {
        tracing::info!(external_id, "Terminating DigitalOcean droplet");

        let resp = self
            .request_builder(
                reqwest::Method::DELETE,
                &format!("/v2/droplets/{}", external_id),
            )
            .send()
            .await
            .with_context(|| format!("Failed to delete droplet {}", external_id))?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            tracing::warn!(external_id, "Droplet not found, assuming already deleted");
            return Ok(());
        }
        Self::handle_error(resp).await
    }

    async fn health_check(&self, external_id: &str) -> Result<HealthStatus> {
        let droplet = match self.get_droplet(external_id.parse().context("Invalid droplet ID")?).await {
            Ok(Some(d)) => d,
            Ok(None) => {
                return Ok(HealthStatus::Unhealthy {
                    reason: "Droplet not found".to_string(),
                });
            }
            Err(e) => {
                return Ok(HealthStatus::Unhealthy {
                    reason: format!("Failed to check droplet: {:#}", e),
                });
            }
        };

        match droplet.status.as_str() {
            "active" => {
                let uptime_seconds = chrono::DateTime::parse_from_rfc3339(&droplet.created_at)
                    .map(|dt| {
                        dt.signed_duration_since(chrono::Utc::now())
                            .num_seconds()
                            .unsigned_abs()
                    })
                    .unwrap_or(0);
                Ok(HealthStatus::Healthy { uptime_seconds })
            }
            "new" => Ok(HealthStatus::Unhealthy {
                reason: "Droplet is still being created".to_string(),
            }),
            "off" => Ok(HealthStatus::Unhealthy {
                reason: "Droplet is powered off".to_string(),
            }),
            "archive" => Ok(HealthStatus::Unhealthy {
                reason: "Droplet is archived".to_string(),
            }),
            other => Ok(HealthStatus::Unhealthy {
                reason: format!("Unknown droplet status: {}", other),
            }),
        }
    }

    async fn get_instance(&self, external_id: &str) -> Result<Option<Instance>> {
        let droplet_id: i64 = external_id.parse().context("Invalid droplet ID")?;
        match self.get_droplet(droplet_id).await? {
            Some(droplet) => Ok(Some(self.droplet_to_instance(&droplet))),
            None => Ok(None),
        }
    }

    async fn list_running_instances(&self) -> Result<Vec<RunningInstance>> {
        let mut all_instances = Vec::new();
        let mut page = 1u32;

        loop {
            let resp = self
                .request_builder(
                    reqwest::Method::GET,
                    &format!("/v2/droplets?tag_name={}&page={}&per_page=200", DC_AGENT_TAG, page),
                )
                .send()
                .await
                .context("Failed to list droplets")?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                bail!("Failed to list droplets: status={}, body={}", status, body);
            }

            let droplets_resp: DropletsResponse = resp
                .json()
                .await
                .context("Failed to parse droplets list response")?;

            for droplet in &droplets_resp.droplets {
                let contract_id = extract_contract_id(&droplet.name);
                all_instances.push(RunningInstance {
                    external_id: droplet.id.to_string(),
                    contract_id,
                });
            }

            let total = droplets_resp.meta.as_ref().map(|m| m.total).unwrap_or(0);
            if (page * 200) as i64 >= total {
                break;
            }
            page += 1;
        }

        Ok(all_instances)
    }

    async fn verify_setup(&self) -> SetupVerification {
        let mut result = SetupVerification::default();

        match self
            .request_builder(reqwest::Method::GET, "/v2/droplets?per_page=1")
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                result.api_reachable = Some(true);
            }
            Ok(resp) => {
                result.api_reachable = Some(false);
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                result.errors.push(format!(
                    "DigitalOcean API returned error: status={}, body={}",
                    status, body
                ));
                return result;
            }
            Err(e) => {
                result.api_reachable = Some(false);
                result
                    .errors
                    .push(format!("Cannot reach DigitalOcean API: {:#}", e));
                return result;
            }
        }

        result
    }
}

#[cfg(test)]
#[path = "digitalocean_tests.rs"]
mod tests;
