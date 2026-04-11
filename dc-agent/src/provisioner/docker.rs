use super::{
    extract_contract_id, HealthStatus, Instance, ProvisionRequest, Provisioner, RunningInstance,
    SetupVerification,
};
use crate::config::DockerConfig;
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use bollard::container::{
    Config, CreateContainerOptions, InspectContainerOptions, ListContainersOptions,
    RemoveContainerOptions, StartContainerOptions, StopContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::models::{HostConfig, PortBinding};
use bollard::service::ContainerInspectResponse;
use bollard::Docker;
use futures::StreamExt;
use std::collections::HashMap;
use tracing;

// ── procfs helpers ─────────────────────────────────────────────────────────

/// Parse the CPU model from `/proc/cpuinfo` content.
///
/// Tries fields in order:
/// 1. `model name` — x86 and AArch64 kernels
/// 2. `Hardware` — ARM32 boards (e.g. "BCM2835")
/// 3. `Processor` — older ARM kernels (e.g. "ARMv7 Processor rev 3")
pub(crate) fn parse_cpu_model(cpuinfo: &str) -> Option<String> {
    for line in cpuinfo.lines() {
        if let Some(rest) = line.strip_prefix("model name") {
            let after_colon = rest.trim_start_matches(['\t', ' ']);
            if let Some(value) = after_colon.strip_prefix(':') {
                let model = value.trim().to_string();
                if !model.is_empty() {
                    return Some(model);
                }
            }
        }
    }
    for line in cpuinfo.lines() {
        if let Some(rest) = line.strip_prefix("Hardware") {
            let after_colon = rest.trim_start_matches(['\t', ' ']);
            if let Some(value) = after_colon.strip_prefix(':') {
                let model = value.trim().to_string();
                if !model.is_empty() {
                    return Some(model);
                }
            }
        }
    }
    for line in cpuinfo.lines() {
        if let Some(rest) = line.strip_prefix("Processor") {
            let after_colon = rest.trim_start_matches(['\t', ' ']);
            if let Some(value) = after_colon.strip_prefix(':') {
                let model = value.trim().to_string();
                if !model.is_empty() {
                    return Some(model);
                }
            }
        }
    }
    None
}

/// Parse "MemAvailable" from `/proc/meminfo` content, returning megabytes.
///
/// `/proc/meminfo` reports values in kB; we convert to MB.
pub(crate) fn parse_mem_available_mb(meminfo: &str) -> Option<u64> {
    for line in meminfo.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            // format: "MemAvailable:   12345678 kB"
            let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb / 1024);
        }
    }
    None
}

/// Read the filesystem total and available space (in bytes) for `path` via
/// statvfs.  Returns `(total_bytes, avail_bytes)`.
fn fs_stats(path: &str) -> Option<(u64, u64)> {
    let stat = nix::sys::statvfs::statvfs(path).ok()?;
    Some((
        stat.blocks() * stat.fragment_size(),
        stat.blocks_available() * stat.fragment_size(),
    ))
}

fn container_name(contract_id: &str) -> String {
    format!("dc-{}", contract_id)
}

fn is_docker_not_found(e: &bollard::errors::Error) -> bool {
    matches!(
        e,
        bollard::errors::Error::DockerResponseServerError {
            status_code: 404,
            ..
        }
    )
}

pub struct DockerProvisioner {
    config: DockerConfig,
    client: Docker,
}

