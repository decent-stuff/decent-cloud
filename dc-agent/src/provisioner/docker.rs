use super::{
    HealthStatus, Instance, ProvisionRequest, Provisioner, RunningInstance, SetupVerification,
};
use crate::config::DockerConfig;
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use bollard::models::{HostConfig, PortBinding};
use bollard::container::{
    Config, CreateContainerOptions, InspectContainerOptions, ListContainersOptions,
    RemoveContainerOptions, StartContainerOptions, StopContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::service::ContainerInspectResponse;
use bollard::Docker;
use futures::StreamExt;
use std::collections::HashMap;
use tracing;

fn container_name(contract_id: &str) -> String {
    format!("dc-{}", contract_id)
}

fn extract_contract_id(name: &str) -> Option<String> {
    name.strip_prefix("dc-").map(String::from)
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
        let images = self.client.list_images::<String>(None).await;
        let already_present = images
            .unwrap_or_default()
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

    fn build_container_config(
        &self,
        request: &ProvisionRequest,
        image: &str,
    ) -> Config<String> {
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
            labels: Some(labels),
            host_config: Some(host_config),
            ..Default::default()
        }
    }

    fn container_to_instance(
        &self,
        inspect: &ContainerInspectResponse,
        id: &str,
    ) -> Option<Instance> {
        let name = inspect
            .name
            .as_ref()
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_default();

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
            ipv6_address: None,
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
        Ok(containers
            .into_iter()
            .next()
            .and_then(|c| c.id))
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
                if e.to_string().contains("No such container") {
                    tracing::warn!(id = %external_id, "Container not found, assuming already removed");
                    return Ok(());
                }
                return Err(e).with_context(|| format!("Failed to inspect container {}", external_id));
            }
        };

        let running = inspect
            .state
            .as_ref()
            .and_then(|s| s.running)
            .unwrap_or(false);

        if running {
            self.client
                .stop_container(
                    external_id,
                    Some(StopContainerOptions {
                        t: 30,
                    }),
                )
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
                if e.to_string().contains("No such container") {
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
                .and_then(|ts| {
                    chrono::DateTime::parse_from_rfc3339(ts).ok()
                });

            let uptime_seconds = started_at.map(|t| {
                t.signed_duration_since(chrono::Utc::now())
                    .num_seconds()
                    .unsigned_abs()
            }).unwrap_or(0);

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
                if e.to_string().contains("No such container") {
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
            Ok(_) => {
                result.storage_accessible = Some(true);
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
        let client = Docker::connect_with_http(
            "http://localhost:1",
            120,
            bollard::API_DEFAULT_VERSION,
        )
        .expect("connect_with_http should not fail");
        Self { config, client }
    }
}

#[cfg(test)]
#[path = "docker_tests.rs"]
mod docker_tests;
