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
    /// Gateway configuration for DC-level reverse proxy (optional)
    #[serde(default)]
    pub gateway: Option<GatewayConfig>,
}

/// Gateway configuration for per-host reverse proxy (Caddy)
#[derive(Debug, Clone, Deserialize)]
pub struct GatewayConfig {
    /// Unique datacenter identifier (2-20 chars [a-z0-9-], e.g., "a3x9f2b1")
    /// Generate with: openssl rand -hex 4
    pub dc_id: String,
    /// This host's public IPv4 address
    pub public_ip: String,
    /// Base domain for gateway subdomains (default: "decent-cloud.org")
    #[serde(default = "default_domain")]
    pub domain: String,
    /// Gateway DNS prefix (default: "gw", use "dev-gw" for dev)
    #[serde(default = "default_gw_prefix")]
    pub gw_prefix: String,
    /// Start of port range for TCP/UDP mapping (default: 20000)
    #[serde(default = "default_port_range_start")]
    pub port_range_start: u16,
    /// End of port range for TCP/UDP mapping (default: 59999)
    #[serde(default = "default_port_range_end")]
    pub port_range_end: u16,
    /// Number of ports to allocate per VM (default: 10)
    #[serde(default = "default_ports_per_vm")]
    pub ports_per_vm: u16,
    /// Directory for Caddy site configuration files
    #[serde(default = "default_caddy_sites_dir")]
    pub caddy_sites_dir: String,
    /// DEPRECATED: Legacy field name for caddy_sites_dir. Traefik is no longer supported.
    /// If set, Config::load() will fail with a clear error message.
    #[serde(default)]
    traefik_dynamic_dir: Option<String>,
    /// Path to port allocations state file
    #[serde(default = "default_port_allocations_path")]
    pub port_allocations_path: String,
    // Note: DNS is managed via the central API (/api/v1/agents/dns)
    // TLS: per-provider wildcard cert via DNS-01 with acme-dns (*.{dc_id}.{gw_prefix}.{domain})
}

fn default_domain() -> String {
    "decent-cloud.org".to_string()
}

fn default_gw_prefix() -> String {
    "gw".to_string()
}

fn default_port_range_start() -> u16 {
    20000
}

fn default_port_range_end() -> u16 {
    59999
}

fn default_ports_per_vm() -> u16 {
    10
}

fn default_caddy_sites_dir() -> String {
    "/etc/caddy/sites".to_string()
}

