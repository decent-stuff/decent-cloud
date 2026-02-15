//! Cloud backend abstraction for self-provisioning

use async_trait::async_trait;

use crate::cloud::types::{BackendCatalog, Server, ServerMetrics};

pub mod hetzner;
pub mod proxmox_api;
pub mod types;

pub use types::CreateServerRequest;

#[derive(Debug, Clone)]
pub struct ProvisionResult {
    pub server: Server,
    pub ssh_key_id: Option<String>,
}

#[async_trait]
pub trait CloudBackend: Send + Sync {
    fn backend_type(&self) -> types::BackendType;

    async fn validate_credentials(&self) -> anyhow::Result<()>;

    async fn get_catalog(&self) -> anyhow::Result<BackendCatalog>;

    async fn list_server_types(&self) -> anyhow::Result<Vec<types::ServerType>>;

    async fn list_locations(&self) -> anyhow::Result<Vec<types::Location>>;

    async fn list_images(&self) -> anyhow::Result<Vec<types::Image>>;

    async fn create_server(&self, req: CreateServerRequest) -> anyhow::Result<ProvisionResult>;

    async fn get_server(&self, id: &str) -> anyhow::Result<Server>;

    async fn start_server(&self, id: &str) -> anyhow::Result<()>;

    async fn stop_server(&self, id: &str) -> anyhow::Result<()>;

    async fn delete_server(&self, id: &str) -> anyhow::Result<()>;

    async fn delete_ssh_key(&self, key_id: &str) -> anyhow::Result<()>;

    async fn get_server_metrics(&self, id: &str) -> anyhow::Result<ServerMetrics>;
}
