use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub api: ApiConfig,
    pub polling: PollingConfig,
    /// Default provisioner (required)
    #[serde(deserialize_with = "deserialize_provisioner")]
    pub provisioner: ProvisionerConfig,
    /// Additional provisioners (optional) for per-offering provisioner support
    #[serde(default, deserialize_with = "deserialize_additional_provisioners")]
    pub additional_provisioners: Vec<ProvisionerConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ApiConfig {
    pub endpoint: String,
    pub provider_pubkey: String,
    /// Agent's secret key for signing API requests (hex or file path)
    pub agent_secret_key: Option<String>,
    /// Provider's secret key (legacy - use agent_secret_key instead)
    #[serde(default)]
    pub provider_secret_key: Option<String>,
    /// Pool ID this agent belongs to (set via setup --token)
    #[serde(default)]
    pub pool_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PollingConfig {
    #[serde(default = "default_interval")]
    pub interval_seconds: u64,
    #[serde(default = "default_health_interval")]
    pub health_check_interval_seconds: u64,
    /// Grace period (in seconds) before orphan VMs are automatically terminated.
    /// Orphans are VMs without valid contract IDs. Default: 3600 (1 hour)
    #[serde(default = "default_orphan_grace_period")]
    pub orphan_grace_period_seconds: u64,
}

/// Provisioner configuration enum.
#[derive(Debug)]
pub enum ProvisionerConfig {
    Proxmox(ProxmoxConfig),
    Script(ScriptConfig),
    Manual(ManualConfig),
}

impl ProvisionerConfig {
    pub fn as_proxmox(&self) -> Option<&ProxmoxConfig> {
        match self {
            ProvisionerConfig::Proxmox(cfg) => Some(cfg),
            _ => None,
        }
    }

    pub fn as_script(&self) -> Option<&ScriptConfig> {
        match self {
            ProvisionerConfig::Script(cfg) => Some(cfg),
            _ => None,
        }
    }

    pub fn as_manual(&self) -> Option<&ManualConfig> {
        match self {
            ProvisionerConfig::Manual(cfg) => Some(cfg),
            _ => None,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            ProvisionerConfig::Proxmox(_) => "proxmox",
            ProvisionerConfig::Script(_) => "script",
            ProvisionerConfig::Manual(_) => "manual",
        }
    }
}

/// Intermediate struct for deserializing the provisioner section.
/// Supports both flat format (all fields in [provisioner]) and nested format ([provisioner.proxmox]).
#[derive(Deserialize)]
struct RawProvisionerConfig {
    #[serde(rename = "type")]
    provisioner_type: String,
    // Nested subsections
    proxmox: Option<ProxmoxConfig>,
    script: Option<ScriptConfig>,
    manual: Option<ManualConfig>,
    // Flat fields for proxmox (when not using nested [provisioner.proxmox])
    api_url: Option<String>,
    api_token_id: Option<String>,
    api_token_secret: Option<String>,
    node: Option<String>,
    template_vmid: Option<u32>,
    #[serde(default = "default_storage")]
    storage: String,
    pool: Option<String>,
    #[serde(default = "default_verify_ssl")]
    verify_ssl: bool,
    // Flat fields for script
    provision: Option<String>,
    terminate: Option<String>,
    health_check: Option<String>,
    #[serde(default = "default_script_timeout")]
    timeout_seconds: u64,
    // Flat fields for manual
    notification_webhook: Option<String>,
}

fn deserialize_provisioner<'de, D>(deserializer: D) -> Result<ProvisionerConfig, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = RawProvisionerConfig::deserialize(deserializer)?;

    match raw.provisioner_type.as_str() {
        "proxmox" => {
            // Prefer nested [provisioner.proxmox] section if present
            let config = if let Some(nested) = raw.proxmox {
                nested
            } else {
                // Fall back to flat format
                let api_url = raw
                    .api_url
                    .ok_or_else(|| serde::de::Error::missing_field("api_url"))?;
                let api_token_id = raw
                    .api_token_id
                    .ok_or_else(|| serde::de::Error::missing_field("api_token_id"))?;
                let api_token_secret = raw
                    .api_token_secret
                    .ok_or_else(|| serde::de::Error::missing_field("api_token_secret"))?;
                let node = raw
                    .node
                    .ok_or_else(|| serde::de::Error::missing_field("node"))?;
                let template_vmid = raw
                    .template_vmid
                    .ok_or_else(|| serde::de::Error::missing_field("template_vmid"))?;

                ProxmoxConfig {
                    api_url,
                    api_token_id,
                    api_token_secret,
                    node,
                    template_vmid,
                    storage: raw.storage,
                    pool: raw.pool,
                    verify_ssl: raw.verify_ssl,
                }
            };
            Ok(ProvisionerConfig::Proxmox(config))
        }
        "script" => {
            let config = if let Some(nested) = raw.script {
                nested
            } else {
                let provision = raw
                    .provision
                    .ok_or_else(|| serde::de::Error::missing_field("provision"))?;
                let terminate = raw
                    .terminate
                    .ok_or_else(|| serde::de::Error::missing_field("terminate"))?;
                let health_check = raw
                    .health_check
                    .ok_or_else(|| serde::de::Error::missing_field("health_check"))?;

                ScriptConfig {
                    provision,
                    terminate,
                    health_check,
                    timeout_seconds: raw.timeout_seconds,
                }
            };
            Ok(ProvisionerConfig::Script(config))
        }
        "manual" => {
            let config = if let Some(nested) = raw.manual {
                nested
            } else {
                ManualConfig {
                    notification_webhook: raw.notification_webhook,
                }
            };
            Ok(ProvisionerConfig::Manual(config))
        }
        other => Err(serde::de::Error::unknown_variant(
            other,
            &["proxmox", "script", "manual"],
        )),
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProxmoxConfig {
    pub api_url: String,
    pub api_token_id: String,
    pub api_token_secret: String,
    pub node: String,
    pub template_vmid: u32,
    #[serde(default = "default_storage")]
    pub storage: String,
    pub pool: Option<String>,
    #[serde(default = "default_verify_ssl")]
    pub verify_ssl: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScriptConfig {
    pub provision: String,
    pub terminate: String,
    pub health_check: String,
    #[serde(default = "default_script_timeout")]
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManualConfig {
    pub notification_webhook: Option<String>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))
    }
}

fn default_interval() -> u64 {
    30
}

fn default_health_interval() -> u64 {
    300
}

fn default_orphan_grace_period() -> u64 {
    3600 // 1 hour
}

fn default_storage() -> String {
    "local-lvm".to_string()
}

fn default_verify_ssl() -> bool {
    true
}

fn default_script_timeout() -> u64 {
    300
}

/// Deserialize additional provisioners from [[additional_provisioners]] array
fn deserialize_additional_provisioners<'de, D>(
    deserializer: D,
) -> Result<Vec<ProvisionerConfig>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw_list: Vec<RawProvisionerConfig> = Vec::deserialize(deserializer)?;
    let mut result = Vec::with_capacity(raw_list.len());

    for raw in raw_list {
        let config = match raw.provisioner_type.as_str() {
            "proxmox" => {
                let config = if let Some(nested) = raw.proxmox {
                    nested
                } else {
                    let api_url = raw
                        .api_url
                        .ok_or_else(|| serde::de::Error::missing_field("api_url"))?;
                    let api_token_id = raw
                        .api_token_id
                        .ok_or_else(|| serde::de::Error::missing_field("api_token_id"))?;
                    let api_token_secret = raw
                        .api_token_secret
                        .ok_or_else(|| serde::de::Error::missing_field("api_token_secret"))?;
                    let node = raw
                        .node
                        .ok_or_else(|| serde::de::Error::missing_field("node"))?;
                    let template_vmid = raw
                        .template_vmid
                        .ok_or_else(|| serde::de::Error::missing_field("template_vmid"))?;

                    ProxmoxConfig {
                        api_url,
                        api_token_id,
                        api_token_secret,
                        node,
                        template_vmid,
                        storage: raw.storage,
                        pool: raw.pool,
                        verify_ssl: raw.verify_ssl,
                    }
                };
                ProvisionerConfig::Proxmox(config)
            }
            "script" => {
                let config = if let Some(nested) = raw.script {
                    nested
                } else {
                    let provision = raw
                        .provision
                        .ok_or_else(|| serde::de::Error::missing_field("provision"))?;
                    let terminate = raw
                        .terminate
                        .ok_or_else(|| serde::de::Error::missing_field("terminate"))?;
                    let health_check = raw
                        .health_check
                        .ok_or_else(|| serde::de::Error::missing_field("health_check"))?;

                    ScriptConfig {
                        provision,
                        terminate,
                        health_check,
                        timeout_seconds: raw.timeout_seconds,
                    }
                };
                ProvisionerConfig::Script(config)
            }
            "manual" => {
                let config = if let Some(nested) = raw.manual {
                    nested
                } else {
                    ManualConfig {
                        notification_webhook: raw.notification_webhook,
                    }
                };
                ProvisionerConfig::Manual(config)
            }
            other => {
                return Err(serde::de::Error::unknown_variant(
                    other,
                    &["proxmox", "script", "manual"],
                ))
            }
        };
        result.push(config);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_nested_proxmox_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Nested format: [provisioner.proxmox]
        let config_content = r#"
[api]
endpoint = "https://api.decent-cloud.org"
provider_pubkey = "ed25519_pubkey_hex_abcdef1234567890"
provider_secret_key = "ed25519_secret_hex_1234567890abcdef"

[polling]
interval_seconds = 45
health_check_interval_seconds = 600

[provisioner]
type = "proxmox"

[provisioner.proxmox]
api_url = "https://proxmox.local:8006"
api_token_id = "root@pam!dc-agent"
api_token_secret = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
node = "pve1"
template_vmid = 9000
storage = "local-zfs"
pool = "dc-vms"
verify_ssl = false
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(&config_path).unwrap();

        assert_eq!(config.api.endpoint, "https://api.decent-cloud.org");
        assert_eq!(config.polling.interval_seconds, 45);

        let proxmox = config.provisioner.as_proxmox().expect("Should be Proxmox");
        assert_eq!(proxmox.api_url, "https://proxmox.local:8006");
        assert_eq!(proxmox.api_token_id, "root@pam!dc-agent");
        assert_eq!(proxmox.node, "pve1");
        assert_eq!(proxmox.template_vmid, 9000);
        assert_eq!(proxmox.storage, "local-zfs");
        assert_eq!(proxmox.pool, Some("dc-vms".to_string()));
        assert!(!proxmox.verify_ssl);
    }

    #[test]
    fn test_load_flat_proxmox_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Flat format: all in [provisioner]
        let config_content = r#"
