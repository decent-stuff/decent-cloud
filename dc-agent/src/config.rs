use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub api: ApiConfig,
    pub polling: PollingConfig,
    pub provisioner: ProvisionerConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiConfig {
    pub endpoint: String,
    pub provider_pubkey: String,
    pub provider_secret_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PollingConfig {
    pub interval_seconds: u64,
    pub health_check_interval_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ProvisionerConfig {
    Proxmox { proxmox: ProxmoxConfig },
    Script { script: ScriptConfig },
    Manual { manual: ManualConfig },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProxmoxConfig {
    pub api_url: String,
    pub api_token_id: String,
    pub api_token_secret: String,
    pub node: String,
    pub template_vmid: u32,
    pub storage: String,
    pub pool: Option<String>,
    #[serde(default = "default_verify_ssl")]
    pub verify_ssl: bool,
}

/// Script provisioner configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptConfig {
    pub provision: String,
    pub terminate: String,
    pub health_check: String,
    #[serde(default = "default_script_timeout")]
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ManualConfig {
    pub notification_webhook: Option<String>,
}

fn default_verify_ssl() -> bool {
    true
}

fn default_script_timeout() -> u64 {
    300
}

impl Config {
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        config.validate()?;

        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.api.endpoint.is_empty() {
            anyhow::bail!("api.endpoint cannot be empty");
        }
        if self.api.provider_pubkey.is_empty() {
            anyhow::bail!("api.provider_pubkey cannot be empty");
        }
        if self.api.provider_secret_key.is_empty() {
            anyhow::bail!("api.provider_secret_key cannot be empty");
        }
        if self.polling.interval_seconds == 0 {
            anyhow::bail!("polling.interval_seconds must be greater than 0");
        }
        if self.polling.health_check_interval_seconds == 0 {
            anyhow::bail!("polling.health_check_interval_seconds must be greater than 0");
        }

        match &self.provisioner {
            ProvisionerConfig::Proxmox { proxmox } => {
                if proxmox.api_url.is_empty() {
                    anyhow::bail!("provisioner.proxmox.api_url cannot be empty");
                }
                if proxmox.api_token_id.is_empty() {
                    anyhow::bail!("provisioner.proxmox.api_token_id cannot be empty");
                }
                if proxmox.api_token_secret.is_empty() {
                    anyhow::bail!("provisioner.proxmox.api_token_secret cannot be empty");
                }
                if proxmox.node.is_empty() {
                    anyhow::bail!("provisioner.proxmox.node cannot be empty");
                }
                if proxmox.template_vmid == 0 {
                    anyhow::bail!("provisioner.proxmox.template_vmid must be greater than 0");
                }
                if proxmox.storage.is_empty() {
                    anyhow::bail!("provisioner.proxmox.storage cannot be empty");
                }
            }
            ProvisionerConfig::Script { script } => {
                if script.provision.is_empty() {
                    anyhow::bail!("provisioner.script.provision cannot be empty");
                }
                if script.terminate.is_empty() {
                    anyhow::bail!("provisioner.script.terminate cannot be empty");
                }
                if script.health_check.is_empty() {
                    anyhow::bail!("provisioner.script.health_check cannot be empty");
                }
            }
            ProvisionerConfig::Manual { .. } => {
                // No required fields for manual provisioner
            }
        }

        Ok(())
    }
}