impl DockerProvisioner {
    pub fn new(config: DockerConfig) -> Result<Self> {
        let client = if config.socket_path.is_empty() {
            Docker::connect_with_local_defaults()
                .context("Failed to connect to Docker daemon via default socket")?
        } else {
            Docker::connect_with_local(&config.socket_path, 120, bollard::API_DEFAULT_VERSION)
                .with_context(|| format!("Failed to connect to Docker at {}", config.socket_path))?
        };

        Ok(Self { config, client })
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

    async fn pull_image_if_needed(&self, image: &str) -> Result<()> {
        let images = self
            .client
            .list_images::<String>(None)
            .await
            .context("Failed to list Docker images to check local cache")?;
        let already_present = images
            .iter()
            .any(|img| img.repo_tags.iter().any(|t| t == image));

        if already_present {
            tracing::debug!(image, "Image already present locally");
            return Ok(());
        }

        tracing::info!(image, "Pulling image");
        let create_options = CreateImageOptions::<String> {
            from_image: image.to_string(),
            ..Default::default()
        };

        let mut stream = self.client.create_image(Some(create_options), None, None);
        while let Some(item) = stream.next().await {
            match item {
                Ok(_) => {}
                Err(e) => bail!("Failed to pull image '{}': {}", image, e),
            }
        }

        Ok(())
    }

    fn build_container_config(&self, request: &ProvisionRequest, image: &str) -> Config<String> {
        let ssh_port_str = self.config.ssh_port.to_string();

        let mut exposed_ports = HashMap::new();
        exposed_ports.insert(ssh_port_str.clone(), HashMap::new());

        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            ssh_port_str,
            Some(vec![PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: Some("0".to_string()),
            }]),
        );

        let mut env = Vec::new();

        if let Some(ssh_key) = &request.requester_ssh_pubkey {
            env.push(format!("SSH_PUBLIC_KEY={}", ssh_key));
        }

        let cmd = Some(vec![
            "/bin/bash".to_string(),
            "-c".to_string(),
            concat!(
                "set -e; ",
                "mkdir -p /root/.ssh && chmod 700 /root/.ssh; ",
                r#"[ -n "$SSH_PUBLIC_KEY" ] && printf '%s\n' "$SSH_PUBLIC_KEY" > /root/.ssh/authorized_keys && chmod 600 /root/.ssh/authorized_keys; "#,
                "mkdir -p /run/sshd; ",
                "exec /usr/sbin/sshd -D -e"
            ).to_string(),
        ]);

        let mut labels = HashMap::new();
        labels.insert("dc-agent".to_string(), "true".to_string());
        labels.insert("dc-contract-id".to_string(), request.contract_id.clone());

        let cpu_count = request.cpu_cores.unwrap_or(1) as i64;
        let memory_bytes = request.memory_mb.unwrap_or(512) as i64 * 1024 * 1024;

        let host_config = HostConfig {
            cpu_count: Some(cpu_count),
            memory: Some(memory_bytes),
            port_bindings: Some(port_bindings),
            network_mode: Some(self.config.network.clone()),
            restart_policy: Some(bollard::service::RestartPolicy {
                name: Some(bollard::service::RestartPolicyNameEnum::UNLESS_STOPPED),
                ..Default::default()
            }),
            ..Default::default()
        };

