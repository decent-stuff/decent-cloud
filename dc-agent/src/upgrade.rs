//! DC-Agent self-upgrade functionality.
//!
//! Provides secure binary upgrades with SHA256 verification, rollback capability,
//! and systemd service restart handling.

use anyhow::{bail, Context, Result};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

const GITHUB_REPO: &str = "decent-stuff/decent-cloud";
const BINARY_NAME: &str = "dc-agent-linux-amd64";
const LOCK_FILE: &str = "/var/run/dc-agent-upgrade.lock";

/// Parse a version string like "0.4.9" or "v0.4.9" into (major, minor, patch).
pub fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let v = v.trim().trim_start_matches('v');
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    Some((
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ))
}

/// Check if `latest` is newer than `current` using semver comparison.
pub fn is_newer(current: &str, latest: &str) -> bool {
    match (parse_version(current), parse_version(latest)) {
        (Some(c), Some(l)) => l > c,
        _ => false,
    }
}

/// Fetch the latest release version from GitHub API.
pub async fn check_latest_version() -> Result<String> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    let client = reqwest::Client::builder()
        .user_agent("dc-agent")
        .build()?;

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch latest release")?;

    if !response.status().is_success() {
        bail!(
            "GitHub API returned status {}: {}",
            response.status(),
            response.text().await.unwrap_or_default()
        );
    }

    let json: serde_json::Value = response.json().await?;
    let tag = json["tag_name"]
        .as_str()
        .context("Missing tag_name in release")?;

    // Strip leading 'v' if present
    Ok(tag.trim_start_matches('v').to_string())
}

/// Download a file from URL to the specified path.
async fn download_file(url: &str, dest: &Path) -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent("dc-agent")
        .build()?;

    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to download {}", url))?;

    if !response.status().is_success() {
        bail!("Download failed with status {}", response.status());
    }

    let bytes = response.bytes().await?;
    fs::write(dest, &bytes).with_context(|| format!("Failed to write to {}", dest.display()))?;

    Ok(())
}

/// Calculate SHA256 checksum of a file.
fn calculate_sha256(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};

    let data = fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let hash = Sha256::digest(&data);
    Ok(format!("{:x}", hash))
}

/// Parse SHA256SUMS file and extract checksum for the specified filename.
fn parse_checksum_file(content: &str, filename: &str) -> Option<String> {
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[1] == filename {
            return Some(parts[0].to_string());
        }
    }
    None
}

/// Verify that a binary runs and reports the expected version.
fn verify_binary_version(binary_path: &Path, expected: &str) -> Result<()> {
    let output = Command::new(binary_path)
        .arg("--version")
        .output()
        .with_context(|| format!("Failed to execute {}", binary_path.display()))?;

    if !output.status.success() {
        bail!("Binary exited with non-zero status");
    }

    let version_output = String::from_utf8_lossy(&output.stdout);
    if !version_output.contains(expected) {
        bail!(
            "Version mismatch: expected {} but got {}",
            expected,
            version_output.trim()
        );
    }

    Ok(())
}

