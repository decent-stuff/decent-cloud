use super::{HealthStatus, Instance, ProvisionRequest, Provisioner};
use crate::config::ProxmoxConfig;
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize};
use std::time::Duration;

/// FNV-1a hash - stable across Rust versions (unlike DefaultHasher).
pub(crate) fn fnv1a_hash(data: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET_BASIS;
    for byte in data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

pub struct ProxmoxProvisioner {
    config: ProxmoxConfig,
    client: Client,
}

/// Proxmox API response wrapper.
#[derive(Deserialize, Debug)]
struct ProxmoxResponse<T> {
    data: T,
}

/// Task UPID response (for async operations like clone, start).
#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum TaskResponse {
    Upid(String),
    Object { upid: String },
}

impl TaskResponse {
    fn upid(self) -> String {
        match self {
            TaskResponse::Upid(upid) => upid,
            TaskResponse::Object { upid } => upid,
        }
    }
}

/// VM status response.
#[derive(Deserialize, Debug)]
struct VmStatus {
    #[allow(dead_code)]
    vmid: Option<u32>,
    status: String,
    uptime: Option<u64>,
    name: Option<String>,
}

/// Task status response.
#[derive(Deserialize, Debug)]
struct TaskStatus {
    status: String,
    exitstatus: Option<String>,
}

/// Network interfaces response from QEMU guest agent.
#[derive(Deserialize, Debug)]
struct NetworkResponse {
    result: Vec<NetworkInterface>,
}

#[derive(Deserialize, Debug)]
struct NetworkInterface {
    name: String,
    #[serde(rename = "ip-addresses")]
    ip_addresses: Option<Vec<IpAddress>>,
}

#[derive(Deserialize, Debug)]
struct IpAddress {
    #[serde(rename = "ip-address")]
    ip_address: String,
    #[serde(rename = "ip-address-type")]
    ip_address_type: String,
}

