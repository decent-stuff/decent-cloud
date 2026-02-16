//! Proxmox VE API client
//!
//! Implements the CloudBackend trait for Proxmox VE.
//!
//! API docs: https://pve.proxmox.com/wiki/Proxmox_VE_API

use anyhow::Context;
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::Deserialize;

use crate::cloud::types::{
    BackendCatalog, CreateServerRequest, Image, Location, Server, ServerMetrics, ServerStatus,
    ServerType,
};
use super::{CloudBackend, ProvisionResult};

const REQUEST_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, Deserialize)]
pub struct ProxmoxConfig {
    pub url: String,
    pub token: String,
    pub node: Option<String>,
}

pub struct ProxmoxApiBackend {
    client: Client,
    config: ProxmoxConfig,
}

impl ProxmoxApiBackend {
    pub fn new(config: ProxmoxConfig) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .danger_accept_invalid_certs(false)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client, config })
    }

    fn api_url(&self, path: &str) -> String {
        let base = self.config.url.trim_end_matches('/');
        format!("{}/api2/json{}", base, path)
    }

    fn request_builder(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.client
            .request(method, self.api_url(path))
            .header("Authorization", format!("PVEAPIToken={}", self.config.token))
            .header("Content-Type", "application/json")
    }

    async fn get_node(&self) -> anyhow::Result<String> {
        if let Some(node) = &self.config.node {
            return Ok(node.clone());
        }

        let response = self
            .request_builder(reqwest::Method::GET, "/nodes")
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to list nodes: {}", response.status());
        }

        let data: ProxmoxResponse<Vec<ProxmoxNode>> = response.json().await?;
        data.data
            .first()
            .map(|n| n.node.clone())
            .ok_or_else(|| anyhow::anyhow!("No Proxmox nodes found"))
    }
}

#[derive(Debug, Deserialize)]
struct ProxmoxResponse<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
struct ProxmoxNode {
    node: String,
    #[allow(dead_code)]
    status: String,
}

#[derive(Debug, Deserialize)]
struct ProxmoxVm {
    vmid: i64,
    name: String,
    status: String,
    #[serde(default)]
    ip: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    maxcpu: f64,
    #[serde(default)]
    #[allow(dead_code)]
    maxmem: i64,
    #[serde(default)]
    #[allow(dead_code)]
    maxdisk: i64,
}

impl ProxmoxApiBackend {
    fn convert_vm(&self, vm: ProxmoxVm, node: &str) -> Server {
        let status = match vm.status.as_str() {
            "running" => ServerStatus::Running,
            "stopped" => ServerStatus::Stopped,
            _ => ServerStatus::Provisioning,
        };

        Server {
            id: vm.vmid.to_string(),
            name: vm.name,
            status,
            public_ip: vm.ip,
            server_type: "custom".to_string(),
            location: node.to_string(),
            image: "unknown".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[async_trait]
impl CloudBackend for ProxmoxApiBackend {
    fn backend_type(&self) -> super::types::BackendType {
        super::types::BackendType::ProxmoxApi
    }

    async fn validate_credentials(&self) -> anyhow::Result<()> {
        let response = self
            .request_builder(reqwest::Method::GET, "/version")
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Proxmox API authentication failed: {}", response.status());
        }

        Ok(())
    }

    async fn get_catalog(&self) -> anyhow::Result<BackendCatalog> {
        Ok(BackendCatalog {
            server_types: self.list_server_types().await?,
            locations: self.list_locations().await?,
            images: self.list_images().await?,
        })
    }

    async fn list_server_types(&self) -> anyhow::Result<Vec<ServerType>> {
        Ok(vec![
            ServerType {
                id: "small".to_string(),
                name: "Small (2 vCPU, 4GB)".to_string(),
                cores: 2,
                memory_gb: 4.0,
                disk_gb: 32,
                price_monthly: None,
                price_hourly: None,
            },
            ServerType {
                id: "medium".to_string(),
                name: "Medium (4 vCPU, 8GB)".to_string(),
                cores: 4,
                memory_gb: 8.0,
                disk_gb: 64,
                price_monthly: None,
                price_hourly: None,
            },
            ServerType {
                id: "large".to_string(),
                name: "Large (8 vCPU, 16GB)".to_string(),
                cores: 8,
                memory_gb: 16.0,
                disk_gb: 128,
                price_monthly: None,
                price_hourly: None,
            },
        ])
    }

    async fn list_locations(&self) -> anyhow::Result<Vec<Location>> {
        let response = self
            .request_builder(reqwest::Method::GET, "/nodes")
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to list nodes: {}", response.status());
        }

        let data: ProxmoxResponse<Vec<ProxmoxNode>> = response.json().await?;
        Ok(data
            .data
            .into_iter()
            .map(|n| Location {
                id: n.node.clone(),
                name: n.node,
                city: "Datacenter".to_string(),
                country: "XX".to_string(),
            })
            .collect())
    }

    async fn list_images(&self) -> anyhow::Result<Vec<Image>> {
        let node = self.get_node().await?;

        let response = self
            .request_builder(
                reqwest::Method::GET,
                &format!("/nodes/{}/storage/local/content?content=vztmpl", node),
            )
            .send()
            .await?;

        let images = if response.status().is_success() {
            let data: ProxmoxResponse<Vec<serde_json::Value>> = response.json().await?;
            data.data
                .into_iter()
                .filter_map(|v| {
                    let volid = v.get("volid")?.as_str()?;
                    let parts: Vec<&str> = volid.split(':').collect();
                    let name = parts.get(1)?.to_string();
                    Some(Image {
                        id: volid.to_string(),
                        name,
                        os_type: "linux".to_string(),
                        os_version: None,
                    })
                })
                .collect()
        } else {
            vec![Image {
                id: "ubuntu-22.04".to_string(),
                name: "Ubuntu 22.04".to_string(),
                os_type: "linux".to_string(),
                os_version: Some("22.04".to_string()),
            }]
        };

        Ok(images)
    }

    async fn create_server(&self, _req: CreateServerRequest) -> anyhow::Result<ProvisionResult> {
        anyhow::bail!("Proxmox VM creation requires clone from template - use dc-agent for full Proxmox support")
    }

    async fn get_server(&self, id: &str) -> anyhow::Result<Server> {
        let node = self.get_node().await?;
        let vmid: i64 = id.parse().context("Invalid VM ID")?;

        let response = self
            .request_builder(
                reqwest::Method::GET,
                &format!("/nodes/{}/qemu/{}/status/current", node, vmid),
            )
            .send()
            .await?;

        if response.status() == StatusCode::NOT_FOUND {
            anyhow::bail!("VM not found: {}", id);
        }

        if !response.status().is_success() {
            anyhow::bail!("Failed to get VM: {}", response.status());
        }

        let data: ProxmoxResponse<ProxmoxVm> = response.json().await?;
        Ok(self.convert_vm(data.data, &node))
    }

    async fn start_server(&self, id: &str) -> anyhow::Result<()> {
        let node = self.get_node().await?;
        let vmid: i64 = id.parse().context("Invalid VM ID")?;

        let response = self
            .request_builder(
                reqwest::Method::POST,
                &format!("/nodes/{}/qemu/{}/status/start", node, vmid),
            )
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to start VM: {}", response.status());
        }

        Ok(())
    }