/// Check if dc-agent is running as a systemd service.
fn is_systemd_service() -> bool {
    // Check for systemd-specific environment variable
    std::env::var("INVOCATION_ID").is_ok()
        || Command::new("systemctl")
            .args(["is-active", "dc-agent"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
}

/// Restart the dc-agent systemd service.
fn restart_service() -> Result<()> {
    let status = Command::new("systemctl")
        .args(["restart", "dc-agent"])
        .status()
        .context("Failed to execute systemctl")?;

    if !status.success() {
        bail!("systemctl restart failed");
    }

    // Wait a moment for service to start
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Verify service is running
    let check = Command::new("systemctl")
        .args(["is-active", "dc-agent"])
        .output()?;

    if String::from_utf8_lossy(&check.stdout).trim() != "active" {
        bail!("Service failed to start after restart");
    }

    Ok(())
}

/// Get the path to the currently running binary.
fn current_binary_path() -> Result<PathBuf> {
    std::env::current_exe().context("Failed to determine current binary path")
}

/// Acquire upgrade lock to prevent concurrent upgrades.
fn acquire_lock() -> Result<()> {
    if Path::new(LOCK_FILE).exists() {
        bail!("Another upgrade is in progress (lock file exists: {})", LOCK_FILE);
    }
    fs::write(LOCK_FILE, std::process::id().to_string())
        .context("Failed to create lock file")?;
    Ok(())
}

/// Release upgrade lock.
fn release_lock() {
    let _ = fs::remove_file(LOCK_FILE);
}

/// Main upgrade function.
pub async fn run_upgrade(check_only: bool, skip_confirm: bool, force: bool) -> Result<()> {
    println!("dc-agent upgrade");
    println!("================\n");

    let current_version = env!("CARGO_PKG_VERSION");
    println!("Current version: {}", current_version);

    // Check for latest version
    print!("Checking for updates... ");
    io::stdout().flush()?;

    let latest_version = check_latest_version().await?;
    println!("done");
    println!("Latest version:  {}", latest_version);
    println!();

    // Compare versions
    let needs_upgrade = is_newer(current_version, &latest_version);

    if !needs_upgrade && !force {
        println!("✓ Already up to date!");
        return Ok(());
    }

    if !needs_upgrade && force {
        println!("Note: Already up to date, but --force specified");
    } else {
        println!("Upgrade available: {} → {}", current_version, latest_version);
    }

    if check_only {
        println!("\nRun 'dc-agent upgrade' to install the update.");
        return Ok(());
    }

    // Confirm upgrade
    if !skip_confirm {
        print!("\nProceed with upgrade? [y/N] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Upgrade cancelled.");
            return Ok(());
        }
    }

    println!();

    // Acquire lock
    acquire_lock()?;

    // Ensure lock is released on exit
    let _lock_guard = scopeguard::guard((), |_| release_lock());

    // Download binary
    let download_url = format!(
        "https://github.com/{}/releases/download/v{}/{}",
        GITHUB_REPO, latest_version, BINARY_NAME
    );
    let checksums_url = format!(
        "https://github.com/{}/releases/download/v{}/SHA256SUMS",
        GITHUB_REPO, latest_version
    );

    let temp_binary = PathBuf::from(format!("/tmp/dc-agent-{}", latest_version));
    let temp_checksums = PathBuf::from("/tmp/dc-agent-SHA256SUMS");

    println!("Downloading dc-agent {}...", latest_version);
    download_file(&download_url, &temp_binary).await?;
    println!("  [ok] Downloaded to {}", temp_binary.display());

    // Download checksums
    println!("\nDownloading checksums...");
    download_file(&checksums_url, &temp_checksums).await?;

    // Verify checksum
    println!("\nVerifying checksum...");
    let checksums_content = fs::read_to_string(&temp_checksums)?;
    let expected_checksum = parse_checksum_file(&checksums_content, BINARY_NAME)
        .context("Checksum for dc-agent not found in SHA256SUMS")?;

    let actual_checksum = calculate_sha256(&temp_binary)?;

    if expected_checksum != actual_checksum {
        fs::remove_file(&temp_binary).ok();
        fs::remove_file(&temp_checksums).ok();
        bail!(
            "CHECKSUM VERIFICATION FAILED!\n\
             Expected: {}\n\
             Got:      {}\n\n\
             The downloaded binary may be corrupted or tampered with.\n\
             Upgrade aborted for security reasons.",
            expected_checksum,
            actual_checksum
        );
    }
    println!("  [ok] SHA256 checksum verified");

    // Make binary executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temp_binary, fs::Permissions::from_mode(0o755))?;
    }

    // Validate new binary
    println!("\nValidating new binary...");
    verify_binary_version(&temp_binary, &latest_version)?;
    println!("  [ok] Version check passed");

    // Get current binary path and backup path
    let install_path = current_binary_path()?;
    let backup_path = install_path.with_extension("previous");

    // Backup current binary
    println!("\nInstalling...");
    if install_path.exists() {
        fs::copy(&install_path, &backup_path)
            .context("Failed to backup current binary")?;
        println!("  [ok] Backed up to {}", backup_path.display());
    }

    // Install new binary
    if let Err(e) = fs::copy(&temp_binary, &install_path) {
        // Restore backup on failure
        if backup_path.exists() {
            fs::copy(&backup_path, &install_path).ok();
        }
        bail!("Failed to install new binary: {}", e);
    }
    println!("  [ok] Installed to {}", install_path.display());

    // Clean up temp files
    fs::remove_file(&temp_binary).ok();
    fs::remove_file(&temp_checksums).ok();

    // Restart service if applicable
    if is_systemd_service() {
        println!("\nRestarting service...");
        if let Err(e) = restart_service() {
            // Rollback on restart failure
            println!("  [FAILED] Service restart failed: {}", e);
            println!("\nRolling back...");
            if backup_path.exists() {
                fs::copy(&backup_path, &install_path)?;
                println!("  [ok] Restored previous version");
                // Try to restart with old version
                if let Err(e2) = restart_service() {
                    bail!(
                        "Rollback complete but service still failed to start: {}\n\
                         Manual intervention required: systemctl status dc-agent",
                        e2
                    );
                }
                println!("  [ok] Service restarted with previous version");
            }
            bail!("Upgrade failed, rolled back to previous version");
        }
        println!("  [ok] Service restarted");
    } else {
        println!("\nNote: Not running as systemd service.");
        println!("Please restart dc-agent manually to use the new version.");
    }

    println!("\n✓ Upgrade complete: {} → {}", current_version, latest_version);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("0.4.9"), Some((0, 4, 9)));
        assert_eq!(parse_version("v0.4.9"), Some((0, 4, 9)));
        assert_eq!(parse_version("1.0.0"), Some((1, 0, 0)));
        assert_eq!(parse_version("0.4.10"), Some((0, 4, 10)));
        assert_eq!(parse_version("invalid"), None);
        assert_eq!(parse_version("0.4"), None);
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn test_is_newer() {
        // Basic comparisons
        assert!(is_newer("0.4.9", "0.4.10"));
        assert!(is_newer("0.4.9", "0.5.0"));
        assert!(is_newer("0.4.9", "1.0.0"));

        // Not newer
        assert!(!is_newer("0.4.10", "0.4.9"));
        assert!(!is_newer("0.4.9", "0.4.9"));
        assert!(!is_newer("1.0.0", "0.9.9"));

        // Edge cases
        assert!(is_newer("0.9.9", "0.10.0"));
        assert!(is_newer("0.4.99", "0.5.0"));
    }

    #[test]
    fn test_parse_checksum_file() {
        let content = "\
abc123def456  dc-agent-linux-amd64
789xyz000111  decent-cloud-linux-amd64
fedcba654321  decent-cloud-darwin-arm64";

        assert_eq!(
            parse_checksum_file(content, "dc-agent-linux-amd64"),
            Some("abc123def456".to_string())
        );
        assert_eq!(
            parse_checksum_file(content, "decent-cloud-linux-amd64"),
            Some("789xyz000111".to_string())
        );
        assert_eq!(parse_checksum_file(content, "nonexistent"), None);
    }

    #[test]
    fn test_parse_checksum_file_with_asterisk() {
        // Some sha256sum implementations prefix binary files with '*'
        let content = "abc123def456 *dc-agent-linux-amd64";
        assert_eq!(
            parse_checksum_file(content, "*dc-agent-linux-amd64"),
            Some("abc123def456".to_string())
        );
    }
}