impl ProxmoxProvisioner {
    pub fn new(config: ProxmoxConfig) -> Result<Self> {
        let client = Client::builder()
            .danger_accept_invalid_certs(!config.verify_ssl)
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self { config, client })
    }

    fn auth_header(&self) -> String {
        format!(
            "PVEAPIToken={}={}",
            self.config.api_token_id, self.config.api_token_secret
        )
    }

    fn base_url(&self) -> &str {
        self.config.api_url.trim_end_matches('/')
    }

    pub(crate) fn allocate_vmid(&self, contract_id: &str) -> u32 {
        let hash = fnv1a_hash(contract_id.as_bytes());
        10000 + (hash % 990000) as u32
    }

    /// Execute a GET request to the Proxmox API.
    async fn api_get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url(), path);
        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .with_context(|| format!("Failed GET {}", path))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            bail!("GET {} failed ({}): {}", path, status, body);
        }

        let result: ProxmoxResponse<T> = response
            .json()
            .await
            .with_context(|| format!("Failed to parse GET {} response", path))?;

        Ok(result.data)
    }

    /// Execute a POST request to the Proxmox API.
    async fn api_post<T: DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url(), path);
        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .form(params)
            .send()
            .await
            .with_context(|| format!("Failed POST {}", path))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            bail!("POST {} failed ({}): {}", path, status, body);
        }

        let result: ProxmoxResponse<T> = response
            .json()
            .await
            .with_context(|| format!("Failed to parse POST {} response", path))?;

        Ok(result.data)
    }

    /// Execute a PUT request to the Proxmox API.
    async fn api_put(&self, path: &str, params: &[(&str, String)]) -> Result<()> {
        let url = format!("{}{}", self.base_url(), path);
        let response = self
            .client
            .put(&url)
            .header("Authorization", self.auth_header())
            .form(params)
            .send()
            .await
            .with_context(|| format!("Failed PUT {}", path))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            bail!("PUT {} failed ({}): {}", path, status, body);
        }

        Ok(())
    }

    /// Execute a DELETE request to the Proxmox API.
    async fn api_delete(&self, path: &str, query: &[(&str, &str)]) -> Result<()> {
        let url = format!("{}{}", self.base_url(), path);
        let response = self
            .client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .query(query)
            .send()
            .await
            .with_context(|| format!("Failed DELETE {}", path))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            bail!("DELETE {} failed ({}): {}", path, status, body);
        }

        Ok(())
    }

    async fn wait_for_task(&self, upid: &str) -> Result<()> {
        let parts: Vec<&str> = upid.split(':').collect();
        if parts.len() < 2 {
            bail!("Invalid UPID format: {}", upid);
        }
        let node = parts[1];

        let max_attempts = 60;
        let poll_interval = Duration::from_secs(5);

        for attempt in 1..=max_attempts {
            let path = format!(
                "/api2/json/nodes/{}/tasks/{}/status",
                node,
                urlencoding::encode(upid)
            );
            let task_status: TaskStatus = self.api_get(&path).await?;

            if task_status.status == "stopped" {
                return match task_status.exitstatus.as_deref() {
                    Some("OK") => Ok(()),
                    Some(exit) => bail!("Task failed with status: {}", exit),
                    None => bail!("Task stopped without exit status"),
                };
            }

            if attempt < max_attempts {
                tokio::time::sleep(poll_interval).await;
            }
        }

        bail!(
            "Task did not complete within {} seconds",
            max_attempts * poll_interval.as_secs()
        );
    }

    async fn clone_vm(&self, template_vmid: u32, new_vmid: u32, name: &str) -> Result<String> {
        let path = format!(
            "/api2/json/nodes/{}/qemu/{}/clone",
            self.config.node, template_vmid
        );

        let mut params = vec![
            ("newid", new_vmid.to_string()),
            ("name", name.to_string()),
            ("full", "1".to_string()),
            ("storage", self.config.storage.clone()),
        ];

        if let Some(pool) = &self.config.pool {
            params.push(("pool", pool.clone()));
        }

        let task_response: TaskResponse = self.api_post(&path, &params).await?;
        Ok(task_response.upid())
    }

    async fn configure_vm(&self, vmid: u32, request: &ProvisionRequest) -> Result<()> {
        let path = format!("/api2/json/nodes/{}/qemu/{}/config", self.config.node, vmid);

        let mut params = vec![("ipconfig0", "ip=dhcp".to_string())];

        if let Some(ssh_key) = &request.requester_ssh_pubkey {
            let encoded_key = urlencoding::encode(ssh_key);
            params.push(("sshkeys", encoded_key.to_string()));
        }

        if let Some(cores) = request.cpu_cores {
            params.push(("cores", cores.to_string()));
        }
        if let Some(memory_mb) = request.memory_mb {
            params.push(("memory", memory_mb.to_string()));
        }

        self.api_put(&path, &params).await?;

        // Resize disk if requested
        if let Some(storage_gb) = request.storage_gb {
            self.resize_disk(vmid, storage_gb).await?;
        }

        Ok(())
    }

    /// Resize the VM's primary disk (scsi0) to the specified size
    async fn resize_disk(&self, vmid: u32, size_gb: u32) -> Result<()> {
        let path = format!("/api2/json/nodes/{}/qemu/{}/resize", self.config.node, vmid);

        // Proxmox resize sets the disk to the specified size (not delta)
        // Use scsi0 as the primary disk (most common for cloud-init templates)
        let params = [
            ("disk", "scsi0".to_string()),
            ("size", format!("{}G", size_gb)),
        ];

        tracing::debug!("Resizing disk scsi0 on VM {} to {}GB", vmid, size_gb);
        self.api_put(&path, &params)
            .await
            .context("Failed to resize disk")?;

        Ok(())
    }

    async fn start_vm(&self, vmid: u32) -> Result<String> {
        let path = format!(
            "/api2/json/nodes/{}/qemu/{}/status/start",
            self.config.node, vmid
        );

        let task_response: TaskResponse = self.api_post(&path, &[]).await?;
        Ok(task_response.upid())
    }

    async fn stop_vm(&self, vmid: u32) -> Result<()> {
        let path = format!(
            "/api2/json/nodes/{}/qemu/{}/status/stop",
            self.config.node, vmid
        );

        let task_response: TaskResponse = self.api_post(&path, &[]).await?;
        self.wait_for_task(&task_response.upid()).await
    }

    async fn delete_vm(&self, vmid: u32) -> Result<()> {
        let path = format!("/api2/json/nodes/{}/qemu/{}", self.config.node, vmid);

        self.api_delete(
            &path,
            &[("purge", "1"), ("destroy-unreferenced-disks", "1")],
        )
        .await
    }

    async fn get_vm_status(&self, vmid: u32) -> Result<VmStatus> {
        let path = format!(
            "/api2/json/nodes/{}/qemu/{}/status/current",
            self.config.node, vmid
        );

        self.api_get(&path).await
    }

    async fn get_vm_ip(&self, vmid: u32) -> Result<(Option<String>, Option<String>)> {
        let path = format!(
            "/api2/json/nodes/{}/qemu/{}/agent/network-get-interfaces",
            self.config.node, vmid
        );

        let url = format!("{}{}", self.base_url(), path);
        let response = self
            .client
            .get(url)
            .header("Authorization", self.auth_header())
            .send()
            .await;

        let response = match response {
            Ok(r) => r,
            Err(_) => return Ok((None, None)),
        };

        if !response.status().is_success() {
            return Ok((None, None));
        }

        let net_response: ProxmoxResponse<NetworkResponse> = match response.json().await {
            Ok(r) => r,
            Err(_) => return Ok((None, None)),
        };

        let mut ipv4 = None;
        let mut ipv6 = None;

        for interface in net_response.data.result {
            if interface.name == "lo" {
                continue;
            }

            if let Some(ip_addresses) = interface.ip_addresses {
                for ip in ip_addresses {
                    if ip.ip_address_type == "ipv4"
                        && ipv4.is_none()
                        && ip.ip_address != "127.0.0.1"
                    {
                        ipv4 = Some(ip.ip_address.clone());
                    } else if ip.ip_address_type == "ipv6"
                        && ipv6.is_none()
                        && !ip.ip_address.starts_with("::1")
                        && !ip.ip_address.starts_with("fe80")
                    {
                        ipv6 = Some(ip.ip_address.clone());
                    }
                }
            }
        }

        Ok((ipv4, ipv6))
    }
}

