//! Vultr Cloud API v2 client
//!
//! Implements the CloudBackend trait for Vultr Cloud.
//!
//! API docs: https://www.vultr.com/api/

use anyhow::Context;
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

use super::{CloudBackend, ProvisionResult};
use crate::cloud::types::{
    BackendCatalog, CreateServerRequest, Image, Location, Server, ServerMetrics, ServerStatus,
    ServerType,
};

const VULTR_API_BASE: &str = "https://api.vultr.com/v2";
const REQUEST_TIMEOUT_SECS: u64 = 30;
const IP_ASSIGNMENT_TIMEOUT_SECS: u64 = 120;

pub struct VultrBackend {
    client: Client,
    api_key: String,
    base_url: String,
    poll_interval: std::time::Duration,
    ip_wait_timeout_secs: u64,
    ssh_wait_timeout_secs: u64,
}

impl VultrBackend {
    pub fn new(api_key: String) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            api_key,
            base_url: VULTR_API_BASE.to_string(),
            poll_interval: std::time::Duration::from_secs(5),
            ip_wait_timeout_secs: IP_ASSIGNMENT_TIMEOUT_SECS,
            ssh_wait_timeout_secs: 120,
        })
    }

    #[cfg(test)]
    fn new_for_mockito(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("Failed to build test HTTP client");
        Self {
            client,
            api_key: "test-key".to_string(),
            base_url,
            poll_interval: std::time::Duration::from_millis(10),
            ip_wait_timeout_secs: 1,
            ssh_wait_timeout_secs: 0,
        }
    }

    fn request_builder(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        self.client
            .request(method, &url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
    }

    async fn handle_error(&self, response: reqwest::Response) -> anyhow::Error {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        match status {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                anyhow::anyhow!("Vultr API authentication failed ({}): {}", status, body)
            }
            StatusCode::UNPROCESSABLE_ENTITY => {
                anyhow::anyhow!("Invalid request parameters: {}", body)
            }
            StatusCode::TOO_MANY_REQUESTS => {
                anyhow::anyhow!("Rate limited by Vultr API: {}", body)
            }
            _ => anyhow::anyhow!("Vultr API error ({}): {}", status, body),
        }
    }
}

#[derive(Debug, Serialize)]
struct CreateSshKeyRequest {
    name: String,
    ssh_key: String,
}

#[derive(Debug, Deserialize)]
struct SshKeyResponse {
    ssh_key: VultrSshKey,
}

#[derive(Debug, Deserialize)]
struct VultrSshKey {
    id: String,
    #[allow(dead_code)]
    name: String,
}

#[derive(Debug, Serialize)]
struct CreateInstanceRequest {
    label: String,
    plan: String,
    region: String,
    os_id: i64,
    sshkey_id: Vec<String>,
    enable_ipv6: bool,
}

#[derive(Debug, Deserialize)]
struct InstanceResponse {
    instance: VultrInstance,
}

#[derive(Debug, Deserialize)]
struct PlansResponse {
    plans: Vec<VultrPlan>,
}

#[derive(Debug, Deserialize)]
struct RegionsResponse {
    regions: Vec<VultrRegion>,
}