    async fn stop_server(&self, id: &str) -> anyhow::Result<()> {
        let node = self.get_node().await?;
        let vmid: i64 = id.parse().context("Invalid VM ID")?;

        let response = self
            .request_builder(
                reqwest::Method::POST,
                &format!("/nodes/{}/qemu/{}/status/stop", node, vmid),
            )
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to stop VM: {}", response.status());
        }

        Ok(())
    }

    async fn delete_server(&self, id: &str) -> anyhow::Result<()> {
        let node = self.get_node().await?;
        let vmid: i64 = id.parse().context("Invalid VM ID")?;

        let response = self
            .request_builder(
                reqwest::Method::DELETE,
                &format!("/nodes/{}/qemu/{}", node, vmid),
            )
            .send()
            .await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(());
        }

        if !response.status().is_success() {
            anyhow::bail!("Failed to delete VM: {}", response.status());
        }

        Ok(())
    }

    async fn get_server_metrics(&self, id: &str) -> anyhow::Result<ServerMetrics> {
        let node = self.get_node().await?;
        let vmid: i64 = id.parse().context("Invalid VM ID")?;

        let response = self
            .request_builder(
                reqwest::Method::GET,
                &format!("/nodes/{}/qemu/{}/status/current", node, vmid),
            )
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(ServerMetrics {
                cpu_percent: None,
                memory_percent: None,
                disk_percent: None,
                network_in_bytes: None,
                network_out_bytes: None,
            });
        }

        let data: ProxmoxResponse<ProxmoxVm> = response.json().await?;
        let vm = data.data;

        let cpu_percent = if vm.maxcpu > 0.0 {
            Some(vm.maxcpu)
        } else {
            None
        };

        let memory_percent = if vm.maxmem > 0 {
            Some(0.0)
        } else {
            None
        };

        Ok(ServerMetrics {
            cpu_percent,
            memory_percent,
            disk_percent: None,
            network_in_bytes: None,
            network_out_bytes: None,
        })
    }

    async fn delete_ssh_key(&self, _key_id: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxmox_config() {
        let config = ProxmoxConfig {
            url: "https://proxmox.example.com:8006".to_string(),
            token: "user@realm!tokenid=secret".to_string(),
            node: Some("pve1".to_string()),
        };

        assert_eq!(config.url, "https://proxmox.example.com:8006");
        assert_eq!(config.node, Some("pve1".to_string()));
    }
}