[api]
endpoint = "https://api.decent-cloud.org"
provider_pubkey = "ed25519_pubkey_hex_abcdef1234567890"
provider_secret_key = "ed25519_secret_hex_1234567890abcdef"

[polling]
interval_seconds = 45
health_check_interval_seconds = 600

[provisioner]
type = "proxmox"
api_url = "https://proxmox.local:8006"
api_token_id = "root@pam!dc-agent"
api_token_secret = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
node = "pve1"
template_vmid = 9000
storage = "local-zfs"
pool = "dc-vms"
verify_ssl = false
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(&config_path).unwrap();

        let proxmox = config.provisioner.as_proxmox().expect("Should be Proxmox");
        assert_eq!(proxmox.api_url, "https://proxmox.local:8006");
        assert_eq!(proxmox.api_token_id, "root@pam!dc-agent");
        assert_eq!(proxmox.storage, "local-zfs");
        assert!(!proxmox.verify_ssl);
    }

    #[test]
    fn test_load_nested_script_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
[api]
endpoint = "https://api.decent-cloud.org"
provider_pubkey = "ed25519_pubkey_hex"
provider_secret_key = "ed25519_secret_hex"

[polling]

[provisioner]
type = "script"

[provisioner.script]
provision = "/opt/dc-agent/provision.sh"
terminate = "/opt/dc-agent/terminate.sh"
health_check = "/opt/dc-agent/health.sh"
timeout_seconds = 600
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(&config_path).unwrap();

        let script = config.provisioner.as_script().expect("Should be Script");
        assert_eq!(script.provision, "/opt/dc-agent/provision.sh");
        assert_eq!(script.terminate, "/opt/dc-agent/terminate.sh");
        assert_eq!(script.health_check, "/opt/dc-agent/health.sh");
        assert_eq!(script.timeout_seconds, 600);
    }

    #[test]
    fn test_load_flat_script_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
