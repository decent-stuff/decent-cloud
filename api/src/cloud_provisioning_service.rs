//! Background service for provisioning and terminating cloud resources.
//!
//! Polls for pending cloud resources and provisions/deletes them via the appropriate backend.
//! For contract-linked resources, also executes post-provision scripts and updates contract status.

use crate::cloud::{
    hetzner::HetznerBackend, proxmox_api::ProxmoxApiBackend, types::BackendType,
    CloudBackend, CreateServerRequest,
};
use crate::crypto::{decrypt_server_credential, ServerEncryptionKey};
use crate::database::cloud_resources::PendingProvisioningResource;
use crate::database::Database;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

pub struct CloudProvisioningService {
    database: Arc<Database>,
    poll_interval: Duration,
    termination_poll_interval: Duration,
    lock_holder: String,
}

impl CloudProvisioningService {
    pub fn new(database: Arc<Database>, poll_interval_secs: u64, termination_poll_interval_secs: u64) -> Self {
        Self {
            database,
            poll_interval: Duration::from_secs(poll_interval_secs),
            termination_poll_interval: Duration::from_secs(termination_poll_interval_secs),
            lock_holder: format!("api-server-{}", Uuid::new_v4()),
        }
    }

    pub async fn run(self) {
        let enabled = std::env::var("CLOUD_PROVISIONING_ENABLED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(false);

        if !enabled {
            tracing::info!(
                "CLOUD_PROVISIONING_ENABLED not set - cloud provisioning service disabled. Set CLOUD_PROVISIONING_ENABLED=true to enable."
            );
            return;
        }

        let encryption_key = match ServerEncryptionKey::from_env() {
            Ok(key) => key,
            Err(e) => {
                tracing::error!(
                    "CLOUD_PROVISIONING_ENABLED=true but CREDENTIAL_ENCRYPTION_KEY is invalid: {}. The server cannot start without a valid encryption key.",
                    e
                );
                panic!(
                    "CLOUD_PROVISIONING_ENABLED=true but CREDENTIAL_ENCRYPTION_KEY is invalid: {}. \
                     Set CREDENTIAL_ENCRYPTION_KEY to a valid 64-character hex string (32 bytes) to enable cloud provisioning.",
                    e
                );
            }
        };

        tracing::info!(
            "Starting cloud provisioning service (provision interval: {}s, termination interval: {}s)",
            self.poll_interval.as_secs(),
            self.termination_poll_interval.as_secs()
        );

        let db = self.database.clone();
        let lock_holder = self.lock_holder.clone();
        let key = encryption_key.clone();

        let prov_interval = self.poll_interval;
        let provision_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(prov_interval);
            loop {
                interval.tick().await;
                if let Err(e) = provision_pending_resources(&db, &lock_holder, &key).await {
                    tracing::error!("Cloud provisioning failed: {:#}", e);
                }
            }
        });

        let db = self.database.clone();
        let lock_holder = self.lock_holder.clone();
        let key = encryption_key.clone();

        let term_interval = self.termination_poll_interval;
        let termination_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(term_interval);
            loop {
                interval.tick().await;
                if let Err(e) = terminate_pending_resources(&db, &lock_holder, &key).await {
                    tracing::error!("Cloud termination failed: {:#}", e);
                }
            }
        });

        let _ = tokio::try_join!(provision_task, termination_task);
    }
}

