//! DC-Agent self-upgrade functionality.
//!
//! Provides secure binary upgrades with SHA256 verification, rollback capability,
//! and systemd service restart handling.

use anyhow::{bail, Context, Result};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

const GITHUB_REPO: &str = "decent-stuff/decent-cloud";
const BINARY_NAME: &str = "dc-agent-linux-amd64";
const LOCK_FILE: &str = "/var/run/dc-agent-upgrade.lock";

/// Best-effort temp-file removal that logs on failure instead of silently
/// dropping the error via `.ok()`. (ROB-009)
fn remove_temp_file(path: &Path) {
    if let Err(e) = fs::remove_file(path) {
        tracing::debug!(path = %path.display(), error = %e, "cleanup: failed to remove temp file");
    }
}
const UPGRADE_TIMEOUT_SECS: u64 = 120;
/// Bounded budget for `dc-agent --version` after downloading a candidate
/// binary. The check is local (no network) so 10s is plenty for even the
/// slowest cold-start; without it a corrupt or interactive binary would hang
/// the upgrade flow forever.
const VERIFY_VERSION_TIMEOUT: Duration = Duration::from_secs(10);
/// Bounded budget for `systemctl` invocations during upgrade (is-active,
/// restart). 30s matches the project-wide HTTP/shell timeout convention.
const SYSTEMCTL_TIMEOUT: Duration = Duration::from_secs(30);

/// Captured output from a child process run with [`run_command_with_timeout`].
#[derive(Debug)]
struct TimedCommandOutput {
    stdout: String,
    status: ExitStatus,
}