[api]
endpoint = "https://api.decent-cloud.org"
provider_pubkey = "ed25519_pubkey_hex"
provider_secret_key = "ed25519_secret_hex"

[polling]

[provisioner]
type = "script"
provision = "/opt/dc-agent/provision.sh"
terminate = "/opt/dc-agent/terminate.sh"
health_check = "/opt/dc-agent/health.sh"
timeout_seconds = 600
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(&config_path).unwrap();

        let script = config.provisioner.as_script().expect("Should be Script");
        assert_eq!(script.provision, "/opt/dc-agent/provision.sh");
        assert_eq!(script.timeout_seconds, 600);
    }

    #[test]
    fn test_load_nested_manual_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
[api]
endpoint = "https://api.decent-cloud.org"
provider_pubkey = "ed25519_pubkey_hex"
provider_secret_key = "ed25519_secret_hex"

[polling]

[provisioner]
type = "manual"

[provisioner.manual]
notification_webhook = "https://slack.webhook/xyz"
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(&config_path).unwrap();

        let manual = config.provisioner.as_manual().expect("Should be Manual");
        assert_eq!(
            manual.notification_webhook,
            Some("https://slack.webhook/xyz".to_string())
        );
    }

    #[test]
    fn test_load_flat_manual_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
[api]
endpoint = "https://api.decent-cloud.org"
provider_pubkey = "ed25519_pubkey_hex"
provider_secret_key = "ed25519_secret_hex"

