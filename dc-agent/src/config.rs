use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub api: ApiConfig,
    pub polling: PollingConfig,
    pub provisioner: ProvisionerConfig,
}

#[derive(Debug, Deserialize)]
pub struct ApiConfig {
    pub endpoint: String,
    pub provider_pubkey: String,
    pub provider_secret_key: String, // hex or file path
}

#[derive(Debug, Deserialize)]
pub struct PollingConfig {
    #[serde(default = "default_interval")]
    pub interval_seconds: u64,
    #[serde(default = "default_health_interval")]
    pub health_check_interval_seconds: u64,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ProvisionerConfig {
    Proxmox(ProxmoxConfig),
    Script(ScriptConfig),
    Manual(ManualConfig),
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct ScriptConfig {
    pub provision: String,
    pub terminate: String,
    pub health_check: String,
    #[serde(default = "default_script_timeout")]
    pub timeout_seconds: u64,
}

#[derive(Debug, Deserialize)]
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

// Default functions
fn default_interval() -> u64 {
    30
}

fn default_health_interval() -> u64 {
    300
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_valid_proxmox_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

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
        assert_eq!(
            config.api.provider_pubkey,
            "ed25519_pubkey_hex_abcdef1234567890"
        );
        assert_eq!(
            config.api.provider_secret_key,
            "ed25519_secret_hex_1234567890abcdef"
        );

        assert_eq!(config.polling.interval_seconds, 45);
        assert_eq!(config.polling.health_check_interval_seconds, 600);

        match config.provisioner {
            ProvisionerConfig::Proxmox(proxmox) => {
                assert_eq!(proxmox.api_url, "https://proxmox.local:8006");
                assert_eq!(proxmox.api_token_id, "root@pam!dc-agent");
                assert_eq!(
                    proxmox.api_token_secret,
                    "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
                );
                assert_eq!(proxmox.node, "pve1");
                assert_eq!(proxmox.template_vmid, 9000);
                assert_eq!(proxmox.storage, "local-zfs");
                assert_eq!(proxmox.pool, Some("dc-vms".to_string()));
                assert!(!proxmox.verify_ssl);
            }
            _ => panic!("Expected Proxmox provisioner"),
        }
    }

    #[test]
    fn test_load_valid_script_config() {
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

        match config.provisioner {
            ProvisionerConfig::Script(script) => {
                assert_eq!(script.provision, "/opt/dc-agent/provision.sh");
                assert_eq!(script.terminate, "/opt/dc-agent/terminate.sh");
                assert_eq!(script.health_check, "/opt/dc-agent/health.sh");
                assert_eq!(script.timeout_seconds, 600);
            }
            _ => panic!("Expected Script provisioner"),
        }
    }

    #[test]
    fn test_load_valid_manual_config() {
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

        match config.provisioner {
            ProvisionerConfig::Manual(manual) => {
                assert_eq!(
                    manual.notification_webhook,
                    Some("https://slack.webhook/xyz".to_string())
                );
            }
            _ => panic!("Expected Manual provisioner"),
        }
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

[provisioner.proxmox]
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
        match config.provisioner {
            ProvisionerConfig::Proxmox(proxmox) => {
                assert_eq!(proxmox.storage, "local-lvm");
                assert!(proxmox.verify_ssl);
                assert_eq!(proxmox.pool, None);
            }
            _ => panic!("Expected Proxmox provisioner"),
        }
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

[provisioner.script]
provision = "/opt/dc-agent/provision.sh"
terminate = "/opt/dc-agent/terminate.sh"
health_check = "/opt/dc-agent/health.sh"
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(&config_path).unwrap();

        match config.provisioner {
            ProvisionerConfig::Script(script) => {
                assert_eq!(script.timeout_seconds, 300);
            }
            _ => panic!("Expected Script provisioner"),
        }
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

[provisioner.manual]
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(&config_path).unwrap();

        match config.provisioner {
            ProvisionerConfig::Manual(manual) => {
                assert_eq!(manual.notification_webhook, None);
            }
            _ => panic!("Expected Manual provisioner"),
        }
    }

    #[test]
    fn test_error_on_missing_api_section() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
[polling]

[provisioner]
type = "manual"

[provisioner.manual]
"#;

        fs::write(&config_path, config_content).unwrap();

        let result = Config::load(&config_path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("missing field `api`"));
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

[provisioner.proxmox]
api_url = "https://proxmox.local:8006"
api_token_id = "root@pam!dc-agent"
api_token_secret = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
node = "pve1"
template_vmid = 9000
"#;

        fs::write(&config_path, config_content).unwrap();

        let result = Config::load(&config_path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("missing field `type`"));
    }

    #[test]
    fn test_error_on_missing_proxmox_required_fields() {
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

[provisioner.proxmox]
api_url = "https://proxmox.local:8006"
node = "pve1"
"#;

        fs::write(&config_path, config_content).unwrap();

        let result = Config::load(&config_path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        // Should fail on missing api_token_id or api_token_secret
        assert!(
            err_msg.contains("missing field")
                && (err_msg.contains("api_token_id") || err_msg.contains("api_token_secret"))
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
}