fn default_port_allocations_path() -> String {
    "/var/lib/dc-agent/port-allocations.json".to_string()
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
    /// Path to orphan tracker persistence file.
    /// Default: /var/lib/dc-agent/orphans.json
    #[serde(default = "default_orphan_tracker_path")]
    pub orphan_tracker_path: String,
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
    #[serde(default = "default_ip_wait_attempts")]
    ip_wait_attempts: u32,
    #[serde(default = "default_ip_wait_interval_secs")]
    ip_wait_interval_secs: u64,
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
            // Check for dual format (both flat fields AND nested section)
            let has_flat_fields = raw.api_url.is_some()
                || raw.api_token_id.is_some()
                || raw.api_token_secret.is_some();
            let has_nested = raw.proxmox.is_some();
            if has_flat_fields && has_nested {
                // Can't use tracing in serde deserializer, print to stderr
                eprintln!("WARNING: Both flat and nested provisioner config found for 'proxmox'. Using nested [provisioner.proxmox] format, flat fields ignored.");
            }

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
                    ip_wait_attempts: raw.ip_wait_attempts,
                    ip_wait_interval_secs: raw.ip_wait_interval_secs,
                }
            };
            Ok(ProvisionerConfig::Proxmox(config))
        }
        "script" => {
            // Check for dual format (both flat fields AND nested section)
            let has_flat_fields =
                raw.provision.is_some() || raw.terminate.is_some() || raw.health_check.is_some();
            let has_nested = raw.script.is_some();
            if has_flat_fields && has_nested {
                eprintln!("WARNING: Both flat and nested provisioner config found for 'script'. Using nested [provisioner.script] format, flat fields ignored.");
            }

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
            // Check for dual format (both flat fields AND nested section)
            let has_flat_fields = raw.notification_webhook.is_some();
            let has_nested = raw.manual.is_some();
            if has_flat_fields && has_nested {
                eprintln!("WARNING: Both flat and nested provisioner config found for 'manual'. Using nested [provisioner.manual] format, flat fields ignored.");
            }

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
    /// Number of attempts to wait for VM IP address (default: 12)
    #[serde(default = "default_ip_wait_attempts")]
    pub ip_wait_attempts: u32,
    /// Seconds between IP address check attempts (default: 10)
    #[serde(default = "default_ip_wait_interval_secs")]
    pub ip_wait_interval_secs: u64,
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

/// Placeholder patterns that indicate unconfigured values
const PLACEHOLDER_PATTERNS: &[&str] = &[
    "YOUR-PROXMOX-HOST",
    "REPLACE-WITH-YOUR",
    "YOUR_TOKEN",
    "CHANGEME",
    "YOUR-API",
    "EXAMPLE.COM",
];

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        // Check for deprecated traefik_dynamic_dir
        if let Some(ref gw) = config.gateway {
            if gw.traefik_dynamic_dir.is_some() {
                anyhow::bail!(
                    "Config uses deprecated 'traefik_dynamic_dir'. Rename to 'caddy_sites_dir'. \
                     Traefik is no longer supported - the gateway now uses Caddy."
                );
            }
            // Validate dc_id format
            let dc_id = &gw.dc_id;
            if dc_id.len() < 2 || dc_id.len() > 20 {
                anyhow::bail!(
                    "Invalid gateway.dc_id '{}': must be 2-20 characters",
                    dc_id
                );
            }
            if !dc_id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            {
                anyhow::bail!(
                    "Invalid gateway.dc_id '{}': must contain only [a-z0-9-]",
                    dc_id
                );
            }
            if dc_id.starts_with('-') || dc_id.ends_with('-') {
                anyhow::bail!(
                    "Invalid gateway.dc_id '{}': must not start or end with a hyphen",
                    dc_id
                );
            }
        }

        Ok(config)
    }

    /// Validate config for placeholder values that indicate unconfigured settings.
    /// Returns Err with a clear message if placeholders are found.
    pub fn validate(&self) -> Result<()> {
        let mut placeholders_found = Vec::new();

        // Check provisioner config based on type
        match &self.provisioner {
            ProvisionerConfig::Proxmox(proxmox) => {
                Self::check_placeholder(
                    &proxmox.api_url,
                    "provisioner.proxmox.api_url",
                    &mut placeholders_found,
                );
                Self::check_placeholder(
                    &proxmox.api_token_id,
                    "provisioner.proxmox.api_token_id",
                    &mut placeholders_found,
                );
                Self::check_placeholder(
                    &proxmox.api_token_secret,
                    "provisioner.proxmox.api_token_secret",
                    &mut placeholders_found,
                );
                Self::check_placeholder(
                    &proxmox.node,
                    "provisioner.proxmox.node",
                    &mut placeholders_found,
                );
            }
            ProvisionerConfig::Script(script) => {
                Self::check_placeholder(
                    &script.provision,
                    "provisioner.script.provision",
                    &mut placeholders_found,
                );
                Self::check_placeholder(
                    &script.terminate,
                    "provisioner.script.terminate",
                    &mut placeholders_found,
                );
                Self::check_placeholder(
                    &script.health_check,
                    "provisioner.script.health_check",
                    &mut placeholders_found,
                );
            }
            ProvisionerConfig::Manual(manual) => {
                if let Some(webhook) = &manual.notification_webhook {
                    Self::check_placeholder(
                        webhook,
                        "provisioner.manual.notification_webhook",
                        &mut placeholders_found,
                    );
                }
            }
        }

        // Check additional provisioners
        for (idx, additional) in self.additional_provisioners.iter().enumerate() {
            match additional {
                ProvisionerConfig::Proxmox(proxmox) => {
                    Self::check_placeholder(
                        &proxmox.api_url,
                        &format!("additional_provisioners[{}].api_url", idx),
                        &mut placeholders_found,
                    );
                    Self::check_placeholder(
                        &proxmox.api_token_secret,
                        &format!("additional_provisioners[{}].api_token_secret", idx),
                        &mut placeholders_found,
                    );
                }
                ProvisionerConfig::Script(script) => {
                    Self::check_placeholder(
                        &script.provision,
                        &format!("additional_provisioners[{}].provision", idx),
                        &mut placeholders_found,
                    );
                }
                ProvisionerConfig::Manual(_) => {}
            }
        }

        if !placeholders_found.is_empty() {
            anyhow::bail!(
                "Config contains placeholder values that must be configured:\n  - {}\n\nRun 'dc-agent setup token --token <TOKEN>' to configure, or edit the config file manually.",
                placeholders_found.join("\n  - ")
            );
        }

        Ok(())
    }

    /// Check if a value contains any placeholder patterns
    fn check_placeholder(value: &str, field_name: &str, placeholders: &mut Vec<String>) {
        let upper = value.to_uppercase();
        for pattern in PLACEHOLDER_PATTERNS {
            if upper.contains(pattern) {
                placeholders.push(format!("{} contains '{}'", field_name, pattern));
                break;
            }
        }
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

fn default_orphan_tracker_path() -> String {
    "/var/lib/dc-agent/orphans.json".to_string()
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

fn default_ip_wait_attempts() -> u32 {
    12
}

fn default_ip_wait_interval_secs() -> u64 {
    10
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
                        ip_wait_attempts: raw.ip_wait_attempts,
                        ip_wait_interval_secs: raw.ip_wait_interval_secs,
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
        assert_eq!(config.polling.orphan_grace_period_seconds, 3600);
        assert_eq!(
            config.polling.orphan_tracker_path,
            "/var/lib/dc-agent/orphans.json"
        );

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
            ip_wait_attempts: 12,
            ip_wait_interval_secs: 10,
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

    #[test]
    fn test_load_with_gateway_config() {
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

[gateway]
dc_id = "a3x9f2b1"
public_ip = "203.0.113.1"
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(&config_path).unwrap();

        let gateway = config.gateway.expect("Gateway should be configured");
        assert_eq!(gateway.dc_id, "a3x9f2b1");
        assert_eq!(gateway.public_ip, "203.0.113.1");
        assert_eq!(gateway.domain, "decent-cloud.org");
        assert_eq!(gateway.gw_prefix, "gw");
        assert_eq!(gateway.port_range_start, 20000);
        assert_eq!(gateway.port_range_end, 59999);
        assert_eq!(gateway.ports_per_vm, 10);
        assert_eq!(gateway.caddy_sites_dir, "/etc/caddy/sites");
        assert_eq!(
            gateway.port_allocations_path,
            "/var/lib/dc-agent/port-allocations.json"
        );
    }

    #[test]
    fn test_load_with_gateway_custom_ports() {
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

[gateway]
dc_id = "dc-us"
public_ip = "10.0.0.1"
port_range_start = 30000
port_range_end = 40000
ports_per_vm = 5
caddy_sites_dir = "/custom/caddy"
port_allocations_path = "/custom/allocations.json"
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(&config_path).unwrap();

        let gateway = config.gateway.expect("Gateway should be configured");
        assert_eq!(gateway.dc_id, "dc-us");
        assert_eq!(gateway.port_range_start, 30000);
        assert_eq!(gateway.port_range_end, 40000);
        assert_eq!(gateway.ports_per_vm, 5);
        assert_eq!(gateway.caddy_sites_dir, "/custom/caddy");
        assert_eq!(gateway.port_allocations_path, "/custom/allocations.json");
    }

    #[test]
    fn test_deprecated_traefik_dynamic_dir_fails() {
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

[gateway]
dc_id = "dc-us"
public_ip = "10.0.0.1"
traefik_dynamic_dir = "/old/traefik/path"
"#;

        fs::write(&config_path, config_content).unwrap();

        let result = Config::load(&config_path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("traefik_dynamic_dir") && err_msg.contains("caddy_sites_dir"),
            "Error should mention both old and new field names: {}",
            err_msg
        );
    }

    #[test]
    fn test_gateway_dc_id_validation_too_short() {
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

[gateway]
dc_id = "a"
public_ip = "10.0.0.1"
"#;

        fs::write(&config_path, config_content).unwrap();

        let result = Config::load(&config_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("2-20 characters"));
    }

    #[test]
    fn test_gateway_dc_id_validation_leading_hyphen() {
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

[gateway]
dc_id = "-abc"
public_ip = "10.0.0.1"
"#;

        fs::write(&config_path, config_content).unwrap();

        let result = Config::load(&config_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("hyphen"));
    }

    #[test]
    fn test_gateway_dc_id_validation_uppercase_rejected() {
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

[gateway]
dc_id = "DC-LK"
public_ip = "10.0.0.1"
"#;

        fs::write(&config_path, config_content).unwrap();

        let result = Config::load(&config_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("[a-z0-9-]"));
    }

    #[test]
    fn test_load_without_gateway_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Config without gateway section should still work (backward compat)
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

        assert!(config.gateway.is_none());
    }

    #[test]
    fn test_validate_rejects_proxmox_placeholder_api_url() {
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
api_url = "https://YOUR-PROXMOX-HOST:8006"
api_token_id = "root@pam!dc-agent"
api_token_secret = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
node = "pve1"
template_vmid = 9000
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(&config_path).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("YOUR-PROXMOX-HOST"),
            "Error should mention placeholder: {}",
            err_msg
        );
    }

    #[test]
    fn test_validate_rejects_proxmox_placeholder_token_secret() {
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
api_token_secret = "REPLACE-WITH-YOUR-API-TOKEN-SECRET"
node = "pve1"
template_vmid = 9000
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(&config_path).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("REPLACE-WITH-YOUR"),
            "Error should mention placeholder: {}",
            err_msg
        );
    }

    #[test]
    fn test_validate_accepts_valid_proxmox_config() {
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
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_accepts_manual_config() {
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
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_rejects_changeme_placeholder() {
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
api_token_secret = "changeme"
node = "pve1"
template_vmid = 9000
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(&config_path).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("CHANGEME"),
            "Error should mention placeholder: {}",
            err_msg
        );
    }

    #[test]
    fn test_validate_case_insensitive() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Test lowercase version of placeholder
        let config_content = r#"
[api]
endpoint = "https://api.decent-cloud.org"
provider_pubkey = "ed25519_pubkey_hex"
provider_secret_key = "ed25519_secret_hex"

[polling]

[provisioner]
type = "proxmox"
api_url = "https://your-proxmox-host:8006"
api_token_id = "root@pam!dc-agent"
api_token_secret = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
node = "pve1"
template_vmid = 9000
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(&config_path).unwrap();
        let result = config.validate();
        assert!(result.is_err(), "Should reject lowercase placeholder too");
    }

    #[test]
    fn test_validate_multiple_placeholders() {
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
api_url = "https://YOUR-PROXMOX-HOST:8006"
api_token_id = "YOUR_TOKEN_ID"
api_token_secret = "REPLACE-WITH-YOUR-API-TOKEN-SECRET"
node = "pve1"
template_vmid = 9000
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(&config_path).unwrap();
        let result = config.validate();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        // Should report all placeholders
        assert!(
            err_msg.contains("api_url") && err_msg.contains("api_token"),
            "Error should mention both fields: {}",
            err_msg
        );
    }

    #[test]
    fn test_custom_orphan_tracker_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
[api]
endpoint = "https://api.decent-cloud.org"
provider_pubkey = "ed25519_pubkey_hex"
provider_secret_key = "ed25519_secret_hex"

[polling]
orphan_tracker_path = "/custom/path/orphans.json"

[provisioner]
type = "manual"
"#;

        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(&config_path).unwrap();
        assert_eq!(
            config.polling.orphan_tracker_path,
            "/custom/path/orphans.json"
        );
    }
}
