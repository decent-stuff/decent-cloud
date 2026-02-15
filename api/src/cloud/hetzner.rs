//! Hetzner Cloud API client
//!
//! Implements the CloudBackend trait for Hetzner Cloud.
//!
//! API docs: https://docs.hetzner.cloud/

use anyhow::Context;
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

use crate::cloud::types::{
    BackendCatalog, CreateServerRequest, Image, Location, Server, ServerMetrics, ServerStatus,
    ServerType,
};
use super::{CloudBackend, ProvisionResult};

const HETZNER_API_BASE: &str = "https://api.hetzner.cloud/v1";
const REQUEST_TIMEOUT_SECS: u64 = 30;

pub struct HetznerBackend {
    client: Client,
    token: String,
}

impl HetznerBackend {
    pub fn new(token: String) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client, token })
    }

    fn request_builder(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", HETZNER_API_BASE, path);
        self.client
            .request(method, &url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
    }

    async fn handle_error(&self, response: reqwest::Response) -> anyhow::Error {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        match status {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                anyhow::anyhow!("Hetzner API authentication failed ({}): {}", status, body)
            }
            StatusCode::PAYMENT_REQUIRED => {
                anyhow::anyhow!("Hetzner account has insufficient balance: {}", body)
            }
            StatusCode::UNPROCESSABLE_ENTITY => {
                anyhow::anyhow!("Invalid request parameters: {}", body)
            }
            StatusCode::TOO_MANY_REQUESTS => {
                anyhow::anyhow!("Rate limited by Hetzner API: {}", body)
            }
            _ => anyhow::anyhow!("Hetzner API error ({}): {}", status, body),
        }
    }
}

#[derive(Debug, Serialize)]
struct CreateSshKeyRequest {
    name: String,
    public_key: String,
}

#[derive(Debug, Deserialize)]
struct SshKeyResponse {
    ssh_key: HetznerSshKey,
}

#[derive(Debug, Deserialize)]
struct HetznerSshKey {
    #[allow(dead_code)]
    id: i64,
    #[allow(dead_code)]
    name: String,
}

#[derive(Debug, Serialize)]
struct CreateServerRequestHetzner {
    name: String,
    server_type: String,
    location: String,
    image: String,
    ssh_keys: Vec<i64>,
    start_after_create: bool,
}

#[derive(Debug, Deserialize)]
struct ServerResponse {
    server: HetznerServer,
}

#[derive(Debug, Deserialize)]
struct ServerTypesResponse {
    server_types: Vec<HetznerServerType>,
}

#[derive(Debug, Deserialize)]
struct LocationsResponse {
    locations: Vec<HetznerLocation>,
}

#[derive(Debug, Deserialize)]
struct ImagesResponse {
    images: Vec<HetznerImage>,
}

#[derive(Debug, Deserialize)]
struct HetznerServer {
    id: i64,
    name: String,
    status: String,
    public_net: HetznerPublicNet,
    server_type: HetznerServerTypeRef,
    datacenter: HetznerDatacenter,
    image: Option<HetznerImageRef>,
    created: String,
}

#[derive(Debug, Deserialize)]
struct HetznerPublicNet {
    ipv4: HetznerIpv4,
}

#[derive(Debug, Deserialize)]
struct HetznerIpv4 {
    ip: String,
}

#[derive(Debug, Deserialize)]
struct HetznerServerTypeRef {
    name: String,
}

#[derive(Debug, Deserialize)]
struct HetznerDatacenter {
    #[allow(dead_code)]
    name: String,
    location: HetznerLocationRef,
}

#[derive(Debug, Deserialize)]
struct HetznerLocationRef {
    name: String,
}

#[derive(Debug, Deserialize)]
struct HetznerImageRef {
    name: String,
}

#[derive(Debug, Deserialize)]
struct HetznerServerType {
    id: i64,
    name: String,
    cores: i32,
    memory: f64,
    disk: i32,
    prices: Vec<HetznerPrice>,
}

