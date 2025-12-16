use super::{HealthStatus, Instance, ProvisionRequest, Provisioner};
use crate::config::ProxmoxConfig;
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

pub struct ProxmoxProvisioner {
    config: ProxmoxConfig,
    client: Client,
}

// Proxmox API response wrapper
#[derive(Deserialize, Debug)]
struct ProxmoxResponse<T> {
    data: T,
}

// Task UPID response (for async operations like clone, start)
#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum TaskResponse {
    Upid(String),
    Object { upid: String },
}

// VM status response
#[derive(Deserialize, Debug)]
struct VmStatus {
    #[allow(dead_code)]
    vmid: Option<u32>,
    status: String, // "running" or "stopped"
    uptime: Option<u64>,
    name: Option<String>,
}

// Task status response (for polling async operations)
#[derive(Deserialize, Debug)]
struct TaskStatus {
    status: String, // "running" or "stopped"
    exitstatus: Option<String>, // "OK" when successful, "some error" on failure
}

// Network interfaces response from QEMU guest agent
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
    ip_address_type: String, // "ipv4" or "ipv6"
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

    fn allocate_vmid(&self, contract_id: &str) -> u32 {
        // Generate deterministic VMID from contract_id hash
        // Range: 100-999999 (Proxmox valid range, avoiding template range)
        let mut hasher = DefaultHasher::new();
        contract_id.hash(&mut hasher);
        let hash = hasher.finish();

        // Map to range 10000-999999 to avoid conflicts with templates
        let vmid = 10000 + (hash % 990000) as u32;
        vmid
    }

    async fn wait_for_task(&self, upid: &str) -> Result<()> {
        // Parse UPID to extract node name
        // Format: UPID:node:pid:pstart:starttime:type:id:user:
        let parts: Vec<&str> = upid.split(':').collect();
        if parts.len() < 2 {
            bail!("Invalid UPID format: {}", upid);
        }
        let node = parts[1];

        // Poll task status until complete (max 5 minutes)
        let max_attempts = 60; // 5 minutes with 5 second intervals
        let poll_interval = Duration::from_secs(5);

        for attempt in 1..=max_attempts {
            let url = format!(
                "{}/api2/json/nodes/{}/tasks/{}/status",
                self.base_url(),
                node,
                urlencoding::encode(upid)
            );

            let response = self
                .client
                .get(&url)
                .header("Authorization", self.auth_header())
                .send()
                .await
                .context("Failed to get task status")?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                bail!("Task status request failed ({}): {}", status, body);
            }

            let task_response: ProxmoxResponse<TaskStatus> = response
                .json()
                .await
                .context("Failed to parse task status response")?;

            let task_status = task_response.data;

            if task_status.status == "stopped" {
                if let Some(exitstatus) = &task_status.exitstatus {
                    if exitstatus == "OK" {
                        return Ok(());
                    } else {
                        bail!("Task failed with status: {}", exitstatus);
                    }
                } else {
                    bail!("Task stopped without exit status");
                }
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
        let url = format!(
            "{}/api2/json/nodes/{}/qemu/{}/clone",
            self.base_url(),
            self.config.node,
            template_vmid
        );

        let mut params = vec![
            ("newid", new_vmid.to_string()),
            ("name", name.to_string()),
            ("full", "1".to_string()), // Full clone
            ("storage", self.config.storage.clone()),
        ];

        if let Some(pool) = &self.config.pool {
            params.push(("pool", pool.clone()));
        }

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .form(&params)
            .send()
            .await
            .context("Failed to send clone request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("Clone request failed ({}): {}", status, body);
        }

        let clone_response: ProxmoxResponse<TaskResponse> = response
            .json()
            .await
            .context("Failed to parse clone response")?;

        let upid = match clone_response.data {
            TaskResponse::Upid(upid) => upid,
            TaskResponse::Object { upid } => upid,
        };

        Ok(upid)
    }

    async fn configure_vm(&self, vmid: u32, request: &ProvisionRequest) -> Result<()> {
        let url = format!(
            "{}/api2/json/nodes/{}/qemu/{}/config",
            self.base_url(),
            self.config.node,
            vmid
        );

        let mut params = vec![
            ("ipconfig0", "ip=dhcp".to_string()), // DHCP by default
        ];

        // Set SSH keys if provided
        if let Some(ssh_key) = &request.requester_ssh_pubkey {
            let encoded_key = urlencoding::encode(ssh_key);
            params.push(("sshkeys", encoded_key.to_string()));
        }

        // Set resources if specified
        if let Some(cores) = request.cpu_cores {
            params.push(("cores", cores.to_string()));
        }
        if let Some(memory_mb) = request.memory_mb {
            params.push(("memory", memory_mb.to_string()));
        }

        let response = self
            .client
            .put(&url)
            .header("Authorization", self.auth_header())
            .form(&params)
            .send()
            .await
            .context("Failed to send config request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("Config request failed ({}): {}", status, body);
        }

        Ok(())
    }

    async fn start_vm(&self, vmid: u32) -> Result<String> {
        let url = format!(
            "{}/api2/json/nodes/{}/qemu/{}/status/start",
            self.base_url(),
            self.config.node,
            vmid
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .context("Failed to send start request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("Start request failed ({}): {}", status, body);
        }

        let start_response: ProxmoxResponse<TaskResponse> = response
            .json()
            .await
            .context("Failed to parse start response")?;

        let upid = match start_response.data {
            TaskResponse::Upid(upid) => upid,
            TaskResponse::Object { upid } => upid,
        };

        Ok(upid)
    }

    async fn stop_vm(&self, vmid: u32) -> Result<()> {
        let url = format!(
            "{}/api2/json/nodes/{}/qemu/{}/status/stop",
            self.base_url(),
            self.config.node,
            vmid
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .context("Failed to send stop request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("Stop request failed ({}): {}", status, body);
        }

        // Wait for stop to complete
        let stop_response: ProxmoxResponse<TaskResponse> = response
            .json()
            .await
            .context("Failed to parse stop response")?;

        let upid = match stop_response.data {
            TaskResponse::Upid(upid) => upid,
            TaskResponse::Object { upid } => upid,
        };

        self.wait_for_task(&upid).await?;

        Ok(())
    }

    async fn delete_vm(&self, vmid: u32) -> Result<()> {
        let url = format!(
            "{}/api2/json/nodes/{}/qemu/{}",
            self.base_url(),
            self.config.node,
            vmid
        );

        let params = vec![
            ("purge", "1"),
            ("destroy-unreferenced-disks", "1"),
        ];

        let response = self
            .client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .query(&params)
            .send()
            .await
            .context("Failed to send delete request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("Delete request failed ({}): {}", status, body);
        }

        Ok(())
    }

    async fn get_vm_status(&self, vmid: u32) -> Result<VmStatus> {
        let url = format!(
            "{}/api2/json/nodes/{}/qemu/{}/status/current",
            self.base_url(),
            self.config.node,
            vmid
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .context("Failed to get VM status")?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 404 {
                bail!("VM not found");
            }
            let body = response.text().await.unwrap_or_default();
            bail!("Status request failed ({}): {}", status, body);
        }

        let status_response: ProxmoxResponse<VmStatus> = response
            .json()
            .await
            .context("Failed to parse status response")?;

        Ok(status_response.data)
    }

    async fn get_vm_ip(&self, vmid: u32) -> Result<(Option<String>, Option<String>)> {
        let url = format!(
            "{}/api2/json/nodes/{}/qemu/{}/agent/network-get-interfaces",
            self.base_url(),
            self.config.node,
            vmid
        );

        // Try to get IP from QEMU guest agent
        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await;

        // If guest agent is not available, return None for IPs
        let response = match response {
            Ok(r) => r,
            Err(_) => return Ok((None, None)),
        };

        if !response.status().is_success() {
            // Guest agent not available or not running yet
            return Ok((None, None));
        }

        let net_response: ProxmoxResponse<NetworkResponse> = match response.json().await {
            Ok(r) => r,
            Err(_) => return Ok((None, None)),
        };

        let mut ipv4 = None;
        let mut ipv6 = None;

        // Find first non-loopback IP addresses
        for interface in net_response.data.result {
            if interface.name == "lo" {
                continue;
            }

            if let Some(ip_addresses) = interface.ip_addresses {
                for ip in ip_addresses {
                    if ip.ip_address_type == "ipv4" && ipv4.is_none() && ip.ip_address != "127.0.0.1" {
                        ipv4 = Some(ip.ip_address.clone());
                    } else if ip.ip_address_type == "ipv6" && ipv6.is_none() && !ip.ip_address.starts_with("::1") && !ip.ip_address.starts_with("fe80") {
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

        // Step 1: Clone template to new VMID
        tracing::debug!("Cloning template {} to VMID {}", self.config.template_vmid, vmid);
        let clone_upid = self.clone_vm(self.config.template_vmid, vmid, &vm_name).await
            .context("Failed to clone VM")?;

        tracing::debug!("Waiting for clone task to complete: {}", clone_upid);
        self.wait_for_task(&clone_upid).await
            .context("Clone task failed")?;

        // Step 2: Configure VM (cloud-init, resources)
        tracing::debug!("Configuring VM {}", vmid);
        self.configure_vm(vmid, request).await
            .context("Failed to configure VM")?;

        // Step 3: Start VM
        tracing::debug!("Starting VM {}", vmid);
        let start_upid = self.start_vm(vmid).await
            .context("Failed to start VM")?;

        tracing::debug!("Waiting for start task to complete: {}", start_upid);
        self.wait_for_task(&start_upid).await
            .context("Start task failed")?;

        // Step 4: Wait for VM to boot and get IP (with retries)
        tracing::debug!("Waiting for VM to boot and obtain IP address");
        let mut ipv4 = None;
        let mut ipv6 = None;

        for attempt in 1..=12 {
            tokio::time::sleep(Duration::from_secs(10)).await;

            match self.get_vm_ip(vmid).await {
                Ok((v4, v6)) => {
                    if v4.is_some() {
                        ipv4 = v4;
                        ipv6 = v6;
                        break;
                    }
                }
                Err(e) => {
                    tracing::debug!("Failed to get IP on attempt {}: {}", attempt, e);
                }
            }

            if attempt == 12 {
                tracing::warn!("VM {} started but no IP address obtained after 2 minutes", vmid);
            }
        }

        let instance = Instance {
            external_id: vmid.to_string(),
            ip_address: ipv4,
            ipv6_address: ipv6,
            ssh_port: 22,
            root_password: None, // Proxmox cloud-init doesn't return password
            additional_details: Some(serde_json::json!({
                "vmid": vmid,
                "node": self.config.node,
                "name": vm_name,
            })),
        };

        tracing::info!("Successfully provisioned VM {} with IP: {:?}", vmid, instance.ip_address);

        Ok(instance)
    }

    async fn terminate(&self, external_id: &str) -> Result<()> {
        let vmid: u32 = external_id.parse()
            .context("Invalid VMID format")?;

        tracing::info!("Terminating VM {}", vmid);

        // Check if VM exists and is running
        match self.get_vm_status(vmid).await {
            Ok(status) => {
                if status.status == "running" {
                    tracing::debug!("Stopping VM {}", vmid);
                    self.stop_vm(vmid).await
                        .context("Failed to stop VM")?;
                }
            }
            Err(e) => {
                if e.to_string().contains("VM not found") {
                    tracing::warn!("VM {} not found, assuming already deleted", vmid);
                    return Ok(());
                }
                return Err(e);
            }
        }

        // Delete VM
        tracing::debug!("Deleting VM {}", vmid);
        self.delete_vm(vmid).await
            .context("Failed to delete VM")?;

        tracing::info!("Successfully terminated VM {}", vmid);

        Ok(())
    }

    async fn health_check(&self, external_id: &str) -> Result<HealthStatus> {
        let vmid: u32 = external_id.parse()
            .context("Invalid VMID format")?;

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
                if e.to_string().contains("VM not found") {
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
        let vmid: u32 = external_id.parse()
            .context("Invalid VMID format")?;

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
                if e.to_string().contains("VM not found") {
                    Ok(None)
                } else {
                    Err(e)
                }
            }
        }
    }
}
