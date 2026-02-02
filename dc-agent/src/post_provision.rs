//! Post-provision script execution via SSH.
//!
//! Executes scripts on newly provisioned VMs after they become reachable.
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

/// Execute a post-provision script on a remote VM via SSH.
///
/// The script should include a shebang line to specify the interpreter.
/// If no shebang is present, /bin/sh is used as the default.
///
/// # Arguments
/// * `ip_address` - IP address of the VM
/// * `ssh_port` - SSH port (usually 22)
/// * `script` - The script content to execute
/// * `contract_id` - For logging purposes
///
/// # Returns
/// * `Ok(())` if script executed successfully (exit code 0)
/// * `Err(_)` if script failed or couldn't be executed
pub async fn execute_post_provision_script(
    ip_address: &str,
    ssh_port: u16,
    script: &str,
    contract_id: &str,
) -> Result<()> {
    info!(
        contract_id = %contract_id,
        ip_address = %ip_address,
        script_lines = script.lines().count(),
        "Executing post-provision script"
    );

    // Wait for SSH to become available
    wait_for_ssh(ip_address, ssh_port, contract_id).await?;

    // Execute the script
    execute_script_via_ssh(ip_address, ssh_port, script, contract_id).await
}

/// Wait for SSH to become available on the VM.
async fn wait_for_ssh(ip_address: &str, ssh_port: u16, contract_id: &str) -> Result<()> {
    let start = std::time::Instant::now();
    let mut attempt = 0;

    while start.elapsed() < SSH_READY_TIMEOUT {
        attempt += 1;
        debug!(
            contract_id = %contract_id,
            attempt = attempt,
            "Checking SSH availability"
        );

        // Use ssh with a quick connection test (-o ConnectTimeout=5)
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
                "true", // Just check if we can connect
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        match result {
            Ok(status) if status.success() => {
                info!(
                    contract_id = %contract_id,
                    elapsed_secs = start.elapsed().as_secs(),
                    "SSH is ready"
                );
                return Ok(());
            }
            Ok(_) => {
                debug!(
                    contract_id = %contract_id,
                    "SSH not ready yet, waiting..."
                );
            }
            Err(e) => {
                warn!(
                    contract_id = %contract_id,
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

/// Execute a script on the remote VM via SSH.
async fn execute_script_via_ssh(
    ip_address: &str,
    ssh_port: u16,
    script: &str,
    contract_id: &str,
) -> Result<()> {
    // Ensure script has a shebang, default to /bin/sh
    let script_with_shebang = if script.starts_with("#!") {
        script.to_string()
    } else {
        format!("#!/bin/sh\n{}", script)
    };

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
        contract_id = %contract_id,
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
            contract_id = %contract_id,
            exit_code = 0,
            stdout_lines = stdout.lines().count(),
            "Post-provision script completed successfully"
        );
        if !stdout.is_empty() {
            debug!(contract_id = %contract_id, stdout = %stdout, "Script stdout");
        }
        Ok(())
    } else {
        let exit_code = output.status.code().unwrap_or(-1);
        warn!(
            contract_id = %contract_id,
            exit_code = exit_code,
            stderr = %stderr,
            stdout = %stdout,
            "Post-provision script failed"
        );
        anyhow::bail!("Script exited with code {}: {}", exit_code, stderr)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_script_shebang_added() {
        let script = "echo hello";
        let with_shebang = if script.starts_with("#!") {
            script.to_string()
        } else {
            format!("#!/bin/sh\n{}", script)
        };
        assert!(with_shebang.starts_with("#!/bin/sh"));
    }

    #[test]
    fn test_script_shebang_preserved() {
        let script = "#!/usr/bin/env python3\nprint('hello')";
        let with_shebang = if script.starts_with("#!") {
            script.to_string()
        } else {
            format!("#!/bin/sh\n{}", script)
        };
        assert!(with_shebang.starts_with("#!/usr/bin/env python3"));
    }
}