#[derive(Debug, Deserialize)]
struct OsResponse {
    os: Vec<VultrOs>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AccountResponse {
    account: VultrAccount,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct VultrAccount {
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct VultrInstance {
    id: String,
    label: String,
    status: String,
    main_ip: String,
    plan: String,
    region: String,
    os_id: i64,
    date_created: String,
}

#[derive(Debug, Deserialize)]
struct VultrPlan {
    id: String,
    vcpu_count: i32,
    ram: i64,
    disk: i32,
    monthly_cost: f64,
    hourly_cost: f64,
    #[serde(rename = "type")]
    type_: String,
    locations: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct VultrRegion {
    id: String,
    city: String,
    country: String,
}

#[derive(Debug, Deserialize)]
struct VultrOs {
    id: i64,
    name: String,
    #[allow(dead_code)]
    arch: String,
    family: String,
}

#[derive(Debug)]
pub struct VultrProvisionerConfig {
    pub plan: String,
    pub region: String,
    pub os_id: i64,
}

pub fn resolve_provisioner_config(
    provisioner_config: Option<&str>,
    datacenter_city: &str,
    template_name: Option<&str>,
) -> anyhow::Result<VultrProvisionerConfig> {
    let config: serde_json::Value = provisioner_config
        .map(serde_json::from_str)
        .transpose()
        .context("Invalid provisioner_config JSON")?
        .unwrap_or(serde_json::json!({}));

    let plan = config["plan"].as_str().unwrap_or("vc2-1c-1gb").to_string();
    let region = config["region"]
        .as_str()
        .unwrap_or(&datacenter_city.to_lowercase())
        .to_string();
    let os_id = config["os_id"]
        .as_i64()
        .or_else(|| {
            let name = config["os_name"]
                .as_str()
                .or(template_name)
                .unwrap_or("ubuntu-24.04");
            default_os_id_for_name(name)
        })
        .unwrap_or(2284);

    Ok(VultrProvisionerConfig {
        plan,
        region,
        os_id,
    })
}

fn default_os_id_for_name(name: &str) -> Option<i64> {
    match name {
        "ubuntu-24.04" | "ubuntu-24.04-lts" => Some(2284),
        "ubuntu-22.04" | "ubuntu-22.04-lts" => Some(1743),
        "debian-12" => Some(2136),
        "debian-11" => Some(477),
        _ => None,
    }
}

fn check_plan_region(plans: &[VultrPlan], plan: &str, region: &str) -> anyhow::Result<()> {
    let p = plans.iter().find(|p| p.id == plan).ok_or_else(|| {
        let known: Vec<&str> = plans.iter().map(|p| p.id.as_str()).collect();
        anyhow::anyhow!(
            "Unknown Vultr plan '{}'. Available plans: {}",
            plan,
            if known.is_empty() {
                "(none)".to_string()
            } else {
                known.join(", ")
            }
        )
    })?;

    if !p.locations.contains(&region.to_string()) {
        anyhow::bail!(
            "Plan '{}' not available in region '{}'. Available regions: {}",
            plan,
            region,
            if p.locations.is_empty() {
                "(none)".to_string()
            } else {
                p.locations.join(", ")
            }
        );
    }

    Ok(())
}

fn check_os_exists(os_list: &[VultrOs], os_id: i64) -> anyhow::Result<()> {
    if os_list.iter().any(|o| o.id == os_id) {
        return Ok(());
    }
    let known: Vec<String> = os_list
        .iter()
        .map(|o| format!("{} (id={})", o.name, o.id))
        .collect();
    anyhow::bail!(
        "Unknown Vultr OS id '{}'. Available OS: {}",
        os_id,
        if known.is_empty() {
            "(none)".to_string()
        } else {
            known.join(", ")
        }
    )
}

impl VultrBackend {
    pub async fn validate_offering_config(
        &self,
        config: &VultrProvisionerConfig,
    ) -> anyhow::Result<()> {
        let plans_response = self
            .request_builder(reqwest::Method::GET, "/plans")
            .send()
            .await
            .context("Failed to query Vultr plans")?;

        if !plans_response.status().is_success() {
            return Err(self.handle_error(plans_response).await);
        }

        let plans_data: PlansResponse = plans_response.json().await?;
        check_plan_region(&plans_data.plans, &config.plan, &config.region)?;

        let os_response = self
            .request_builder(reqwest::Method::GET, "/os")
            .send()
            .await
            .context("Failed to query Vultr OS catalog")?;

        if !os_response.status().is_success() {
            return Err(self.handle_error(os_response).await);
        }

        let os_data: OsResponse = os_response.json().await?;
        check_os_exists(&os_data.os, config.os_id)?;

        Ok(())
    }

    fn convert_instance(&self, i: VultrInstance) -> Server {
        let status = match i.status.as_str() {
            "pending" | "installing" | "resizing" => ServerStatus::Provisioning,
            "active" => ServerStatus::Running,
            "halted" | "paused" => ServerStatus::Stopped,
            "suspending" | "suspended" => ServerStatus::Stopped,
            "destroying" => ServerStatus::Deleting,
            other => {
                tracing::warn!(
                    "Unknown Vultr instance status '{}', treating as failed",
                    other
                );
                ServerStatus::Failed
            }
        };

        let created_at =
            chrono::DateTime::parse_from_rfc3339(&format!("{}T00:00:00Z", i.date_created))
                .map(|dt| dt.with_timezone(&chrono::Utc).to_rfc3339())
                .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339());

        let public_ip = if i.main_ip == "0.0.0.0" || i.main_ip.is_empty() {
            None
        } else {
            Some(i.main_ip)
        };

        Server {
            id: i.id,
            name: i.label,
            status,
            public_ip,
            server_type: i.plan,
            location: i.region,
            image: i.os_id.to_string(),
            created_at,
        }
    }

    fn convert_plan(&self, p: VultrPlan) -> ServerType {
        ServerType {
            id: p.id,
            name: format!("{} ({} vCPU, {} MB)", p.type_, p.vcpu_count, p.ram),
            cores: p.vcpu_count as u32,
            memory_gb: p.ram as f64 / 1024.0,
            disk_gb: p.disk as u32,
            price_monthly: Some(p.monthly_cost),
            price_hourly: Some(p.hourly_cost),
        }
    }

    fn convert_region(&self, r: VultrRegion) -> Location {
        Location {
            id: r.id.clone(),
            name: r.id,
            city: r.city,
            country: r.country,
        }
    }

    fn convert_os(&self, o: VultrOs) -> Option<Image> {
        let filtered_families = ["iso", "snapshot", "backup", "application"];
        if filtered_families.contains(&o.family.as_str()) {
            return None;
        }

        let os_version = o
            .name
            .split_whitespace()
            .nth(1)
            .map(|s| s.trim_end_matches("LTS").trim().to_string());

        Some(Image {
            id: o.id.to_string(),
            name: o.name,
            os_type: o.family,
            os_version,
        })
    }

    async fn wait_for_ssh_reachable(&self, ip: &str, timeout_secs: u64) -> anyhow::Result<bool> {
        let addr = format!("{}:22", ip);
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);

        while start.elapsed() < timeout {
            if let Ok(mut stream) = TcpStream::connect(&addr).await {
                let mut banner = [0u8; 256];
                if let Ok(Ok(n)) = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    stream.read(&mut banner),
                )
                .await
                {
                    if n > 0 {
                        let banner_str = String::from_utf8_lossy(&banner[..n]);
                        if banner_str.contains("SSH") {
                            tracing::info!("SSH reachable at {} after {:?}", addr, start.elapsed());
                            return Ok(true);
                        }
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }

        tracing::warn!("SSH not reachable at {} after {}s", addr, timeout_secs);
        Ok(false)
    }

    async fn wait_for_server_status(
        &self,
        id: &str,
        expected: ServerStatus,
        timeout_secs: u64,
    ) -> anyhow::Result<()> {
        let start = std::time::Instant::now();
        loop {
            let server = self.get_server(id).await?;
            if server.status == expected {
                return Ok(());
            }
            if start.elapsed().as_secs() >= timeout_secs {
                anyhow::bail!(
                    "Server {} did not reach '{}' status within {}s (current: '{}')",
                    id,
                    expected,
                    timeout_secs,
                    server.status
                );
            }
            tokio::time::sleep(self.poll_interval).await;
        }
    }
}

#[async_trait]
impl CloudBackend for VultrBackend {
    fn backend_type(&self) -> super::types::BackendType {
        super::types::BackendType::Vultr
    }

    async fn validate_credentials(&self) -> anyhow::Result<()> {
        let response = self
            .request_builder(reqwest::Method::GET, "/account")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        Ok(())
    }

    async fn get_catalog(&self) -> anyhow::Result<BackendCatalog> {
        let server_types = self.list_server_types().await?;
        let locations = self.list_locations().await?;
        let images = self.list_images().await?;

        Ok(BackendCatalog {
            server_types,
            locations,
            images,
        })
    }

    async fn list_server_types(&self) -> anyhow::Result<Vec<ServerType>> {
        let response = self
            .request_builder(reqwest::Method::GET, "/plans")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        let data: PlansResponse = response.json().await?;
        Ok(data
            .plans
            .into_iter()
            .map(|p| self.convert_plan(p))
            .collect())
    }

    async fn list_locations(&self) -> anyhow::Result<Vec<Location>> {
        let response = self
            .request_builder(reqwest::Method::GET, "/regions")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        let data: RegionsResponse = response.json().await?;
        Ok(data
            .regions
            .into_iter()
            .map(|r| self.convert_region(r))
            .collect())
    }

    async fn list_images(&self) -> anyhow::Result<Vec<Image>> {
        let response = self
            .request_builder(reqwest::Method::GET, "/os")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        let data: OsResponse = response.json().await?;
        Ok(data
            .os
            .into_iter()
            .filter_map(|o| self.convert_os(o))
            .collect())
    }

    async fn create_server(&self, req: CreateServerRequest) -> anyhow::Result<ProvisionResult> {
        let os_id: i64 = req.image.parse().context(format!(
            "Vultr requires numeric OS ID, got '{}'. Use Vultr OS catalog IDs (e.g., 2284 for Ubuntu 24.04).",
            req.image
        ))?;

        self.validate_offering_config(&VultrProvisionerConfig {
            plan: req.server_type.clone(),
            region: req.location.clone(),
            os_id,
        })
        .await?;

        let ssh_key_response = self
            .request_builder(reqwest::Method::POST, "/ssh-keys")
            .json(&CreateSshKeyRequest {
                name: format!("dc-{}", &req.name),
                ssh_key: req.ssh_pubkey.clone(),
            })
            .send()
            .await?;

        if !ssh_key_response.status().is_success() {
            return Err(self.handle_error(ssh_key_response).await);
        }

        let ssh_key_data: SshKeyResponse = ssh_key_response.json().await?;
        let ssh_key_id = ssh_key_data.ssh_key.id.clone();

        let instance_req = CreateInstanceRequest {
            label: req.name.clone(),
            plan: req.server_type.clone(),
            region: req.location.clone(),
            os_id,
            sshkey_id: vec![ssh_key_data.ssh_key.id],
            enable_ipv6: false,
        };

        let instance_response = self
            .request_builder(reqwest::Method::POST, "/instances")
            .json(&instance_req)
            .send()
            .await?;

        if !instance_response.status().is_success() {
            cleanup_ssh_key(self, &ssh_key_id).await;
            return Err(self.handle_error(instance_response).await);
        }

        let instance_data: InstanceResponse = instance_response.json().await?;
        let mut server = self.convert_instance(instance_data.instance);

        let mut retries = 0;
        while server.status == ServerStatus::Provisioning && retries < 60 {
            tokio::time::sleep(self.poll_interval).await;
            server = self.get_server(&server.id).await?;
            retries += 1;
        }

        if server.status != ServerStatus::Running {
            cleanup_server_and_key(self, &server.id, &ssh_key_id).await;
            anyhow::bail!("Server failed to reach running state: {:?}", server.status);
        }

        if server.public_ip.is_none() {
            let ip_wait_start = std::time::Instant::now();
            let ip_timeout = std::time::Duration::from_secs(self.ip_wait_timeout_secs);
            while server.public_ip.is_none() && ip_wait_start.elapsed() < ip_timeout {
                tokio::time::sleep(self.poll_interval).await;
                match self.get_server(&server.id).await {
                    Ok(s) => server = s,
                    Err(e) => {
                        tracing::warn!("Error polling server {} for IP: {:#}", server.id, e);
                    }
                }
            }
        }

        let ip = match server.public_ip {
            Some(ref ip) => ip.clone(),
            None => {
                cleanup_server_and_key(self, &server.id, &ssh_key_id).await;
                anyhow::bail!(
                    "Server {} reached running state but never got a public IP within {}s",
                    server.id,
                    self.ip_wait_timeout_secs
                );
            }
        };

        if self.ssh_wait_timeout_secs > 0
            && !self
                .wait_for_ssh_reachable(&ip, self.ssh_wait_timeout_secs)
                .await?
        {
            cleanup_server_and_key(self, &server.id, &ssh_key_id).await;
            anyhow::bail!("SSH port not reachable after 120s");
        }

        Ok(ProvisionResult {
            server,
            ssh_key_id: Some(ssh_key_id),
        })
    }

    async fn get_server(&self, id: &str) -> anyhow::Result<Server> {
        let response = self
            .request_builder(reqwest::Method::GET, &format!("/instances/{}", id))
            .send()
            .await?;

        if response.status() == StatusCode::NOT_FOUND {
            anyhow::bail!("Server not found: {}", id);
        }

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        let data: InstanceResponse = response.json().await?;
        Ok(self.convert_instance(data.instance))
    }

    async fn start_server(&self, id: &str) -> anyhow::Result<()> {
        let response = self
            .request_builder(reqwest::Method::POST, &format!("/instances/{}/start", id))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        self.wait_for_server_status(id, ServerStatus::Running, 120)
            .await
    }

    async fn stop_server(&self, id: &str) -> anyhow::Result<()> {
        let response = self
            .request_builder(reqwest::Method::POST, &format!("/instances/{}/halt", id))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        self.wait_for_server_status(id, ServerStatus::Stopped, 120)
            .await
    }

    async fn delete_server(&self, id: &str) -> anyhow::Result<()> {
        let response = self
            .request_builder(reqwest::Method::DELETE, &format!("/instances/{}", id))
            .send()
            .await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(());
        }

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        Ok(())
    }

    async fn get_server_metrics(&self, _id: &str) -> anyhow::Result<ServerMetrics> {
        Ok(ServerMetrics {
            cpu_percent: None,
            memory_percent: None,
            disk_percent: None,
            network_in_bytes: None,
            network_out_bytes: None,
        })
    }

    async fn delete_ssh_key(&self, key_id: &str) -> anyhow::Result<()> {
        let response = self
            .request_builder(reqwest::Method::DELETE, &format!("/ssh-keys/{}", key_id))
            .send()
            .await?;

        if !response.status().is_success() && response.status() != StatusCode::NOT_FOUND {
            tracing::warn!(
                "Failed to delete Vultr SSH key {}: {:?}",
                key_id,
                response.status()
            );
        }

        Ok(())
    }
}

async fn cleanup_server_and_key(backend: &VultrBackend, server_id: &str, ssh_key_id: &str) {
    if let Err(e) = backend.delete_server(server_id).await {
        tracing::warn!("Cleanup: failed to delete server {}: {:#}", server_id, e);
    }
    cleanup_ssh_key(backend, ssh_key_id).await;
}

async fn cleanup_ssh_key(backend: &VultrBackend, ssh_key_id: &str) {
    if let Err(e) = backend.delete_ssh_key(ssh_key_id).await {
        tracing::warn!("Cleanup: failed to delete SSH key {}: {:#}", ssh_key_id, e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_instance(status: &str, main_ip: &str) -> VultrInstance {
        VultrInstance {
            id: "abc123-def456".to_string(),
            label: "test-server".to_string(),
            status: status.to_string(),
            main_ip: main_ip.to_string(),
            plan: "vc2-1c-1gb".to_string(),
            region: "ewr".to_string(),
            os_id: 2284,
            date_created: "2024-01-01".to_string(),
        }
    }

    #[test]
    fn test_vultr_status_conversion_all_states() {
        let backend = VultrBackend::new("test_key".to_string()).unwrap();

        for status in &["pending", "installing", "resizing"] {
            let converted = backend.convert_instance(make_test_instance(status, "1.2.3.4"));
            assert_eq!(
                converted.status,
                ServerStatus::Provisioning,
                "status '{status}'"
            );
        }

        let converted = backend.convert_instance(make_test_instance("active", "1.2.3.4"));
        assert_eq!(converted.status, ServerStatus::Running);
        assert_eq!(converted.public_ip, Some("1.2.3.4".to_string()));

        for status in &["halted", "paused", "suspended"] {
            let converted = backend.convert_instance(make_test_instance(status, "1.2.3.4"));
            assert_eq!(converted.status, ServerStatus::Stopped, "status '{status}'");
        }

        let converted = backend.convert_instance(make_test_instance("destroying", "1.2.3.4"));
        assert_eq!(converted.status, ServerStatus::Deleting);

        let converted = backend.convert_instance(make_test_instance("exploded", "1.2.3.4"));
        assert_eq!(converted.status, ServerStatus::Failed);
    }

    #[test]
    fn test_vultr_zero_ip_treated_as_no_ip() {
        let backend = VultrBackend::new("test_key".to_string()).unwrap();
        let converted = backend.convert_instance(make_test_instance("active", "0.0.0.0"));
        assert_eq!(converted.public_ip, None);
    }

    #[test]
    fn test_vultr_empty_ip_treated_as_no_ip() {
        let backend = VultrBackend::new("test_key".to_string()).unwrap();
        let converted = backend.convert_instance(make_test_instance("active", ""));
        assert_eq!(converted.public_ip, None);
    }

    #[test]
    fn test_vultr_plan_conversion() {
        let backend = VultrBackend::new("test_key".to_string()).unwrap();
        let plan = VultrPlan {
            id: "vc2-1c-1gb".to_string(),
            vcpu_count: 1,
            ram: 1024,
            disk: 25,
            monthly_cost: 5.0,
            hourly_cost: 0.007,
            type_: "vc2".to_string(),
            locations: vec!["ewr".to_string(), "ams".to_string()],
        };
        let converted = backend.convert_plan(plan);
        assert_eq!(converted.id, "vc2-1c-1gb");
        assert_eq!(converted.cores, 1);
        assert_eq!(converted.memory_gb, 1.0);
        assert_eq!(converted.disk_gb, 25);
        assert_eq!(converted.price_monthly, Some(5.0));
        assert_eq!(converted.price_hourly, Some(0.007));
    }

    #[test]
    fn test_vultr_region_conversion() {
        let backend = VultrBackend::new("test_key".to_string()).unwrap();
        let region = VultrRegion {
            id: "ewr".to_string(),
            city: "Piscataway".to_string(),
            country: "US".to_string(),
        };
        let converted = backend.convert_region(region);
        assert_eq!(converted.id, "ewr");
        assert_eq!(converted.city, "Piscataway");
        assert_eq!(converted.country, "US");
    }

    #[test]
    fn test_vultr_os_conversion_filters_non_system() {
        let backend = VultrBackend::new("test_key".to_string()).unwrap();

        let iso = VultrOs {
            id: 159,
            name: "Custom".to_string(),
            arch: "x64".to_string(),
            family: "iso".to_string(),
        };
        assert!(backend.convert_os(iso).is_none());

        let snapshot = VultrOs {
            id: 164,
            name: "Snapshot".to_string(),
            arch: "x64".to_string(),
            family: "snapshot".to_string(),
        };
        assert!(backend.convert_os(snapshot).is_none());

        let backup = VultrOs {
            id: 180,
            name: "Backup".to_string(),
            arch: "x64".to_string(),
            family: "backup".to_string(),
        };
        assert!(backend.convert_os(backup).is_none());

        let app = VultrOs {
            id: 186,
            name: "Application".to_string(),
            arch: "x64".to_string(),
            family: "application".to_string(),
        };
        assert!(backend.convert_os(app).is_none());
    }

    #[test]
    fn test_vultr_os_conversion_system() {
        let backend = VultrBackend::new("test_key".to_string()).unwrap();
        let os = VultrOs {
            id: 2284,
            name: "Ubuntu 24.04 LTS x64".to_string(),
            arch: "x64".to_string(),
            family: "ubuntu".to_string(),
        };
        let converted = backend.convert_os(os).unwrap();
        assert_eq!(converted.id, "2284");
        assert_eq!(converted.name, "Ubuntu 24.04 LTS x64");
        assert_eq!(converted.os_type, "ubuntu");
    }

    #[test]
    fn test_check_plan_region_valid() {
        let plans = vec![VultrPlan {
            id: "vc2-1c-1gb".to_string(),
            vcpu_count: 1,
            ram: 1024,
            disk: 25,
            monthly_cost: 5.0,
            hourly_cost: 0.007,
            type_: "vc2".to_string(),
            locations: vec!["ewr".to_string(), "ams".to_string()],
        }];
        assert!(check_plan_region(&plans, "vc2-1c-1gb", "ewr").is_ok());
        assert!(check_plan_region(&plans, "vc2-1c-1gb", "ams").is_ok());
    }

    #[test]
    fn test_check_plan_region_wrong_region() {
        let plans = vec![VultrPlan {
            id: "vc2-1c-1gb".to_string(),
            vcpu_count: 1,
            ram: 1024,
            disk: 25,
            monthly_cost: 5.0,
            hourly_cost: 0.007,
            type_: "vc2".to_string(),
            locations: vec!["ewr".to_string()],
        }];
        let err = check_plan_region(&plans, "vc2-1c-1gb", "nrt")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("not available in region 'nrt'"),
            "unexpected: {err}"
        );
        assert!(
            err.contains("ewr"),
            "error should list available regions: {err}"
        );
    }

    #[test]
    fn test_check_plan_region_unknown_plan() {
        let plans = vec![VultrPlan {
            id: "vc2-1c-1gb".to_string(),
            vcpu_count: 1,
            ram: 1024,
            disk: 25,
            monthly_cost: 5.0,
            hourly_cost: 0.007,
            type_: "vc2".to_string(),
            locations: vec!["ewr".to_string()],
        }];
        let err = check_plan_region(&plans, "nonexistent", "ewr")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("Unknown Vultr plan 'nonexistent'"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn test_check_os_exists_valid() {
        let os_list = vec![VultrOs {
            id: 2284,
            name: "Ubuntu 24.04 LTS x64".to_string(),
            arch: "x64".to_string(),
            family: "ubuntu".to_string(),
        }];
        assert!(check_os_exists(&os_list, 2284).is_ok());
    }

    #[test]
    fn test_check_os_exists_invalid() {
        let os_list = vec![VultrOs {
            id: 2284,
            name: "Ubuntu 24.04 LTS x64".to_string(),
            arch: "x64".to_string(),
            family: "ubuntu".to_string(),
        }];
        let err = check_os_exists(&os_list, 9999).unwrap_err().to_string();
        assert!(
            err.contains("Unknown Vultr OS id '9999'"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn test_resolve_provisioner_config_explicit() {
        let config = resolve_provisioner_config(
            Some(r#"{"plan":"vc2-2c-4gb","region":"ams","os_id":1743}"#),
            "Amsterdam",
            None,
        )
        .unwrap();
        assert_eq!(config.plan, "vc2-2c-4gb");
        assert_eq!(config.region, "ams");
        assert_eq!(config.os_id, 1743);
    }

    #[test]
    fn test_resolve_provisioner_config_defaults() {
        let config = resolve_provisioner_config(None, "Piscataway", None).unwrap();
        assert_eq!(config.plan, "vc2-1c-1gb");
        assert_eq!(config.region, "piscataway");
        assert_eq!(config.os_id, 2284);
    }

    #[test]
    fn test_resolve_provisioner_config_os_name_fallback() {
        let config = resolve_provisioner_config(
            Some(r#"{"plan":"vc2-1c-1gb","region":"ewr","os_name":"ubuntu-22.04"}"#),
            "New Jersey",
            None,
        )
        .unwrap();
        assert_eq!(config.os_id, 1743);
    }

    #[test]
    fn test_resolve_provisioner_config_invalid_json() {
        let err = resolve_provisioner_config(Some("not json"), "ewr", None).unwrap_err();
        assert!(
            err.to_string().contains("Invalid provisioner_config JSON"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn test_vultr_plans_response_deserialization() {
        let json = r#"{
            "plans": [{
                "id": "vc2-1c-1gb",
                "vcpu_count": 1,
                "ram": 1024,
                "disk": 25,
                "monthly_cost": 5.0,
                "hourly_cost": 0.007,
                "type": "vc2",
                "locations": ["ewr", "ams"],
                "disk_type": "SSD",
                "disk_count": 1,
                "bandwidth": 1000,
                "monthly_cost_preemptible": 5.0,
                "hourly_cost_preemptible": 0.007,
                "invoice_type": "monthly",
                "cpu_vendor": "Intel",
                "storage_type": "local_storage",
                "vcpu_type": "thread",
                "deploy_ondemand": true,
                "deploy_preemptible": false,
                "location_cost": {}
            }]
        }"#;
        let resp: PlansResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.plans.len(), 1);
        assert_eq!(resp.plans[0].id, "vc2-1c-1gb");
        assert_eq!(resp.plans[0].type_, "vc2");
    }

    #[test]
    fn test_vultr_regions_response_deserialization() {
        let json = r#"{
            "regions": [{
                "id": "ams",
                "city": "Amsterdam",
                "country": "NL",
                "continent": "Europe",
                "options": ["ddos_protection"],
                "connectivity": ["public_ip"]
            }]
        }"#;
        let resp: RegionsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.regions.len(), 1);
        assert_eq!(resp.regions[0].id, "ams");
        assert_eq!(resp.regions[0].city, "Amsterdam");
    }

    #[test]
    fn test_vultr_os_response_deserialization() {
        let json = r#"{
            "os": [{
                "id": 2284,
                "name": "Ubuntu 24.04 LTS x64",
                "arch": "x64",
                "family": "ubuntu"
            }]
        }"#;
        let resp: OsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.os.len(), 1);
        assert_eq!(resp.os[0].id, 2284);
        assert_eq!(resp.os[0].family, "ubuntu");
    }

    #[test]
    fn test_vultr_instance_response_deserialization() {
        let json = r#"{
            "instance": {
                "id": "abc123-def456",
                "label": "my-server",
                "status": "active",
                "main_ip": "1.2.3.4",
                "plan": "vc2-1c-1gb",
                "region": "ewr",
                "os_id": 2284,
                "date_created": "2024-01-15"
            }
        }"#;
        let resp: InstanceResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.instance.id, "abc123-def456");
        assert_eq!(resp.instance.status, "active");
        assert_eq!(resp.instance.main_ip, "1.2.3.4");
    }

    #[test]
    fn test_vultr_ssh_key_response_deserialization() {
        let json = r#"{
            "ssh_key": {
                "id": "ssh-key-uuid-123",
                "name": "dc-test-server",
                "date_created": "2024-01-15T00:00:00Z",
                "ssh_key": "ssh-ed25519 AAAA..."
            }
        }"#;
        let resp: SshKeyResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.ssh_key.id, "ssh-key-uuid-123");
    }

    #[test]
    fn test_backend_type_vultr_roundtrip() {
        use crate::cloud::types::BackendType;
        assert_eq!(BackendType::Vultr.to_string(), "vultr");
        assert_eq!("vultr".parse::<BackendType>().unwrap(), BackendType::Vultr);
    }

    // ── Mockito-based HTTP mock tests ──────────────────────────────────────────

    fn vultr_instance_json(id: &str, status: &str, main_ip: &str) -> String {
        format!(
            r#"{{"id":"{}","label":"test-server","status":"{}","main_ip":"{}","plan":"vc2-1c-1gb","region":"ewr","os_id":2284,"date_created":"2024-01-15"}}"#,
            id, status, main_ip
        )
    }

    fn make_create_request() -> CreateServerRequest {
        CreateServerRequest {
            name: "test-server".to_string(),
            server_type: "vc2-1c-1gb".to_string(),
            location: "ewr".to_string(),
            image: "2284".to_string(),
            ssh_pubkey: "ssh-ed25519 AAAATEST".to_string(),
        }
    }

    async fn setup_plans_mock(server: &mut mockito::ServerGuard) -> mockito::Mock {
        server
            .mock("GET", "/plans")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"plans":[{"id":"vc2-1c-1gb","vcpu_count":1,"ram":1024,"disk":25,"monthly_cost":5.0,"hourly_cost":0.007,"type":"vc2","locations":["ewr","ams"]}]}"#)
            .create_async()
            .await
    }

    async fn setup_os_mock(server: &mut mockito::ServerGuard) -> mockito::Mock {
        server
            .mock("GET", "/os")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"os":[{"id":2284,"name":"Ubuntu 24.04 LTS x64","arch":"x64","family":"ubuntu"}]}"#)
            .create_async()
            .await
    }

    #[tokio::test]
    async fn test_create_server_ip_assigned_after_active() {
        let mut server = mockito::Server::new_async().await;

        let _plans = setup_plans_mock(&mut server).await;
        let _os = setup_os_mock(&mut server).await;

        let _ssh_key = server
            .mock("POST", "/ssh-keys")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(r#"{"ssh_key":{"id":"ssh-key-1","name":"dc-test-server"}}"#)
            .create_async()
            .await;

        let _create = server
            .mock("POST", "/instances")
            .with_status(202)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"instance":{}}}"#,
                vultr_instance_json("inst-1", "pending", "0.0.0.0")
            ))
            .create_async()
            .await;