async fn provision_pending_resources(
    database: &Database,
    lock_holder: &str,
    encryption_key: &ServerEncryptionKey,
) -> anyhow::Result<()> {
    let pending = database.get_pending_provisioning_resources(5).await?;

    if pending.is_empty() {
        return Ok(());
    }

    tracing::info!("Found {} pending cloud resources to provision", pending.len());

    for resource in pending {
        if !database.acquire_cloud_resource_lock(&resource.id, lock_holder).await? {
            tracing::debug!("Could not acquire lock for resource {}, skipping", resource.id);
            continue;
        }

        let resource_id = resource.id;
        let result = provision_one(database, resource, encryption_key).await;

        if let Err(e) = database.release_cloud_resource_lock(&resource_id).await {
            tracing::error!("Failed to release lock for resource {}: {}", resource_id, e);
        }

        if let Err(e) = result {
            tracing::error!("Failed to provision resource {}: {:#}", resource_id, e);
            if let Err(e) = database.update_cloud_resource_status(&resource_id, "failed").await {
                tracing::error!("Failed to mark resource {} as failed: {}", resource_id, e);
            }
        }
    }

    Ok(())
}

async fn provision_one(
    database: &Database,
    resource: PendingProvisioningResource,
    encryption_key: &ServerEncryptionKey,
) -> anyhow::Result<()> {
    let resource_id = resource.id;
    tracing::info!("Provisioning resource {} ({})", resource_id, resource.name);

    let credentials = decrypt_server_credential(&resource.credentials_encrypted, encryption_key)?;
    let backend_type: BackendType = resource.backend_type.parse()?;
    let backend = create_backend(backend_type, &credentials).await?;

    let request = CreateServerRequest {
        name: resource.name.clone(),
        server_type: resource.server_type.clone(),
        location: resource.location.clone(),
        image: resource.image.clone(),
        ssh_pubkey: resource.ssh_pubkey.clone(),
    };

    let result = backend.create_server(request).await?;

    let public_ip = result.server.public_ip.ok_or_else(|| anyhow::anyhow!("Server has no public IP"))?;
    let ssh_key_id = result.ssh_key_id.unwrap_or_default();

    let gateway_slug = generate_gateway_slug();
    let gateway_ssh_port = 20000;
    let gateway_port_range_start = 20001;
    let gateway_port_range_end = 20010;

    // Execute post-provision script if present (recipe provisioning)
    if let Some(script) = &resource.post_provision_script {
        let context_id = resource.contract_id
            .as_ref()
            .map(hex::encode)
            .unwrap_or_else(|| resource_id.to_string());

        if let Err(e) = dcc_common::ssh_exec::execute_post_provision_script(
            &public_ip,
            22,
            script,
            &context_id,
        ).await {
            tracing::error!(
                resource_id = %resource_id,
                "Post-provision script failed, cleaning up VM: {:#}",
                e
            );
            // Cleanup: delete VM + SSH key on script failure
            cleanup_failed_provision(&*backend, &result.server.id, &ssh_key_id).await;
            return Err(e.context("Post-provision script failed"));
        }
    }

    database.update_cloud_resource_provisioned(
        &resource_id,
        &public_ip,
        &ssh_key_id,
        &gateway_slug,
        gateway_ssh_port,
        gateway_port_range_start,
        gateway_port_range_end,
    ).await?;

    // If linked to a contract, update contract status to active
    if let Some(contract_id) = &resource.contract_id {
        let instance_details = serde_json::json!({
            "public_ip": public_ip,
            "ssh_port": 22,
            "gateway_slug": gateway_slug,
            "gateway_ssh_port": gateway_ssh_port,
            "gateway_port_range_start": gateway_port_range_start,
            "gateway_port_range_end": gateway_port_range_end,
        })
        .to_string();

        if let Err(e) = database.update_contract_provisioned_by_cloud_resource(
            contract_id,
            &instance_details,
        ).await {
            tracing::error!(
                contract_id = %hex::encode(contract_id),
                "Failed to update contract status after provisioning: {:#}",
                e
            );
        }
    }

    tracing::info!(
        "Successfully provisioned resource {} with IP {} (gateway: {}, ssh_key_id: {})",
        resource_id,
        public_ip,
        gateway_slug,
        ssh_key_id
    );

    Ok(())
}

