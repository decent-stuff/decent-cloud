//! SSH script execution utilities.
//!
//! Executes scripts on remote VMs via SSH. Used by both dc-agent (post-provision)
//! and api-server (recipe provisioning).
//! Scripts can use any interpreter available on the VM via shebang (#!/bin/bash, #!/usr/bin/env python3, etc.).

use anyhow::{Context, Result};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Maximum time to wait for SSH to become available
const SSH_READY_TIMEOUT: Duration = Duration::from_secs(60);
/// Time between SSH connection attempts
const SSH_RETRY_INTERVAL: Duration = Duration::from_secs(5);
/// Maximum script execution time
const SCRIPT_TIMEOUT: Duration = Duration::from_secs(300);
/// Maximum password reset execution time
const PASSWORD_RESET_TIMEOUT: Duration = Duration::from_secs(30);

/// Execute a post-provision script on a remote VM via SSH.
///
/// The script should include a shebang line to specify the interpreter.
/// If no shebang is present, /bin/sh is used as the default.
///
/// # Arguments
/// * `ip_address` - IP address of the VM
/// * `ssh_port` - SSH port (usually 22)
/// * `script` - The script content to execute
/// * `context_id` - For logging purposes (contract ID or resource ID)
///
/// # Returns
/// * `Ok(())` if script executed successfully (exit code 0)
/// * `Err(_)` if script failed or couldn't be executed
pub async fn execute_post_provision_script(
    ip_address: &str,
    ssh_port: u16,
    script: &str,
    context_id: &str,
) -> Result<()> {
    info!(
        context_id = %context_id,
        ip_address = %ip_address,
        script_lines = script.lines().count(),
        "Executing post-provision script"
    );

    wait_for_ssh(ip_address, ssh_port, context_id).await?;
    execute_script_via_ssh(ip_address, ssh_port, script, context_id).await
}

/// Wait for SSH to become available on the VM.
async fn wait_for_ssh(ip_address: &str, ssh_port: u16, context_id: &str) -> Result<()> {
    let start = std::time::Instant::now();
    let mut attempt = 0;

    while start.elapsed() < SSH_READY_TIMEOUT {
        attempt += 1;
        debug!(
            context_id = %context_id,
            attempt = attempt,
            "Checking SSH availability"
        );

        let result = Command::new("ssh")
            .args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "UserKnownHostsFile=/dev/null",
                "-o",
                "ConnectTimeout=5",
                "-o",
                "BatchMode=yes",
                "-p",
                &ssh_port.to_string(),
                &format!("root@{}", ip_address),
                "true",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        match result {
            Ok(status) if status.success() => {
                info!(
                    context_id = %context_id,
                    elapsed_secs = start.elapsed().as_secs(),
                    "SSH is ready"
                );
                return Ok(());
            }
            Ok(_) => {
                debug!(
                    context_id = %context_id,
                    "SSH not ready yet, waiting..."
                );
            }
            Err(e) => {
                warn!(
                    context_id = %context_id,
                    error = ?e,
                    "SSH check failed"
                );
            }
        }

        tokio::time::sleep(SSH_RETRY_INTERVAL).await;
    }

    anyhow::bail!(
        "SSH not available after {} seconds",
        SSH_READY_TIMEOUT.as_secs()
    )
}

/// Ensure script has a shebang, defaulting to /bin/sh.
fn ensure_shebang(script: &str) -> String {
    if script.starts_with("#!") {
        script.to_string()
    } else {
        format!("#!/bin/sh\n{}", script)
    }
}