#[derive(Debug, Deserialize)]
struct HetznerPrice {
    #[allow(dead_code)]
    location: String,
    price_monthly: HetznerPriceDetail,
    price_hourly: HetznerPriceDetail,
}

#[derive(Debug, Deserialize)]
struct HetznerPriceDetail {
    gross: String,
}

#[derive(Debug, Deserialize)]
struct HetznerLocation {
    #[allow(dead_code)]
    id: i64,
    name: String,
    city: String,
    country: String,
}

#[derive(Debug, Deserialize)]
struct HetznerImage {
    id: i64,
    name: Option<String>,
    os_flavor: String,
    status: String,
    #[serde(rename = "type")]
    type_: Option<String>,
    description: Option<String>,
}

impl HetznerBackend {
    fn convert_server(&self, s: HetznerServer) -> Server {
        let status = match s.status.as_str() {
            "initializing" | "starting" | "rebuilding" | "migrating" => {
                ServerStatus::Provisioning
            }
            "running" => ServerStatus::Running,
            "off" | "stopping" => ServerStatus::Stopped,
            "deleting" => ServerStatus::Deleting,
            other => {
                tracing::warn!("Unknown Hetzner server status '{}', treating as failed", other);
                ServerStatus::Failed
            }
        };

        let created_at = chrono::DateTime::parse_from_rfc3339(&s.created)
            .map(|dt| dt.with_timezone(&chrono::Utc).to_rfc3339())
            .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339());

        Server {
            id: s.id.to_string(),
            name: s.name,
            status,
            public_ip: Some(s.public_net.ipv4.ip),
            server_type: s.server_type.name,
            location: s.datacenter.location.name,
            image: s.image.map(|i| i.name).unwrap_or_default(),
            created_at,
        }
    }

    fn convert_server_type(&self, st: HetznerServerType) -> ServerType {
        let (price_monthly, price_hourly) = st
            .prices
            .first()
            .map(|p| {
                (
                    p.price_monthly.gross.parse::<f64>().ok(),
                    p.price_hourly.gross.parse::<f64>().ok(),
                )
            })
            .unwrap_or((None, None));

        ServerType {
            id: st.id.to_string(),
            name: st.name,
            cores: st.cores as u32,
            memory_gb: st.memory,
            disk_gb: st.disk as u32,
            price_monthly,
            price_hourly,
        }
    }

    fn convert_location(&self, loc: HetznerLocation) -> Location {
        Location {
            id: loc.name.clone(),
            name: loc.name,
            city: loc.city,
            country: loc.country,
        }
    }

    fn convert_image(&self, img: HetznerImage) -> Option<Image> {
        if img.status != "available" {
            return None;
        }
        if img.type_.as_deref() == Some("backup") {
            return None;
        }

        Some(Image {
            id: img.id.to_string(),
            name: img.name.clone().unwrap_or_else(|| img.id.to_string()),
            os_type: img.os_flavor,
            os_version: img.description.and_then(|d| {
                d.split_whitespace()
                    .nth(1)
                    .map(|s| s.trim_end_matches('.').to_string())
            }),
        })
    }

    async fn wait_for_ssh_reachable(&self, ip: &str, timeout_secs: u64) -> anyhow::Result<bool> {
        let addr = format!("{}:22", ip);
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);

        while start.elapsed() < timeout {
            match TcpStream::connect(&addr).await {
                Ok(mut stream) => {
                    let mut banner = [0u8; 256];
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        stream.read(&mut banner)
                    ).await {
                        Ok(Ok(n)) if n > 0 => {
                            let banner_str = String::from_utf8_lossy(&banner[..n]);
                            if banner_str.contains("SSH") {
                                tracing::info!("SSH reachable at {} after {:?}", addr, start.elapsed());
                                return Ok(true);
                            }
                        }
                        _ => {}
                    }
                }
                Err(_) => {}
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }

        tracing::warn!("SSH not reachable at {} after {}s", addr, timeout_secs);
        Ok(false)
    }
}

