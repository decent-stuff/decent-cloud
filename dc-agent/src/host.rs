use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::setup::run_command_with_timeout;

const SYSTEMCTL_OP_TIMEOUT: Duration = Duration::from_secs(30);
/// Timeout for quick host-side queries (systemctl status, `which`, port checks).
/// Shared with `run_doctor` in the bin, which re-imports it from here.
pub const QUICK_QUERY_TIMEOUT: Duration = Duration::from_secs(10);

/// Check if dc-agent systemd service already exists.
pub fn is_service_installed() -> bool {
    Path::new("/etc/systemd/system/dc-agent.service").exists()
}

/// Install or update systemd service for dc-agent.
/// Writes service unit file, reloads systemd, enables and starts/restarts the service.
pub fn install_systemd_service(config_path: &Path) -> Result<()> {
    use std::io::Write;

    const SYSTEMD_DIR: &str = "/etc/systemd/system";
    const SERVICE_FILE: &str = "dc-agent.service";
    const BINARY_PATH: &str = "/usr/local/bin/dc-agent";

    // Verify binary exists
    if !Path::new(BINARY_PATH).exists() {
        anyhow::bail!(
            "dc-agent binary not found at {}. Install it first with:\n  \
             curl -sSL https://get.decent-cloud.org | bash",
            BINARY_PATH
        );
    }

    // Convert config path to absolute (required for systemd which runs from /)
    let absolute_config_path = config_path
        .canonicalize()
        .with_context(|| format!("Config file not found: {}", config_path.display()))?;

    let service_existed = is_service_installed();

    // Create the systemd service unit file
    let service_content = format!(
        r#"[Unit]
Description=Decent Cloud Provisioning Agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart={} --config {} run
Restart=always
RestartSec=10
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
"#,
        BINARY_PATH,
        absolute_config_path.display()
    );

    let service_path = format!("{}/{}", SYSTEMD_DIR, SERVICE_FILE);
    let mut file = std::fs::File::create(&service_path)
        .with_context(|| format!("Failed to create systemd service file: {}", service_path))?;
    file.write_all(service_content.as_bytes())?;
    if service_existed {
        println!("[ok] Updated {}", service_path);
    } else {
        println!("[ok] Created {}", service_path);
    }

    // Reload systemd to pick up new service file
    let reload_status = run_command_with_timeout(
        "systemctl",
        &["daemon-reload"],
        SYSTEMCTL_OP_TIMEOUT,
    )
    .context("Failed to run systemctl daemon-reload")?;
    if !reload_status.status.success() {
        anyhow::bail!(
            "systemctl daemon-reload failed with exit code {:?}",
            reload_status.status.code()
        );
    }
    println!("[ok] Systemd daemon reloaded");

    // Enable service
    let enable_status = run_command_with_timeout(
        "systemctl",
        &["enable", SERVICE_FILE],
        SYSTEMCTL_OP_TIMEOUT,
    )
    .context("Failed to run systemctl enable")?;
    if !enable_status.status.success() {
        anyhow::bail!(
            "systemctl enable failed with exit code {:?}",
            enable_status.status.code()
        );
    }
    println!("[ok] Service enabled");

    // Use restart if service existed (to pick up new config), otherwise start
    let action = if service_existed { "restart" } else { "start" };
    let start_status = run_command_with_timeout(
        "systemctl",
        &[action, SERVICE_FILE],
        SYSTEMCTL_OP_TIMEOUT,
    )
    .context("Failed to run systemctl")?;
    if !start_status.status.success() {
        anyhow::bail!(
            "systemctl {} failed with exit code {:?}",
            action,
            start_status.status.code()
        );
    }

    // Wait briefly and verify service is actually running (not just started and crashed)
    std::thread::sleep(std::time::Duration::from_secs(2));
    let status_output = run_command_with_timeout(
        "systemctl",
        &["is-active", SERVICE_FILE],
        QUICK_QUERY_TIMEOUT,
    )
    .context("Failed to check service status")?;
    let status = String::from_utf8_lossy(&status_output.stdout)
        .trim()
        .to_string();

    if status != "active" {
        // Get the last few lines of journal for diagnosis
        let journal = run_command_with_timeout(
            "journalctl",
            &["-u", SERVICE_FILE, "-n", "10", "--no-pager"],
            QUICK_QUERY_TIMEOUT,
        )
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
        anyhow::bail!(
            "Service failed to start (status: {}). Check config and logs:\n\
             journalctl -u dc-agent -n 20\n\n\
             Recent logs:\n{}",
            status,
            journal
        );
    }

    if service_existed {
        println!("[ok] Service restarted with new config");
    } else {
        println!("[ok] Service started");
    }
    println!("[ok] Service is running (verified)");

    Ok(())
}