/// Cleanup a failed provisioning attempt by deleting the VM and SSH key.
async fn cleanup_failed_provision(backend: &dyn CloudBackend, server_id: &str, ssh_key_id: &str) {
    if let Err(e) = backend.delete_server(server_id).await {
        tracing::error!("Failed to cleanup VM {} after script failure: {:#}", server_id, e);
    }
    if !ssh_key_id.is_empty() {
        if let Err(e) = backend.delete_ssh_key(ssh_key_id).await {
            tracing::error!("Failed to cleanup SSH key {} after script failure: {:#}", ssh_key_id, e);
        }
    }
}

async fn terminate_pending_resources(
    database: &Database,
    lock_holder: &str,
    encryption_key: &ServerEncryptionKey,
) -> anyhow::Result<()> {
    let pending = database.get_pending_termination_resources(5).await?;

    if pending.is_empty() {
        return Ok(());
    }

    tracing::info!("Found {} pending cloud resources to terminate", pending.len());

    for (resource_id, external_id, ssh_key_id, backend_type, credentials_encrypted) in pending {
        if !database.acquire_cloud_resource_lock(&resource_id, lock_holder).await? {
            tracing::debug!("Could not acquire lock for resource {}, skipping", resource_id);
            continue;
        }

        let result = terminate_one(
            database,
            resource_id,
            &external_id,
            ssh_key_id.as_deref(),
            &backend_type,
            &credentials_encrypted,
            encryption_key,
        ).await;

        if let Err(e) = database.release_cloud_resource_lock(&resource_id).await {
            tracing::error!("Failed to release lock for resource {}: {}", resource_id, e);
        }

        if let Err(e) = result {
            tracing::error!("Failed to terminate resource {}: {:#}", resource_id, e);
        }
    }

    Ok(())
}

async fn terminate_one(
    database: &Database,
    resource_id: Uuid,
    external_id: &str,
    ssh_key_id: Option<&str>,
    backend_type_str: &str,
    credentials_encrypted: &str,
    encryption_key: &ServerEncryptionKey,
) -> anyhow::Result<()> {
    tracing::info!("Terminating resource {} (external: {})", resource_id, external_id);

    if external_id.starts_with("pending-") {
        tracing::info!("Resource {} was never provisioned, marking as terminated", resource_id);
        database.mark_cloud_resource_terminated(&resource_id).await?;
        return Ok(());
    }

    let credentials = decrypt_server_credential(credentials_encrypted, encryption_key)?;
    let backend_type: BackendType = backend_type_str.parse()?;
    let backend = create_backend(backend_type, &credentials).await?;

    match backend.delete_server(external_id).await {
        Ok(()) => {
            tracing::info!("Successfully deleted server {} for resource {}", external_id, resource_id);
        }
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("not found") || err_str.contains("404") {
                tracing::info!("Server {} already deleted, marking resource {} as terminated", external_id, resource_id);
            } else {
                return Err(e);
            }
        }
    }

    if let Some(key_id) = ssh_key_id {
        if !key_id.is_empty() {
            if let Err(e) = backend.delete_ssh_key(key_id).await {
                tracing::warn!("Failed to delete SSH key {} for resource {}: {}", key_id, resource_id, e);
            } else {
                tracing::info!("Successfully deleted SSH key {} for resource {}", key_id, resource_id);
            }
        }
    }

    database.mark_cloud_resource_terminated(&resource_id).await?;

    tracing::info!("Successfully terminated resource {}", resource_id);

    Ok(())
}

async fn create_backend(backend_type: BackendType, credentials: &str) -> anyhow::Result<Box<dyn CloudBackend>> {
    match backend_type {
        BackendType::Hetzner => {
            let backend = HetznerBackend::new(credentials.to_string())?;
            Ok(Box::new(backend))
        }
        BackendType::ProxmoxApi => {
            let config = serde_json::from_str(credentials)?;
            let backend = ProxmoxApiBackend::new(config)?;
            Ok(Box::new(backend))
        }
    }
}

fn generate_gateway_slug() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();
    (0..6)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}