#[async_trait]
impl CloudBackend for HetznerBackend {
    fn backend_type(&self) -> super::types::BackendType {
        super::types::BackendType::Hetzner
    }

    async fn validate_credentials(&self) -> anyhow::Result<()> {
        let response = self
            .request_builder(reqwest::Method::GET, "/server_types")
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
            .request_builder(reqwest::Method::GET, "/server_types")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        let data: ServerTypesResponse = response.json().await?;
        Ok(data
            .server_types
            .into_iter()
            .map(|st| self.convert_server_type(st))
            .collect())
    }

    async fn list_locations(&self) -> anyhow::Result<Vec<Location>> {
        let response = self
            .request_builder(reqwest::Method::GET, "/locations")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        let data: LocationsResponse = response.json().await?;
        Ok(data
            .locations
            .into_iter()
            .map(|loc| self.convert_location(loc))
            .collect())
    }

    async fn list_images(&self) -> anyhow::Result<Vec<Image>> {
        let response = self
            .request_builder(reqwest::Method::GET, "/images?type=system")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        let data: ImagesResponse = response.json().await?;
        Ok(data
            .images
            .into_iter()
            .filter_map(|img| self.convert_image(img))
            .collect())
    }

    async fn create_server(&self, req: CreateServerRequest) -> anyhow::Result<ProvisionResult> {
        let ssh_key_response = self
            .request_builder(reqwest::Method::POST, "/ssh_keys")
            .json(&CreateSshKeyRequest {
                name: format!("dc-{}", &req.name),
                public_key: req.ssh_pubkey.clone(),
            })
            .send()
            .await?;

        if !ssh_key_response.status().is_success() {
            return Err(self.handle_error(ssh_key_response).await);
        }

        let ssh_key_data: SshKeyResponse = ssh_key_response.json().await?;
        let ssh_key_id = ssh_key_data.ssh_key.id;

        let server_req = CreateServerRequestHetzner {
            name: req.name.clone(),
            server_type: req.server_type.clone(),
            location: req.location.clone(),
            image: req.image.clone(),
            ssh_keys: vec![ssh_key_id],
            start_after_create: true,
        };

        let server_response = self
            .request_builder(reqwest::Method::POST, "/servers")
            .json(&server_req)
            .send()
            .await?;

        if !server_response.status().is_success() {
            cleanup_ssh_key(self, &ssh_key_id.to_string()).await;
            return Err(self.handle_error(server_response).await);
        }

        let server_data: ServerResponse = server_response.json().await?;
        let mut server = self.convert_server(server_data.server);

        let mut retries = 0;
        while server.status == ServerStatus::Provisioning && retries < 60 {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            server = self.get_server(&server.id).await?;
            retries += 1;
        }

        if server.status != ServerStatus::Running {
            cleanup_server_and_key(self, &server.id, &ssh_key_id.to_string()).await;
            anyhow::bail!("Server failed to reach running state: {:?}", server.status);
        }

        if let Some(ref ip) = server.public_ip {
            if !self.wait_for_ssh_reachable(ip, 120).await? {
                cleanup_server_and_key(self, &server.id, &ssh_key_id.to_string()).await;
                anyhow::bail!("SSH port not reachable after 120s");
            }
        }

        Ok(ProvisionResult {
            server,
            ssh_key_id: Some(ssh_key_id.to_string()),
        })
    }

    async fn get_server(&self, id: &str) -> anyhow::Result<Server> {
        let response = self
            .request_builder(reqwest::Method::GET, &format!("/servers/{}", id))
            .send()
            .await?;

        if response.status() == StatusCode::NOT_FOUND {
            anyhow::bail!("Server not found: {}", id);
        }

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        let data: ServerResponse = response.json().await?;
        Ok(self.convert_server(data.server))
    }