        Config {
            image: Some(image.to_string()),
            exposed_ports: Some(exposed_ports),
            env: Some(env),
            cmd,
            labels: Some(labels),
            host_config: Some(host_config),
            ..Default::default()
        }
    }

    fn extract_ipv6_address(&self, inspect: &ContainerInspectResponse) -> Option<String> {
        let network_settings = inspect.network_settings.as_ref()?;

        // PoC: real Docker inspect data exposes IPv6 on the per-network endpoint,
        // so prefer the configured network and only fall back to Docker's deprecated field.
        network_settings
            .networks
            .as_ref()
            .and_then(|networks| networks.get(&self.config.network))
            .and_then(|network| network.global_ipv6_address.as_ref())
            .filter(|ip| !ip.is_empty())
            .cloned()
            .or_else(|| {
                network_settings
                    .global_ipv6_address
                    .as_ref()
                    .filter(|ip| !ip.is_empty())
                    .cloned()
            })
    }

    fn container_to_instance(
        &self,
        inspect: &ContainerInspectResponse,
        id: &str,
    ) -> Option<Instance> {
        let name = match inspect.name.as_ref() {
            Some(n) => n.trim_start_matches('/').to_string(),
            None => return None, // Completely empty inspect response; Docker always sets a name
        };

        let ip = inspect
            .network_settings
            .as_ref()
            .and_then(|ns| ns.ip_address.as_ref())
            .filter(|ip| !ip.is_empty())
            .cloned();

        let host_ssh_port = inspect
            .network_settings
            .as_ref()
            .and_then(|ns| ns.ports.as_ref())
            .and_then(|ports| {
                ports
                    .iter()
                    .filter_map(|(key, bindings)| {
                        let container_port: u16 = key.split('/').next()?.parse().ok()?;
                        if container_port == self.config.ssh_port {
                            bindings
                                .as_ref()?
                                .iter()
                                .next()?
                                .host_port
                                .as_ref()?
                                .parse()
                                .ok()
                        } else {
                            None
                        }
                    })
                    .next()
            });

        Some(Instance {
            external_id: id.to_string(),
            ip_address: ip,
            ipv6_address: self.extract_ipv6_address(inspect),
            public_ip: None,
            ssh_port: host_ssh_port.unwrap_or(self.config.ssh_port),
            root_password: None,
            additional_details: Some(serde_json::json!({
                "name": name,
                "image": inspect.config.as_ref().and_then(|c| c.image.clone()),
            })),
            gateway_slug: None,
            gateway_subdomain: None,
            gateway_ssh_port: None,
            gateway_port_range_start: None,
            gateway_port_range_end: None,
        })
    }

    async fn find_container_by_name(&self, name: &str) -> Result<Option<String>> {
        let filters = HashMap::from([("name".to_string(), vec![name.to_string()])]);
        let options = ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        };

        let containers = self.client.list_containers(Some(options)).await?;
        Ok(containers.into_iter().next().and_then(|c| c.id))
    }
}

#[async_trait]
impl Provisioner for DockerProvisioner {
    async fn provision(&self, request: &ProvisionRequest) -> Result<Instance> {
        let name = container_name(&request.contract_id);
        let image = self.resolve_image(request);

        tracing::info!(
            contract_id = %request.contract_id,
            image = %image,
            "Provisioning Docker container"
        );

        let existing_id = self.find_container_by_name(&name).await?;
        if let Some(id) = existing_id {
            tracing::info!(name = %name, "Container already exists, checking state");

            let inspect = self
                .client
                .inspect_container(&id, None::<InspectContainerOptions>)
                .await
                .with_context(|| format!("Failed to inspect existing container {}", id))?;

            let running = inspect
                .state
                .as_ref()
                .and_then(|s| s.running)
                .unwrap_or(false);

            if !running {
                tracing::info!(id = %id, "Starting existing stopped container");
                self.client
                    .start_container(&id, None::<StartContainerOptions<String>>)
                    .await
                    .with_context(|| format!("Failed to start container {}", id))?;
            }

            let instance = self
                .container_to_instance(&inspect, &id)
                .context("Failed to build instance from existing container")?;

            tracing::info!(id = %id, "Returning existing container");
            return Ok(instance);
        }

        self.pull_image_if_needed(&image).await?;

        let container_config = self.build_container_config(request, &image);

        let create_options = CreateContainerOptions {
            name: name.clone(),
            platform: None,
        };

        let create_result = self
            .client
            .create_container(Some(create_options), container_config)
            .await
            .with_context(|| format!("Failed to create container with image '{}'", image))?;

        let id = create_result.id;
        tracing::info!(id = %id, "Container created");

        self.client
            .start_container(&id, None::<StartContainerOptions<String>>)
            .await
            .with_context(|| format!("Failed to start container {}", id))?;

        tracing::info!(id = %id, "Container started");

        let inspect = self
            .client
            .inspect_container(&id, None::<InspectContainerOptions>)
            .await
            .with_context(|| format!("Failed to inspect container {} after start", id))?;

        let instance = self
            .container_to_instance(&inspect, &id)
            .context("Failed to build instance from container")?;

        tracing::info!(
            id = %id,
            ip = ?instance.ip_address,
            ssh_port = instance.ssh_port,
            "Container provisioned successfully"
        );

        Ok(instance)
    }