/// Run a pre-built [`Command`] to completion, killing it if it does not exit
/// before `timeout` elapses.
///
/// Mirrors the spawn / poll `try_wait` / SIGKILL-on-deadline pattern of
/// [`crate::setup::execute_command_with_timeout`] but accepts a `&mut Command`
/// directly so callers can pass specific binaries and arg lists (the freshly
/// downloaded upgrade binary, `systemctl restart dc-agent`, ...) without
/// routing through `sh -c`. stdout is captured; stderr is inherited so the
/// operator sees live diagnostics from `systemctl` and the verify binary.
fn run_command_with_timeout(cmd: &mut Command, timeout: Duration) -> Result<TimedCommandOutput> {
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to spawn command")?;

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait()? {
            Some(status) => break status,
            None => {
                if Instant::now() >= deadline {
                    // Best-effort cleanup; errors are logged (not silently
                    // ignored) via the shared helper, then we bail.
                    crate::setup::best_effort_kill_and_reap(&mut child);
                    bail!("Command timed out after {:?}", timeout);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    };

    // Drain captured stdout now that the child has exited; EOF is guaranteed
    // because the child's write end is closed.
    let mut stdout = String::new();
    if let Some(mut s) = child.stdout.take() {
        s.read_to_string(&mut stdout)
            .context("Failed to read command stdout")?;
    }

    Ok(TimedCommandOutput { stdout, status })
}

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
        .timeout(std::time::Duration::from_secs(UPGRADE_TIMEOUT_SECS))
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
        .timeout(std::time::Duration::from_secs(UPGRADE_TIMEOUT_SECS))
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
    let output = run_command_with_timeout(
        Command::new(binary_path).arg("--version"),
        VERIFY_VERSION_TIMEOUT,
    )
    .with_context(|| format!("Failed to execute {}", binary_path.display()))?;

    if !output.status.success() {
        bail!("Binary exited with non-zero status");
    }

    if !output.stdout.contains(expected) {
        bail!(
            "Version mismatch: expected {} but got {}",
            expected,
            output.stdout.trim()
        );
    }

    Ok(())
}

/// Check if dc-agent is running as a systemd service.
fn is_systemd_service() -> bool {
    // Check for systemd-specific environment variable
    std::env::var("INVOCATION_ID").is_ok()
        || run_command_with_timeout(
            Command::new("systemctl").args(["is-active", "dc-agent"]),
            SYSTEMCTL_TIMEOUT,
        )
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Restart the dc-agent systemd service.
fn restart_service() -> Result<()> {
    let status = run_command_with_timeout(
        Command::new("systemctl").args(["restart", "dc-agent"]),
        SYSTEMCTL_TIMEOUT,
    )
    .context("Failed to execute systemctl")?
    .status;

    if !status.success() {
        bail!("systemctl restart failed");
    }

    // Wait a moment for service to start
    std::thread::sleep(Duration::from_secs(2));

    // Verify service is running
    let check = run_command_with_timeout(
        Command::new("systemctl").args(["is-active", "dc-agent"]),
        SYSTEMCTL_TIMEOUT,
    )?;

    if check.stdout.trim() != "active" {
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
        bail!(
            "Another upgrade is in progress (lock file exists: {})",
            LOCK_FILE
        );
    }
    fs::write(LOCK_FILE, std::process::id().to_string()).context("Failed to create lock file")?;
    Ok(())
}

/// Release upgrade lock.
fn release_lock() {
    if let Err(e) = fs::remove_file(LOCK_FILE) {
        // Only warn if file exists but couldn't be removed
        if e.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!("Failed to remove upgrade lock file {}: {e}", LOCK_FILE);
        }
    }
}

/// Main upgrade function.
///
/// If `target_version` is provided, upgrades to that specific version instead of latest.
pub async fn run_upgrade(
    check_only: bool,
    skip_confirm: bool,
    force: bool,
    target_version: Option<&str>,
) -> Result<()> {
    println!("dc-agent upgrade");
    println!("================\n");

    let current_version = env!("CARGO_PKG_VERSION");
    println!("Current version: {}", current_version);

    let latest_version = if let Some(v) = target_version {
        // Use the explicitly requested version
        let v = v.trim().trim_start_matches('v').to_string();
        println!("Target version:  {} (requested via API)", v);
        println!();
        v
    } else {
        // Check GitHub for latest version
        print!("Checking for updates... ");
        io::stdout().flush()?;
        let v = check_latest_version().await?;
        println!("done");
        println!("Latest version:  {}", v);
        println!();
        v
    };

    // Compare versions
    let needs_upgrade = is_newer(current_version, &latest_version);

    if !needs_upgrade && !force {
        println!("✓ Already up to date!");
        return Ok(());
    }

    if !needs_upgrade && force {
        println!("Note: Already up to date, but --force specified");
    } else {
        println!(
            "Upgrade available: {} → {}",
            current_version, latest_version
        );
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
        remove_temp_file(&temp_binary);
        remove_temp_file(&temp_checksums);
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
    // Stage new binary in same directory for atomic rename
    let staged_path = install_path.with_extension("new");

    // Backup current binary
    println!("\nInstalling...");
    if install_path.exists() {
        fs::copy(&install_path, &backup_path).context("Failed to backup current binary")?;
        println!("  [ok] Backed up to {}", backup_path.display());
    }

    // Copy new binary to staging location (same filesystem as target)
    fs::copy(&temp_binary, &staged_path).context("Failed to stage new binary")?;

    // Set permissions on staged binary
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&staged_path, fs::Permissions::from_mode(0o755))?;
    }

    // Atomic rename replaces running binary (old inode stays valid for running process)
    if let Err(e) = fs::rename(&staged_path, &install_path) {
        remove_temp_file(&staged_path);
        bail!("Failed to install new binary: {:#}", e);
    }
    println!("  [ok] Installed to {}", install_path.display());

    // Clean up temp files
    remove_temp_file(&temp_binary);
    remove_temp_file(&temp_checksums);

    // Restart service if applicable
    if is_systemd_service() {
        println!("\nRestarting service...");
        if let Err(e) = restart_service() {
            // Rollback on restart failure
            println!("  [FAILED] Service restart failed: {:#}", e);
            println!("\nRolling back...");
            if backup_path.exists() {
                // Use rename for atomic replacement (avoids ETXTBSY if new binary is running)
                fs::rename(&backup_path, &install_path)?;
                println!("  [ok] Restored previous version");
                // Try to restart with old version
                if let Err(e2) = restart_service() {
                    bail!(
                        "Rollback complete but service still failed to start: {:#}\n\
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

    println!(
        "\n✓ Upgrade complete: {} → {}",
        current_version, latest_version
    );

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

    /// `run_command_with_timeout` must complete promptly when the command is
    /// fast. Mirrors `setup::tests::short_command_completes_under_generous_timeout`.
    #[test]
    fn run_command_with_timeout_captures_fast_output() {
        let out = run_command_with_timeout(
            Command::new("echo").arg("hello"),
            Duration::from_secs(5),
        )
        .expect("echo should succeed");
        assert_eq!(out.stdout, "hello\n");
        assert!(out.status.success());
    }

    /// `run_command_with_timeout` must SIGKILL a long-running child once the
    /// deadline elapses, returning an Err that names the timeout. Mirrors
    /// `setup::tests::timeout_kills_long_running_command`. This is the
    /// assertion that protects `verify_binary_version` and the three
    /// `systemctl` callsites from hanging the upgrade flow on a stuck child.
    #[test]
    fn run_command_with_timeout_kills_long_running_command() {
        let started = Instant::now();
        let err = run_command_with_timeout(
            Command::new("sleep").arg("30"),
            Duration::from_millis(150),
        )
        .expect_err("sleep 30 must time out");
        let elapsed = started.elapsed();
        assert!(
            elapsed < Duration::from_secs(2),
            "should return promptly after timeout; elapsed: {:?}",
            elapsed
        );
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("timed out"),
            "expected timeout error message, got: {}",
            msg
        );
    }

    /// `verify_binary_version` routes through `run_command_with_timeout` with
    /// the 10s `VERIFY_VERSION_TIMEOUT` budget. Exercised here against a tiny
    /// temp script that prints a known version string, so the assertion does
    /// not depend on a real dc-agent binary being present or on a particular
    /// coreutils variant (`/bin/echo --version` is intercepted by GNU
    /// coreutils; `/bin/sh --version` is rejected by dash).
    #[test]
    fn verify_binary_version_passes_on_matching_output() {
        use std::os::unix::fs::PermissionsExt;

        let script = std::env::temp_dir().join(format!(
            "dc-agent-verify-version-test-{}-{}.sh",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&script, "#!/bin/sh\necho dc-agent test payload v9.9.9\n")
            .expect("write temp script");
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
            .expect("chmod temp script");

        let result = verify_binary_version(&script, "v9.9.9");
        super::remove_temp_file(&script);
        result.expect("script output must contain the expected version tag");
    }

    /// Negative path: a version string that is absent from the binary's
    /// `--version` output must produce a meaningful error rather than silently
    /// succeeding.
    #[test]
    fn verify_binary_version_fails_on_mismatch() {
        use std::os::unix::fs::PermissionsExt;

        let script = std::env::temp_dir().join(format!(
            "dc-agent-verify-version-test-{}-{}.sh",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&script, "#!/bin/sh\necho dc-agent test payload v9.9.9\n")
            .expect("write temp script");
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
            .expect("chmod temp script");

        let err = verify_binary_version(&script, "9.9.9-never-exists");
        super::remove_temp_file(&script);
        let err = err.expect_err("mismatch must fail");
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("Version mismatch"),
            "expected mismatch error, got: {}",
            msg
        );
    }
}