    async fn start_server(&self, id: &str) -> anyhow::Result<()> {
        let response = self
            .request_builder(
                reqwest::Method::POST,
                &format!("/servers/{}/actions/poweron", id),
            )
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        Ok(())
    }

    async fn stop_server(&self, id: &str) -> anyhow::Result<()> {
        let response = self
            .request_builder(
                reqwest::Method::POST,
                &format!("/servers/{}/actions/shutdown", id),
            )
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        Ok(())
    }

    async fn delete_server(&self, id: &str) -> anyhow::Result<()> {
        let response = self
            .request_builder(reqwest::Method::DELETE, &format!("/servers/{}", id))
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
        let id: i64 = key_id.parse().context("Invalid SSH key ID")?;
        let response = self
            .request_builder(reqwest::Method::DELETE, &format!("/ssh_keys/{}", id))
            .send()
            .await?;

        if !response.status().is_success() && response.status() != StatusCode::NOT_FOUND {
            tracing::warn!("Failed to delete Hetzner SSH key {}: {:?}", id, response.status());
        }

        Ok(())
    }
}

/// Log-and-continue cleanup for failed provisioning. Best-effort — errors are logged, not propagated.
async fn cleanup_server_and_key(backend: &HetznerBackend, server_id: &str, ssh_key_id: &str) {
    if let Err(e) = backend.delete_server(server_id).await {
        tracing::warn!("Cleanup: failed to delete server {}: {:#}", server_id, e);
    }
    cleanup_ssh_key(backend, ssh_key_id).await;
}