[polling]

[provisioner]
type = "manual"
notification_webhook = "https://slack.webhook/xyz"
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(&config_path).unwrap();

        let manual = config.provisioner.as_manual().expect("Should be Manual");
        assert_eq!(
            manual.notification_webhook,
            Some("https://slack.webhook/xyz".to_string())
        );
    }

    #[test]
    fn test_default_values_applied() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
[api]
endpoint = "https://api.decent-cloud.org"
provider_pubkey = "ed25519_pubkey_hex"
provider_secret_key = "ed25519_secret_hex"

[polling]

[provisioner]
type = "proxmox"
api_url = "https://proxmox.local:8006"
api_token_id = "root@pam!dc-agent"
api_token_secret = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
node = "pve1"
template_vmid = 9000
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(&config_path).unwrap();

        // Check polling defaults
        assert_eq!(config.polling.interval_seconds, 30);
        assert_eq!(config.polling.health_check_interval_seconds, 300);

        // Check proxmox defaults
        let proxmox = config.provisioner.as_proxmox().expect("Should be Proxmox");
        assert_eq!(proxmox.storage, "local-lvm");
        assert!(proxmox.verify_ssl);
        assert_eq!(proxmox.pool, None);
    }

    #[test]
    fn test_script_config_default_timeout() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
[api]
endpoint = "https://api.decent-cloud.org"
provider_pubkey = "ed25519_pubkey_hex"
provider_secret_key = "ed25519_secret_hex"

[polling]

[provisioner]
type = "script"
provision = "/opt/dc-agent/provision.sh"
terminate = "/opt/dc-agent/terminate.sh"
health_check = "/opt/dc-agent/health.sh"
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(&config_path).unwrap();

        let script = config.provisioner.as_script().expect("Should be Script");
        assert_eq!(script.timeout_seconds, 300);
    }

    #[test]
    fn test_manual_config_without_webhook() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
[api]
endpoint = "https://api.decent-cloud.org"
provider_pubkey = "ed25519_pubkey_hex"
provider_secret_key = "ed25519_secret_hex"

[polling]

[provisioner]
type = "manual"
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(&config_path).unwrap();

        let manual = config.provisioner.as_manual().expect("Should be Manual");
        assert_eq!(manual.notification_webhook, None);
    }

    #[test]
    fn test_error_on_missing_api_section() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
[polling]

[provisioner]
type = "manual"
"#;

        fs::write(&config_path, config_content).unwrap();

        let result = Config::load(&config_path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let full_err = format!("{:#}", err);
        assert!(full_err.contains("missing field"));
    }

    #[test]
    fn test_error_on_missing_provisioner_type() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
[api]
endpoint = "https://api.decent-cloud.org"
provider_pubkey = "ed25519_pubkey_hex"
provider_secret_key = "ed25519_secret_hex"

[polling]

[provisioner]
api_url = "https://proxmox.local:8006"
api_token_id = "root@pam!dc-agent"
api_token_secret = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
node = "pve1"
template_vmid = 9000
"#;

        fs::write(&config_path, config_content).unwrap();

        let result = Config::load(&config_path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let full_err = format!("{:#}", err);
        assert!(full_err.contains("missing field") && full_err.contains("type"));
    }

    #[test]
    fn test_proxmox_config_requires_all_fields() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
[api]
endpoint = "https://api.decent-cloud.org"
provider_pubkey = "ed25519_pubkey_hex"
provider_secret_key = "ed25519_secret_hex"

[polling]