    async fn terminate(&self, external_id: &str) -> Result<()> {
        tracing::info!(id = %external_id, "Terminating Docker container");

        let inspect = match self
            .client
            .inspect_container(external_id, None::<InspectContainerOptions>)
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                if is_docker_not_found(&e) {
                    tracing::warn!(id = %external_id, "Container not found, assuming already removed");
                    return Ok(());
                }
                return Err(e)
                    .with_context(|| format!("Failed to inspect container {}", external_id));
            }
        };

        let running = inspect
            .state
            .as_ref()
            .and_then(|s| s.running)
            .unwrap_or(false);

        if running {
            self.client
                .stop_container(external_id, Some(StopContainerOptions { t: 30 }))
                .await
                .with_context(|| format!("Failed to stop container {}", external_id))?;
        }

        self.client
            .remove_container(
                external_id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
            .with_context(|| format!("Failed to remove container {}", external_id))?;

        tracing::info!(id = %external_id, "Container terminated successfully");
        Ok(())
    }

    async fn health_check(&self, external_id: &str) -> Result<HealthStatus> {
        let inspect = match self
            .client
            .inspect_container(external_id, None::<InspectContainerOptions>)
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                if is_docker_not_found(&e) {
                    return Ok(HealthStatus::Unhealthy {
                        reason: "Container not found".to_string(),
                    });
                }
                return Err(e).context("Health check failed");
            }
        };

        let state = inspect.state.as_ref();
        let running = state.and_then(|s| s.running).unwrap_or(false);

        if running {
            let started_at = state
                .and_then(|s| s.started_at.as_deref())
                .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok());

            let uptime_seconds = started_at
                .map(|t| {
                    t.signed_duration_since(chrono::Utc::now())
                        .num_seconds()
                        .unsigned_abs()
                })
                .unwrap_or(0);

            Ok(HealthStatus::Healthy { uptime_seconds })
        } else {
            let status = state
                .and_then(|s| s.status)
                .map(|s| format!("{:?}", s))
                .unwrap_or_else(|| "unknown".to_string());
            let exit_code = state.and_then(|s| s.exit_code);
            let reason = match exit_code {
                Some(code) => format!("Container status: {}, exit code: {}", status, code),
                None => format!("Container status: {}", status),
            };
            Ok(HealthStatus::Unhealthy { reason })
        }
    }

    async fn get_instance(&self, external_id: &str) -> Result<Option<Instance>> {
        let inspect = match self
            .client
            .inspect_container(external_id, None::<InspectContainerOptions>)
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                if is_docker_not_found(&e) {
                    return Ok(None);
                }
                return Err(e).context("Failed to get instance");
            }
        };

        Ok(self.container_to_instance(&inspect, external_id))
    }

    async fn list_running_instances(&self) -> Result<Vec<RunningInstance>> {
        let filters = HashMap::from([("label".to_string(), vec!["dc-agent=true".to_string()])]);
        let options = ListContainersOptions {
            all: false,
            filters,
            ..Default::default()
        };

        let containers = self.client.list_containers(Some(options)).await?;
        let mut instances = Vec::new();

        for container in containers {
            let names = container.names.unwrap_or_default();
            let name = names
                .first()
                .map(|n| n.trim_start_matches('/').to_string())
                .unwrap_or_default();

            let contract_id = extract_contract_id(&name);
            let id = container.id.unwrap_or_default();

            instances.push(RunningInstance {
                external_id: id,
                contract_id,
            });
        }

        Ok(instances)
    }

    async fn collect_resources(&self) -> Option<crate::api_client::ResourceInventory> {
        use crate::api_client::{ResourceInventory, StoragePoolInfo};

        let info = match self.client.info().await {
            Ok(info) => info,
            Err(e) => {
                tracing::warn!(error = ?e, "Failed to get Docker info for resource inventory");
                return None;
            }
        };

        let cpu_threads = info.ncpu.unwrap_or(0) as u32;
        let memory_total_mb = (info.mem_total.unwrap_or(0) / (1024 * 1024)) as u64;

        // ── CPU model from /proc/cpuinfo ──────────────────────────────────
        let cpu_model = match tokio::fs::read_to_string("/proc/cpuinfo").await {
            Ok(content) => parse_cpu_model(&content),
            Err(e) => {
                tracing::warn!(error = ?e, "Failed to read /proc/cpuinfo for CPU model");
                None
            }
        };

        // ── Available memory from /proc/meminfo ───────────────────────────
        let memory_available_mb = match tokio::fs::read_to_string("/proc/meminfo").await {
            Ok(content) => parse_mem_available_mb(&content).unwrap_or(memory_total_mb),
            Err(e) => {
                tracing::warn!(error = ?e, "Failed to read /proc/meminfo for available memory");
                memory_total_mb // fall back to total
            }
        };

        // ── Storage pool from Docker root dir ─────────────────────────────
        let storage_pools = {
            let docker_root = info.docker_root_dir.as_deref().unwrap_or("/var/lib/docker");
            let storage_type = info
                .driver
                .clone()
                .unwrap_or_else(|| "overlay2".to_string());

            match fs_stats(docker_root) {
                Some((total_bytes, avail_bytes)) => {
                    vec![StoragePoolInfo {
                        name: docker_root.to_string(),
                        total_gb: total_bytes / (1024 * 1024 * 1024),
                        available_gb: avail_bytes / (1024 * 1024 * 1024),
                        storage_type,
                    }]
                }
                None => {
                    tracing::warn!(
                        "Failed to get filesystem stats for Docker root '{}'; storage pool omitted",
                        docker_root
                    );
                    vec![]
                }
            }
        };

        Some(ResourceInventory {
            cpu_model,
            cpu_cores: cpu_threads, // Docker reports logical CPUs; no physical-core count available
            cpu_threads,
            cpu_mhz: None, // not exposed by Docker API; would need /proc/cpuinfo MHz field
            memory_total_mb,
            memory_available_mb,
            storage_pools,
            gpu_devices: vec![],
            templates: vec![],
        })
    }

    async fn verify_setup(&self) -> SetupVerification {
        let mut result = SetupVerification::default();

        match self.client.ping().await {
            Ok(_) => {
                result.api_reachable = Some(true);
            }
            Err(e) => {
                result.api_reachable = Some(false);
                result
                    .errors
                    .push(format!("Cannot reach Docker daemon: {:#}", e));
                return result;
            }
        }

        match self.client.list_images::<String>(None).await {
            Ok(images) => {
                result.storage_accessible = Some(true);

                let image_exists = images.iter().any(|img| {
                    img.repo_tags
                        .iter()
                        .any(|t| t == &self.config.default_image)
                });

                if image_exists {
                    result.template_exists = Some(true);
                } else {
                    result.template_exists = Some(false);
                    result.errors.push(format!(
                        "Default image '{}' not found locally. Pull it with: docker pull {}",
                        self.config.default_image, self.config.default_image
                    ));
                }
            }
            Err(e) => {
                result.storage_accessible = Some(false);
                result
                    .errors
                    .push(format!("Cannot list Docker images: {:#}", e));
            }
        }

        result
    }
}

#[cfg(test)]
impl DockerProvisioner {
    fn new_for_test(config: DockerConfig) -> Self {
        let client =
            Docker::connect_with_http("http://localhost:1", 120, bollard::API_DEFAULT_VERSION)
                .expect("connect_with_http should not fail");
        Self { config, client }
    }

    fn new_for_mockito(url: String) -> Self {
        Self::new_for_mockito_with_image(url, "ghcr.io/decent-stuff/dc-agent-ssh:latest".to_string())
    }

    fn new_for_mockito_with_image(url: String, default_image: String) -> Self {
        let client = Docker::connect_with_http(&url, 120, bollard::API_DEFAULT_VERSION)
            .expect("connect_with_http should not fail");
        Self {
            config: DockerConfig {
                socket_path: String::new(),
                network: "bridge".to_string(),
                default_image,
                ssh_port: 22,
            },
            client,
        }
    }
}

#[cfg(test)]
#[path = "docker_tests.rs"]
mod docker_tests;
