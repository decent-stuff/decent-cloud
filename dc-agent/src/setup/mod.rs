//! Setup wizard for configuring dc-agent with various provisioners.

use anyhow::{Context, Result};
use std::process::Command;

pub mod gateway;
pub mod proxmox;

pub use gateway::{detect_public_ip, GatewaySetup};
pub use proxmox::ProxmoxSetup;

/// Output from local shell command execution.
pub struct CommandOutput {
    pub stdout: String,
    #[allow(dead_code)]
    pub stderr: String,
    pub exit_status: i32,
}

/// Execute a shell command locally and return output.
pub fn execute_command(cmd: &str) -> Result<CommandOutput> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .with_context(|| format!("Failed to execute command: {}", cmd))?;

    Ok(CommandOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_status: output.status.code().unwrap_or(-1),
    })
}