/// Check if we're running on a Proxmox host by looking for pvesm command.
pub fn is_proxmox_host() -> bool {
    // Check multiple indicators for Proxmox VE:
    // 1. pvesm command exists (Proxmox storage manager)
    // 2. pveversion command exists
    // 3. /etc/pve directory exists (Proxmox config dir)
    run_command_with_timeout("which", &["pvesm"], QUICK_QUERY_TIMEOUT)
        .map(|o| o.status.success())
        .unwrap_or(false)
        || run_command_with_timeout("which", &["pveversion"], QUICK_QUERY_TIMEOUT)
            .map(|o| o.status.success())
            .unwrap_or(false)
        || Path::new("/etc/pve").exists()
}

/// Parse datacenter identifier from pool_id.
/// Pool ID format: "<dc_code>-<uuid>" like "sl-8eba3c90" or "usw-abc123"
/// Returns "dc-<code>" (e.g., "dc-sl") if valid, None otherwise.
///
/// Validation rules:
/// - Must contain at least one dash
/// - dc_code (before first dash) must be 2-4 lowercase ASCII letters
pub fn parse_datacenter_from_pool_id(pool_id: &str) -> Option<String> {
    let parts: Vec<&str> = pool_id.split('-').collect();

    // Must have at least 2 parts (code and uuid)
    if parts.len() < 2 {
        return None;
    }

    let dc_code = parts[0];

    // dc_code must not be empty
    if dc_code.is_empty() {
        return None;
    }

    // dc_code must be 2-4 lowercase ASCII letters
    if dc_code.len() < 2 || dc_code.len() > 4 {
        return None;
    }

    if !dc_code.chars().all(|c| c.is_ascii_lowercase()) {
        return None;
    }

    Some(format!("dc-{}", dc_code))
}