#[async_trait]
impl Provisioner for ProxmoxProvisioner {
    async fn provision(&self, request: &ProvisionRequest) -> Result<Instance> {
        let vmid = self.allocate_vmid(&request.contract_id);
        let vm_name = format!("dc-{}", request.contract_id);

        tracing::info!(
            "Provisioning VM {} (VMID: {}) from template {}",
            vm_name,
            vmid,
            self.config.template_vmid
        );

        // Check if VM already exists (idempotency)
        if let Ok(status) = self.get_vm_status(vmid).await {
            tracing::info!(
                "VM {} already exists (status: {}), returning existing instance",
                vmid,
                status.status
            );
            // If VM exists but is not running, start it
            if status.status != "running" {
                tracing::debug!("Starting existing VM {}", vmid);
                if let Ok(upid) = self.start_vm(vmid).await {
                    let _ = self.wait_for_task(&upid).await;
                }
            }
            // Get IP and return instance
            let (ipv4, ipv6) = self.get_vm_ip(vmid).await.unwrap_or((None, None));
            return Ok(Instance {
                external_id: vmid.to_string(),
                ip_address: ipv4,
                ipv6_address: ipv6,
                ssh_port: 22,
                root_password: None,
                additional_details: Some(serde_json::json!({
                    "vmid": vmid,
                    "node": self.config.node,
                    "name": vm_name,
                    "reused": true,
                })),
            });
        }

        // Step 1: Clone template
        tracing::debug!(
            "Cloning template {} to VMID {}",
            self.config.template_vmid,
            vmid
        );
        let clone_upid = self
            .clone_vm(self.config.template_vmid, vmid, &vm_name)
            .await
            .context("Failed to clone VM")?;

        tracing::debug!("Waiting for clone task: {}", clone_upid);
        self.wait_for_task(&clone_upid)
            .await
            .context("Clone task failed")?;

        // Step 2: Configure VM
        tracing::debug!("Configuring VM {}", vmid);
        self.configure_vm(vmid, request)
            .await
            .context("Failed to configure VM")?;

        // Step 3: Start VM
        tracing::debug!("Starting VM {}", vmid);
        let start_upid = self.start_vm(vmid).await.context("Failed to start VM")?;

        tracing::debug!("Waiting for start task: {}", start_upid);
        self.wait_for_task(&start_upid)
            .await
            .context("Start task failed")?;

        // Step 4: Wait for IP
        tracing::debug!("Waiting for VM to boot and obtain IP address");
        let mut ipv4 = None;
        let mut ipv6 = None;

        for attempt in 1..=12 {
            tokio::time::sleep(Duration::from_secs(10)).await;

            match self.get_vm_ip(vmid).await {
                Ok((v4, v6)) if v4.is_some() || v6.is_some() => {
                    ipv4 = v4;
                    ipv6 = v6;
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::debug!("Failed to get IP on attempt {}: {}", attempt, e);
                }
            }

            if attempt == 12 {
                tracing::warn!(
                    "VM {} started but no IP address obtained after 2 minutes",
                    vmid
                );
            }
        }

        let instance = Instance {
            external_id: vmid.to_string(),
            ip_address: ipv4,
            ipv6_address: ipv6,
            ssh_port: 22,
            root_password: None,
            additional_details: Some(serde_json::json!({
                "vmid": vmid,
                "node": self.config.node,
                "name": vm_name,
            })),
        };

        tracing::info!(
            "Successfully provisioned VM {} with IP: {:?}",
            vmid,
            instance.ip_address
        );

        Ok(instance)
    }