async fn cleanup_ssh_key(backend: &HetznerBackend, ssh_key_id: &str) {
    if let Err(e) = backend.delete_ssh_key(ssh_key_id).await {
        tracing::warn!("Cleanup: failed to delete SSH key {}: {:#}", ssh_key_id, e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_server(status: &str) -> HetznerServer {
        HetznerServer {
            id: 123,
            name: "test-server".to_string(),
            status: status.to_string(),
            public_net: HetznerPublicNet {
                ipv4: HetznerIpv4 {
                    ip: "1.2.3.4".to_string(),
                },
            },
            server_type: HetznerServerTypeRef {
                name: "cx23".to_string(),
            },
            datacenter: HetznerDatacenter {
                name: "fsn1-dc14".to_string(),
                location: HetznerLocationRef {
                    name: "fsn1".to_string(),
                },
            },
            image: Some(HetznerImageRef {
                name: "ubuntu-24.04".to_string(),
            }),
            created: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_hetzner_status_conversion_all_states() {
        let backend = HetznerBackend::new("test_token".to_string()).unwrap();

        // Provisioning states (server not yet usable)
        for status in ["initializing", "starting", "rebuilding", "migrating"] {
            let converted = backend.convert_server(make_test_server(status));
            assert_eq!(converted.status, ServerStatus::Provisioning, "status '{status}'");
        }

        // Running
        let converted = backend.convert_server(make_test_server("running"));
        assert_eq!(converted.status, ServerStatus::Running);
        assert_eq!(converted.public_ip, Some("1.2.3.4".to_string()));

        // Stopped states
        for status in ["off", "stopping"] {
            let converted = backend.convert_server(make_test_server(status));
            assert_eq!(converted.status, ServerStatus::Stopped, "status '{status}'");
        }

        // Deleting
        let converted = backend.convert_server(make_test_server("deleting"));
        assert_eq!(converted.status, ServerStatus::Deleting);

        // Unknown falls to Failed
        let converted = backend.convert_server(make_test_server("exploded"));
        assert_eq!(converted.status, ServerStatus::Failed);
    }

    #[test]
    fn test_hetzner_price_deserialization() {
        // Hetzner API returns prices as nested objects with string values, not plain floats
        let json = r#"{
            "location": "fsn1",
            "price_monthly": {"net": "3.2900000000", "gross": "3.9151000000"},
            "price_hourly": {"net": "0.0050000000", "gross": "0.0059500000"}
        }"#;
        let price: HetznerPrice = serde_json::from_str(json).unwrap();
        assert_eq!(price.location, "fsn1");
        assert_eq!(price.price_monthly.gross, "3.9151000000");
        assert_eq!(price.price_hourly.gross, "0.0059500000");
    }

    #[test]
    fn test_hetzner_server_type_price_conversion() {
        let backend = HetznerBackend::new("test_token".to_string()).unwrap();
        let st = HetznerServerType {
            id: 1,
            name: "cx22".to_string(),
            cores: 2,
            memory: 4.0,
            disk: 40,
            prices: vec![HetznerPrice {
                location: "fsn1".to_string(),
                price_monthly: HetznerPriceDetail {
                    gross: "3.92".to_string(),
                },
                price_hourly: HetznerPriceDetail {
                    gross: "0.006".to_string(),
                },
            }],
        };
        let converted = backend.convert_server_type(st);
        assert_eq!(converted.price_monthly, Some(3.92));
        assert_eq!(converted.price_hourly, Some(0.006));
    }

    #[test]
    fn test_hetzner_server_type_no_prices() {
        let backend = HetznerBackend::new("test_token".to_string()).unwrap();
        let st = HetznerServerType {
            id: 1,
            name: "cx22".to_string(),
            cores: 2,
            memory: 4.0,
            disk: 40,
            prices: vec![],
        };
        let converted = backend.convert_server_type(st);
        assert_eq!(converted.price_monthly, None);
        assert_eq!(converted.price_hourly, None);
    }

    #[test]
    fn test_hetzner_image_type_field_deserialization() {
        // Hetzner API returns "type" (a Rust keyword) — we use #[serde(rename = "type")]
        let json = r#"{
            "id": 67794396,
            "name": "ubuntu-22.04",
            "os_flavor": "ubuntu",
            "status": "available",
            "type": "system",
            "description": "Ubuntu 22.04 LTS"
        }"#;
        let img: HetznerImage = serde_json::from_str(json).unwrap();
        assert_eq!(img.id, 67794396);
        assert_eq!(img.type_, Some("system".to_string()));
        assert_eq!(img.os_flavor, "ubuntu");
    }

    #[test]
    fn test_hetzner_image_filters_non_available() {
        let backend = HetznerBackend::new("test_token".to_string()).unwrap();
        let img = HetznerImage {
            id: 1,
            name: Some("old-image".to_string()),
            os_flavor: "ubuntu".to_string(),
            status: "deprecated".to_string(),
            type_: Some("system".to_string()),
            description: None,
        };
        assert!(backend.convert_image(img).is_none());
    }

    #[test]
    fn test_hetzner_image_filters_backups() {
        let backend = HetznerBackend::new("test_token".to_string()).unwrap();
        let img = HetznerImage {
            id: 1,
            name: Some("my-backup".to_string()),
            os_flavor: "ubuntu".to_string(),
            status: "available".to_string(),
            type_: Some("backup".to_string()),
            description: None,
        };
        assert!(backend.convert_image(img).is_none());
    }

    #[test]
    fn test_hetzner_server_types_response_deserialization() {
        let json = r#"{
            "server_types": [{
                "id": 22,
                "name": "cpx11",
                "cores": 2,
                "memory": 2.0,
                "disk": 40,
                "prices": [{
                    "location": "fsn1",
                    "price_monthly": {"net": "4.0756", "gross": "4.8499"},
                    "price_hourly": {"net": "0.0073", "gross": "0.0087"}
                }]
            }]
        }"#;
        let resp: ServerTypesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.server_types.len(), 1);
        assert_eq!(resp.server_types[0].name, "cpx11");
    }
}
