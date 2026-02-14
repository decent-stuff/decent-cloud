//! Common types for cloud backends

use poem_openapi::{Enum, Object};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, Enum)]
#[ts(export)]
#[oai(rename_all = "camelCase")]
pub enum BackendType {
    Hetzner,
    ProxmoxApi,
}

impl std::fmt::Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendType::Hetzner => write!(f, "hetzner"),
            BackendType::ProxmoxApi => write!(f, "proxmox_api"),
        }
    }
}

impl std::str::FromStr for BackendType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "hetzner" => Ok(BackendType::Hetzner),
            "proxmox_api" => Ok(BackendType::ProxmoxApi),
            _ => anyhow::bail!("Unknown backend type: {}", s),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Object)]
#[ts(export)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ServerType {
    pub id: String,
    pub name: String,
    pub cores: u32,
    pub memory_gb: f64,
    pub disk_gb: u32,
    pub price_monthly: Option<f64>,
    pub price_hourly: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Object)]
#[ts(export)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub id: String,
    pub name: String,
    pub city: String,
    pub country: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Object)]
#[ts(export)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct Image {
    pub id: String,
    pub name: String,
    pub os_type: String,
    pub os_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, Enum)]
#[ts(export)]
#[oai(rename_all = "camelCase")]
pub enum ServerStatus {
    Provisioning,
    Running,
    Stopped,
    Deleting,
    Failed,
}

impl std::fmt::Display for ServerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerStatus::Provisioning => write!(f, "provisioning"),
            ServerStatus::Running => write!(f, "running"),
            ServerStatus::Stopped => write!(f, "stopped"),
            ServerStatus::Deleting => write!(f, "deleting"),
            ServerStatus::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for ServerStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "provisioning" => Ok(ServerStatus::Provisioning),
            "running" => Ok(ServerStatus::Running),
            "stopped" => Ok(ServerStatus::Stopped),
            "deleting" => Ok(ServerStatus::Deleting),
            "failed" => Ok(ServerStatus::Failed),
            _ => anyhow::bail!("Unknown server status: {}", s),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Object)]
#[ts(export)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub id: String,
    pub name: String,
    pub status: ServerStatus,
    pub public_ip: Option<String>,
    pub server_type: String,
    pub location: String,
    pub image: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Object)]
#[ts(export)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct CreateServerRequest {
    pub name: String,
    pub server_type: String,
    pub location: String,
    pub image: String,
    pub ssh_pubkey: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Object)]
#[ts(export)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct ServerMetrics {
    pub cpu_percent: Option<f64>,
    pub memory_percent: Option<f64>,
    pub disk_percent: Option<f64>,
    pub network_in_bytes: Option<u64>,
    pub network_out_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Object)]
#[ts(export)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct BackendCatalog {
    pub server_types: Vec<ServerType>,
    pub locations: Vec<Location>,
    pub images: Vec<Image>,
}