[provisioner]
type = "proxmox"
api_url = "https://proxmox.local:8006"
node = "pve1"
"#;

        fs::write(&config_path, config_content).unwrap();

        let result = Config::load(&config_path);
        assert!(
            result.is_err(),
            "Should fail due to missing required fields"
        );
        let err_msg = format!("{:#}", result.unwrap_err());
        assert!(
            err_msg.contains("missing field"),
            "Expected 'missing field' in error: {}",
            err_msg
        );
    }

    #[test]
    fn test_error_on_invalid_toml_syntax() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
[api
endpoint = "https://api.decent-cloud.org"
"#;

        fs::write(&config_path, config_content).unwrap();

        let result = Config::load(&config_path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to parse config file"));
    }

    #[test]
    fn test_error_on_nonexistent_file() {
        let config_path = Path::new("/nonexistent/path/config.toml");

        let result = Config::load(config_path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to read config file"));
    }

    #[test]
    fn test_provisioner_type_name() {
        let proxmox = ProvisionerConfig::Proxmox(ProxmoxConfig {
            api_url: "https://test:8006".to_string(),
            api_token_id: "test".to_string(),
            api_token_secret: "secret".to_string(),
            node: "node".to_string(),
            template_vmid: 9000,
            storage: "local".to_string(),
            pool: None,
            verify_ssl: true,
        });
        assert_eq!(proxmox.type_name(), "proxmox");

        let script = ProvisionerConfig::Script(ScriptConfig {
            provision: "/p.sh".to_string(),
            terminate: "/t.sh".to_string(),
            health_check: "/h.sh".to_string(),
            timeout_seconds: 300,
        });
        assert_eq!(script.type_name(), "script");

        let manual = ProvisionerConfig::Manual(ManualConfig {
            notification_webhook: None,
        });
        assert_eq!(manual.type_name(), "manual");
    }

    #[test]
    fn test_error_on_unknown_provisioner_type() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
[api]
endpoint = "https://api.decent-cloud.org"
provider_pubkey = "ed25519_pubkey_hex"
provider_secret_key = "ed25519_secret_hex"

[polling]

[provisioner]
type = "unknown"
"#;

        fs::write(&config_path, config_content).unwrap();

        let result = Config::load(&config_path);
        assert!(result.is_err());
        let err_msg = format!("{:#}", result.unwrap_err());
        assert!(
            err_msg.contains("unknown variant"),
            "Expected 'unknown variant' in error: {}",
            err_msg
        );
    }

    #[test]
    fn test_load_with_additional_provisioners() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Config with default provisioner + additional provisioners
        let config_content = r#"
[api]
endpoint = "https://api.decent-cloud.org"
provider_pubkey = "ed25519_pubkey_hex"
provider_secret_key = "ed25519_secret_hex"

[polling]

[provisioner]
type = "proxmox"
api_url = "https://proxmox.local:8006"
api_token_id = "root@pam!dc-agent"
api_token_secret = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
node = "pve1"
template_vmid = 9000

[[additional_provisioners]]
type = "script"
provision = "/opt/dc-agent/provision.sh"
terminate = "/opt/dc-agent/terminate.sh"
health_check = "/opt/dc-agent/health.sh"

[[additional_provisioners]]
type = "manual"
notification_webhook = "https://slack.webhook/xyz"
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(&config_path).unwrap();

        // Check default provisioner is Proxmox
        assert!(config.provisioner.as_proxmox().is_some());
        assert_eq!(config.provisioner.type_name(), "proxmox");

        // Check additional provisioners
        assert_eq!(config.additional_provisioners.len(), 2);
        assert_eq!(config.additional_provisioners[0].type_name(), "script");
        assert_eq!(config.additional_provisioners[1].type_name(), "manual");

        let script = config.additional_provisioners[0]
            .as_script()
            .expect("Should be Script");
        assert_eq!(script.provision, "/opt/dc-agent/provision.sh");

        let manual = config.additional_provisioners[1]
            .as_manual()
            .expect("Should be Manual");
        assert_eq!(
            manual.notification_webhook,
            Some("https://slack.webhook/xyz".to_string())
        );
    }

    #[test]
    fn test_load_without_additional_provisioners() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Standard config without additional provisioners - should work (backward compat)
        let config_content = r#"
[api]
endpoint = "https://api.decent-cloud.org"
provider_pubkey = "ed25519_pubkey_hex"
provider_secret_key = "ed25519_secret_hex"

[polling]

[provisioner]
type = "manual"
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(&config_path).unwrap();

        assert!(config.provisioner.as_manual().is_some());
        assert!(config.additional_provisioners.is_empty());
    }
}