    async fn terminate(&self, external_id: &str) -> Result<()> {
        let vmid: u32 = external_id.parse().context("Invalid VMID format")?;

        tracing::info!("Terminating VM {}", vmid);

        match self.get_vm_status(vmid).await {
            Ok(status) => {
                if status.status == "running" {
                    tracing::debug!("Stopping VM {}", vmid);
                    self.stop_vm(vmid).await.context("Failed to stop VM")?;
                }
            }
            Err(e) => {
                let err_str = e.to_string();
                // Check for 404 or 500 status codes (may appear as "500" or "500 ")
                if err_str.contains("(500") || err_str.contains("(404") {
                    tracing::warn!("VM {} not found, assuming already deleted", vmid);
                    return Ok(());
                }
                return Err(e);
            }
        }

        tracing::debug!("Deleting VM {}", vmid);
        self.delete_vm(vmid).await.context("Failed to delete VM")?;

        tracing::info!("Successfully terminated VM {}", vmid);

        Ok(())
    }

    async fn health_check(&self, external_id: &str) -> Result<HealthStatus> {
        let vmid: u32 = external_id.parse().context("Invalid VMID format")?;

        match self.get_vm_status(vmid).await {
            Ok(status) => {
                if status.status == "running" {
                    Ok(HealthStatus::Healthy {
                        uptime_seconds: status.uptime.unwrap_or(0),
                    })
                } else {
                    Ok(HealthStatus::Unhealthy {
                        reason: format!("VM status: {}", status.status),
                    })
                }
            }
            Err(e) => {
                let err_str = e.to_string();
                // Check for 404 or 500 status codes (may appear as "500" or "500 ")
                if err_str.contains("(500") || err_str.contains("(404") {
                    Ok(HealthStatus::Unhealthy {
                        reason: "VM not found".to_string(),
                    })
                } else {
                    Ok(HealthStatus::Unknown)
                }
            }
        }
    }

    async fn get_instance(&self, external_id: &str) -> Result<Option<Instance>> {
        let vmid: u32 = external_id.parse().context("Invalid VMID format")?;

        match self.get_vm_status(vmid).await {
            Ok(status) => {
                let (ipv4, ipv6) = self.get_vm_ip(vmid).await.unwrap_or((None, None));

                Ok(Some(Instance {
                    external_id: vmid.to_string(),
                    ip_address: ipv4,
                    ipv6_address: ipv6,
                    ssh_port: 22,
                    root_password: None,
                    additional_details: Some(serde_json::json!({
                        "vmid": vmid,
                        "node": self.config.node,
                        "name": status.name,
                        "status": status.status,
                        "uptime": status.uptime,
                    })),
                }))
            }
            Err(e) => {
                let err_str = e.to_string();
                // Check for 404 or 500 status codes (may appear as "500" or "500 ")
                if err_str.contains("(500") || err_str.contains("(404") {
                    Ok(None)
                } else {
                    Err(e)
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "proxmox_tests.rs"]
mod proxmox_tests;