/// Execute a script on the remote VM via SSH.
async fn execute_script_via_ssh(
    ip_address: &str,
    ssh_port: u16,
    script: &str,
    context_id: &str,
) -> Result<()> {
    let script_with_shebang = ensure_shebang(script);

    // Create the remote command that:
    // 1. Writes the script to a temp file
    // 2. Makes it executable
    // 3. Runs it
    // 4. Cleans up
    let remote_script = format!(
        r#"
set -e
SCRIPT_FILE=$(mktemp /tmp/dc-provision-XXXXXX)
cat > "$SCRIPT_FILE" << 'DC_SCRIPT_EOF'
{}
DC_SCRIPT_EOF
chmod +x "$SCRIPT_FILE"
"$SCRIPT_FILE"
EXIT_CODE=$?
rm -f "$SCRIPT_FILE"
exit $EXIT_CODE
"#,
        script_with_shebang
    );

    debug!(
        context_id = %context_id,
        "Running post-provision script via SSH"
    );

    let output = tokio::time::timeout(
        SCRIPT_TIMEOUT,
        Command::new("ssh")
            .args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "UserKnownHostsFile=/dev/null",
                "-o",
                "ConnectTimeout=10",
                "-o",
                "BatchMode=yes",
                "-p",
                &ssh_port.to_string(),
                &format!("root@{}", ip_address),
                &remote_script,
            ])
            .output(),
    )
    .await
    .context("Script execution timed out")?
    .context("Failed to execute SSH command")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        info!(
            context_id = %context_id,
            exit_code = 0,
            stdout_lines = stdout.lines().count(),
            "Post-provision script completed successfully"
        );
        if !stdout.is_empty() {
            debug!(context_id = %context_id, stdout = %stdout, "Script stdout");
        }
        Ok(())
    } else {
        let exit_code = output.status.code().unwrap_or(-1);
        warn!(
            context_id = %context_id,
            exit_code = exit_code,
            stderr = %stderr,
            stdout = %stdout,
            "Post-provision script failed"
        );
        anyhow::bail!("Script exited with code {}: {}", exit_code, stderr)
    }
}

/// Build a safe password reset command that uses chpasswd.
fn build_password_reset_command(new_password: &str) -> String {
    format!("echo 'root:{new_password}' | chpasswd")
}

/// Reset the root password on a remote VM via SSH.
///
/// # Arguments
/// * `ip_address` - IP address of the VM
/// * `ssh_port` - SSH port (usually 22)
/// * `ssh_user` - SSH user (e.g., "root" or "ubuntu")
/// * `use_sudo` - Whether to use sudo for the password command
/// * `new_password` - The new password to set
/// * `context_id` - For logging purposes
///
/// # Returns
/// * `Ok(())` if password was changed successfully
/// * `Err(_)` if password reset failed
pub async fn reset_password_via_ssh(
    ip_address: &str,
    ssh_port: u16,
    ssh_user: &str,
    use_sudo: bool,
    new_password: &str,
    context_id: &str,
) -> Result<()> {
    info!(
        context_id = %context_id,
        ip_address = %ip_address,
        ssh_user = %ssh_user,
        use_sudo = use_sudo,
        "Resetting root password via SSH"
    );

    let cmd = build_password_reset_command(new_password);
    let full_cmd = if use_sudo {
        format!("sudo sh -c \"{}\"", cmd.replace('"', "\\\""))
    } else {
        cmd
    };

    let output = tokio::time::timeout(
        PASSWORD_RESET_TIMEOUT,
        Command::new("ssh")
            .args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "UserKnownHostsFile=/dev/null",
                "-o",
                "ConnectTimeout=10",
                "-o",
                "BatchMode=yes",
                "-p",
                &ssh_port.to_string(),
                &format!("{}@{}", ssh_user, ip_address),
                &full_cmd,
            ])
            .output(),
    )
    .await
    .context("Password reset timed out")?
    .context("Failed to execute SSH command")?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        info!(
            context_id = %context_id,
            "Password reset completed successfully"
        );
        Ok(())
    } else {
        let exit_code = output.status.code().unwrap_or(-1);
        warn!(
            context_id = %context_id,
            exit_code = exit_code,
            stderr = %stderr,
            "Password reset failed"
        );
        anyhow::bail!("Password reset exited with code {}: {}", exit_code, stderr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_shebang_adds_default() {
        let result = ensure_shebang("echo hello");
        assert!(result.starts_with("#!/bin/sh\n"));
        assert!(result.contains("echo hello"));
    }

    #[test]
    fn test_ensure_shebang_preserves_existing() {
        let script = "#!/usr/bin/env python3\nprint('hello')";
        let result = ensure_shebang(script);
        assert_eq!(result, script);
    }

    #[test]
    fn test_password_reset_command_format() {
        let cmd = build_password_reset_command("NewSecurePass123!");
        assert_eq!(cmd, "echo 'root:NewSecurePass123!' | chpasswd");
    }

    #[test]
    fn test_password_reset_command_with_special_chars() {
        let cmd = build_password_reset_command("Pass'with\"quotes$and`backticks");
        assert!(cmd.contains("chpasswd"));
        assert!(cmd.contains("Pass'with\"quotes$and`backticks"));
    }
}