        let _get_active_no_ip = server
            .mock("GET", "/instances/inst-1")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"instance":{}}}"#,
                vultr_instance_json("inst-1", "active", "0.0.0.0")
            ))
            .expect(1)
            .create_async()
            .await;

        let _get_active_with_ip = server
            .mock("GET", "/instances/inst-1")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"instance":{}}}"#,
                vultr_instance_json("inst-1", "active", "192.168.1.1")
            ))
            .expect(1)
            .create_async()
            .await;

        let backend = VultrBackend::new_for_mockito(server.url());
        let result = backend.create_server(make_create_request()).await;
        assert!(
            result.is_ok(),
            "create_server should succeed after IP assigned: {:?}",
            result.err()
        );

        let provision_result = result.unwrap();
        assert_eq!(provision_result.server.id, "inst-1");
        assert_eq!(
            provision_result.server.public_ip,
            Some("192.168.1.1".to_string())
        );
        assert_eq!(provision_result.ssh_key_id, Some("ssh-key-1".to_string()));
    }

    #[tokio::test]
    async fn test_create_server_ip_never_assigned_cleans_up() {
        let mut server = mockito::Server::new_async().await;

        let _plans = setup_plans_mock(&mut server).await;
        let _os = setup_os_mock(&mut server).await;

        let _ssh_key = server
            .mock("POST", "/ssh-keys")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(r#"{"ssh_key":{"id":"ssh-key-2","name":"dc-test-server"}}"#)
            .create_async()
            .await;

        let _create = server
            .mock("POST", "/instances")
            .with_status(202)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"instance":{}}}"#,
                vultr_instance_json("inst-2", "pending", "0.0.0.0")
            ))
            .create_async()
            .await;

        let _get_active_no_ip = server
            .mock("GET", "/instances/inst-2")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"instance":{}}}"#,
                vultr_instance_json("inst-2", "active", "0.0.0.0")
            ))
            .create_async()
            .await;

        let _delete_instance = server
            .mock("DELETE", "/instances/inst-2")
            .with_status(204)
            .create_async()
            .await;

        let _delete_ssh_key = server
            .mock("DELETE", "/ssh-keys/ssh-key-2")
            .with_status(204)
            .create_async()
            .await;

        let backend = VultrBackend::new_for_mockito(server.url());
        let result = backend.create_server(make_create_request()).await;
        assert!(
            result.is_err(),
            "create_server should fail when IP never assigned"
        );
        let err = format!("{:#}", result.unwrap_err());
        assert!(
            err.contains("never got a public IP"),
            "Error should mention IP wait failure: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_create_server_instance_creation_fails_cleans_up_ssh_key() {
        let mut server = mockito::Server::new_async().await;

        let _plans = setup_plans_mock(&mut server).await;
        let _os = setup_os_mock(&mut server).await;

        let _ssh_key = server
            .mock("POST", "/ssh-keys")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(r#"{"ssh_key":{"id":"ssh-key-3","name":"dc-test-server"}}"#)
            .create_async()
            .await;

        let _create_fail = server
            .mock("POST", "/instances")
            .with_status(422)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error":"Invalid plan"}"#)
            .create_async()
            .await;

        let _delete_ssh_key = server
            .mock("DELETE", "/ssh-keys/ssh-key-3")
            .with_status(204)
            .create_async()
            .await;

        let backend = VultrBackend::new_for_mockito(server.url());
        let result = backend.create_server(make_create_request()).await;
        assert!(
            result.is_err(),
            "create_server should fail on instance creation error"
        );
        let err = format!("{:#}", result.unwrap_err());
        assert!(
            err.contains("422") || err.contains("Invalid"),
            "Error should mention 422 or invalid: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_create_server_never_reaches_running_cleans_up() {
        let mut server = mockito::Server::new_async().await;

        let _plans = setup_plans_mock(&mut server).await;
        let _os = setup_os_mock(&mut server).await;

        let _ssh_key = server
            .mock("POST", "/ssh-keys")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(r#"{"ssh_key":{"id":"ssh-key-4","name":"dc-test-server"}}"#)
            .create_async()
            .await;

        let _create = server
            .mock("POST", "/instances")
            .with_status(202)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"instance":{}}}"#,
                vultr_instance_json("inst-4", "pending", "0.0.0.0")
            ))
            .create_async()
            .await;

        let _get_pending = server
            .mock("GET", "/instances/inst-4")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"instance":{}}}"#,
                vultr_instance_json("inst-4", "pending", "0.0.0.0")
            ))
            .expect(1)
            .create_async()
            .await;

        let _delete_instance = server
            .mock("DELETE", "/instances/inst-4")
            .with_status(204)
            .create_async()
            .await;

        let _delete_ssh_key = server
            .mock("DELETE", "/ssh-keys/ssh-key-4")
            .with_status(204)
            .create_async()
            .await;

        let backend = VultrBackend::new_for_mockito(server.url());
        let result = backend.create_server(make_create_request()).await;
        assert!(
            result.is_err(),
            "create_server should fail when server never reaches running"
        );
        let err = format!("{:#}", result.unwrap_err());
        assert!(
            err.contains("failed to reach running state"),
            "Error should mention running state failure: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_create_server_full_success() {
        let mut server = mockito::Server::new_async().await;

        let _plans = setup_plans_mock(&mut server).await;
        let _os = setup_os_mock(&mut server).await;

        let _ssh_key = server
            .mock("POST", "/ssh-keys")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(r#"{"ssh_key":{"id":"ssh-key-5","name":"dc-test-server"}}"#)
            .create_async()
            .await;

        let _create = server
            .mock("POST", "/instances")
            .with_status(202)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"instance":{}}}"#,
                vultr_instance_json("inst-5", "active", "203.0.113.50")
            ))
            .create_async()
            .await;

        let backend = VultrBackend::new_for_mockito(server.url());
        let result = backend.create_server(make_create_request()).await;
        assert!(
            result.is_ok(),
            "create_server should succeed with immediate IP: {:?}",
            result.err()
        );

        let provision_result = result.unwrap();
        assert_eq!(provision_result.server.id, "inst-5");
        assert_eq!(
            provision_result.server.public_ip,
            Some("203.0.113.50".to_string())
        );
        assert_eq!(provision_result.ssh_key_id, Some("ssh-key-5".to_string()));
    }

    #[tokio::test]
    async fn test_delete_server_not_found_is_ok() {
        let mut server = mockito::Server::new_async().await;

        let _mock = server
            .mock("DELETE", "/instances/gone-id")
            .with_status(404)
            .create_async()
            .await;

        let backend = VultrBackend::new_for_mockito(server.url());
        let result = backend.delete_server("gone-id").await;
        assert!(result.is_ok(), "delete_server should return Ok for 404");
    }

    #[tokio::test]
    async fn test_delete_server_success() {
        let mut server = mockito::Server::new_async().await;

        let _mock = server
            .mock("DELETE", "/instances/inst-6")
            .with_status(204)
            .create_async()
            .await;

        let backend = VultrBackend::new_for_mockito(server.url());
        let result = backend.delete_server("inst-6").await;
        assert!(result.is_ok(), "delete_server should succeed on 204");
    }

    #[tokio::test]
    async fn test_delete_server_api_error_returns_err() {
        let mut server = mockito::Server::new_async().await;

        let _mock = server
            .mock("DELETE", "/instances/inst-7")
            .with_status(500)
            .with_body(r#"{"error":"Internal error"}"#)
            .create_async()
            .await;

        let backend = VultrBackend::new_for_mockito(server.url());
        let result = backend.delete_server("inst-7").await;
        assert!(result.is_err(), "delete_server should fail on 500");
    }

    #[tokio::test]
    async fn test_get_server_success() {
        let mut server = mockito::Server::new_async().await;

        let _mock = server
            .mock("GET", "/instances/inst-8")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"instance":{}}}"#,
                vultr_instance_json("inst-8", "active", "10.0.0.1")
            ))
            .create_async()
            .await;

        let backend = VultrBackend::new_for_mockito(server.url());
        let result = backend.get_server("inst-8").await;
        assert!(result.is_ok(), "get_server should succeed");
        let srv = result.unwrap();
        assert_eq!(srv.id, "inst-8");
        assert_eq!(srv.public_ip, Some("10.0.0.1".to_string()));
    }

    #[tokio::test]
    async fn test_get_server_not_found() {
        let mut server = mockito::Server::new_async().await;

        let _mock = server
            .mock("GET", "/instances/nope")
            .with_status(404)
            .create_async()
            .await;

        let backend = VultrBackend::new_for_mockito(server.url());
        let result = backend.get_server("nope").await;
        assert!(result.is_err(), "get_server should return Err for 404");
        let err = format!("{:#}", result.unwrap_err());
        assert!(
            err.contains("not found"),
            "Error should mention not found: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_validate_credentials_success() {
        let mut server = mockito::Server::new_async().await;

        let _mock = server
            .mock("GET", "/account")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"account":{"name":"Test","email":"test@example.com"}}"#)
            .create_async()
            .await;

        let backend = VultrBackend::new_for_mockito(server.url());
        let result = backend.validate_credentials().await;
        assert!(result.is_ok(), "validate_credentials should succeed");
    }

    #[tokio::test]
    async fn test_validate_credentials_failure() {
        let mut server = mockito::Server::new_async().await;

        let _mock = server
            .mock("GET", "/account")
            .with_status(401)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error":"Invalid API key"}"#)
            .create_async()
            .await;

        let backend = VultrBackend::new_for_mockito(server.url());
        let result = backend.validate_credentials().await;
        assert!(result.is_err(), "validate_credentials should fail on 401");
    }
}