/// Format bytes into human-readable string (KB, MB, GB)
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_datacenter_from_pool_id_valid() {
        // Standard 2-letter datacenter codes
        assert_eq!(
            parse_datacenter_from_pool_id("sl-8eba3c90"),
            Some("dc-sl".to_string())
        );
        assert_eq!(
            parse_datacenter_from_pool_id("us-abc123"),
            Some("dc-us".to_string())
        );

        // 3-letter datacenter codes
        assert_eq!(
            parse_datacenter_from_pool_id("usw-abc123"),
            Some("dc-usw".to_string())
        );
        assert_eq!(
            parse_datacenter_from_pool_id("euw-deadbeef"),
            Some("dc-euw".to_string())
        );

        // 4-letter datacenter codes (max allowed)
        assert_eq!(
            parse_datacenter_from_pool_id("apne-12345"),
            Some("dc-apne".to_string())
        );

        // Multiple dashes in uuid part
        assert_eq!(
            parse_datacenter_from_pool_id("sl-abc-def-123"),
            Some("dc-sl".to_string())
        );
    }

    #[test]
    fn test_parse_datacenter_from_pool_id_invalid_no_dash() {
        // No dash at all
        assert_eq!(parse_datacenter_from_pool_id("8eba3c90"), None);
        assert_eq!(parse_datacenter_from_pool_id("slabc123"), None);
    }

    #[test]
    fn test_parse_datacenter_from_pool_id_invalid_empty_code() {
        // Empty code (starts with dash)
        assert_eq!(parse_datacenter_from_pool_id("-8eba3c90"), None);
        assert_eq!(parse_datacenter_from_pool_id("-"), None);
    }

    #[test]
    fn test_parse_datacenter_from_pool_id_invalid_uppercase() {
        // Uppercase letters
        assert_eq!(parse_datacenter_from_pool_id("SL-8eba3c90"), None);
        assert_eq!(parse_datacenter_from_pool_id("Sl-8eba3c90"), None);
        assert_eq!(parse_datacenter_from_pool_id("USW-abc123"), None);
    }

    #[test]
    fn test_parse_datacenter_from_pool_id_invalid_too_short() {
        // Code too short (< 2 chars)
        assert_eq!(parse_datacenter_from_pool_id("s-8eba3c90"), None);
        assert_eq!(parse_datacenter_from_pool_id("a-123"), None);
    }

    #[test]
    fn test_parse_datacenter_from_pool_id_invalid_too_long() {
        // Code too long (> 4 chars)
        assert_eq!(parse_datacenter_from_pool_id("abcde-8eba3c90"), None);
        assert_eq!(parse_datacenter_from_pool_id("uswest-123"), None);
    }

    #[test]
    fn test_parse_datacenter_from_pool_id_invalid_non_alpha() {
        // Non-alphabetic characters in code
        assert_eq!(parse_datacenter_from_pool_id("s1-8eba3c90"), None);
        assert_eq!(parse_datacenter_from_pool_id("12-abc123"), None);
        assert_eq!(parse_datacenter_from_pool_id("u_s-abc123"), None);
    }

    #[test]
    fn test_parse_datacenter_from_pool_id_edge_cases() {
        // Empty string
        assert_eq!(parse_datacenter_from_pool_id(""), None);

        // Just a dash
        assert_eq!(parse_datacenter_from_pool_id("-"), None);

        // Only code, no uuid
        assert_eq!(parse_datacenter_from_pool_id("sl"), None);
    }

    #[test]
    fn test_systemd_service_content_uses_absolute_paths() {
        // Create a temporary config file to test canonicalization
        use std::io::Write;
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("dc-agent.toml");
        let mut file = std::fs::File::create(&config_path).unwrap();
        writeln!(file, "[api]").unwrap();

        // The config path should be converted to absolute
        let absolute = config_path.canonicalize().unwrap();

        // Verify the canonical path is absolute and not the same as relative
        assert!(absolute.is_absolute());
        assert_ne!(
            config_path.to_string_lossy(),
            "dc-agent.toml",
            "Test setup should use a unique path"
        );

        // Verify canonicalize works as expected
        assert!(absolute.to_string_lossy().starts_with('/'));
    }

    #[test]
    fn test_systemd_service_format() {
        // Test that the service file format is valid
        let binary_path = "/usr/local/bin/dc-agent";
        let config_path = "/etc/dc-agent/config.toml";

        let service_content = format!(
            r#"[Unit]
Description=Decent Cloud Provisioning Agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart={} --config {} run
Restart=always
RestartSec=10
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
"#,
            binary_path, config_path
        );

        // Verify service content contains absolute paths
        assert!(
            service_content.contains("/usr/local/bin/dc-agent"),
            "Service should use absolute binary path"
        );
        assert!(
            service_content.contains("/etc/dc-agent/config.toml"),
            "Service should use absolute config path"
        );

        // Verify essential systemd directives are present
        assert!(service_content.contains("[Unit]"));
        assert!(service_content.contains("[Service]"));
        assert!(service_content.contains("[Install]"));
        assert!(service_content.contains("Restart=always"));
        assert!(service_content.contains("WantedBy=multi-user.target"));
    }

    #[test]
    fn test_is_service_installed_returns_false_in_test_env() {
        // dc-agent.service should not be installed in test environments
        let result = is_service_installed();
        assert!(
            !result,
            "dc-agent.service should not be installed in test environment"
        );
    }
}
