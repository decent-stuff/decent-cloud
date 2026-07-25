//! Setup wizard for configuring dc-agent with various provisioners.

use anyhow::{bail, Context, Result};
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub mod gateway;
pub mod proxmox;

pub use gateway::{detect_public_ip, GatewaySetup};
pub use proxmox::ProxmoxSetup;

/// Default per-command timeout for shell/provisioning steps.
///
/// Generous enough for `apt install`, template downloads, and other slow
/// provisioning work, while still catching commands that would otherwise
/// block forever (e.g. interactive prompts run non-interactively, hung
/// network calls). Override per-call with [`execute_command_with_timeout`].
pub const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(300);

/// Output from local shell command execution.
#[derive(Debug)]
pub struct CommandOutput {
    pub stdout: String,
    #[allow(dead_code)]
    pub stderr: String,
    pub exit_status: i32,
}

/// Execute a shell command locally with the default timeout
/// ([`DEFAULT_COMMAND_TIMEOUT`]).
pub fn execute_command(cmd: &str) -> Result<CommandOutput> {
    execute_command_with_timeout(cmd, DEFAULT_COMMAND_TIMEOUT)
}

/// Execute a shell command locally with an explicit timeout.
///
/// Spawns `sh -c <cmd>`, polls the child, and `SIGKILL`s it if it does not
/// exit before `timeout` elapses — returning an `Err` so the caller surfaces
/// the failure instead of blocking forever. Existing setup steps (gateway,
/// proxmox) route through [`execute_command`]; new callers that need a
/// non-default budget should call this directly.
pub fn execute_command_with_timeout(cmd: &str, timeout: Duration) -> Result<CommandOutput> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to execute command: {}", cmd))?;

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait()? {
            Some(status) => break status,
            None => {
                if Instant::now() >= deadline {
                    // Best-effort cleanup; ignore errors (process may have just exited).
                    let _ = child.kill();
                    let _ = child.wait();
                    bail!("Command timed out after {:?}: {}", timeout, cmd);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    };

    // Drain captured pipes now that the child has exited; EOF is guaranteed
    // because the child's write ends are closed.
    let mut stdout = String::new();
    if let Some(mut s) = child.stdout.take() {
        s.read_to_string(&mut stdout)
            .with_context(|| format!("Failed to read stdout for command: {}", cmd))?;
    }
    let mut stderr = String::new();
    if let Some(mut s) = child.stderr.take() {
        s.read_to_string(&mut stderr)
            .with_context(|| format!("Failed to read stderr for command: {}", cmd))?;
    }

    Ok(CommandOutput {
        stdout,
        stderr,
        exit_status: status.code().unwrap_or(-1),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_stdout_and_exit_code() {
        let out = execute_command("echo hello").expect("echo should succeed");
        assert_eq!(out.stdout, "hello\n");
        assert_eq!(out.exit_status, 0);
    }

    #[test]
    fn propagates_nonzero_exit() {
        let out = execute_command("exit 7").expect("spawn should succeed");
        assert_eq!(out.exit_status, 7);
    }

    #[test]
    fn captures_stderr() {
        let out = execute_command("echo oops 1>&2").expect("spawn should succeed");
        assert_eq!(out.stderr, "oops\n");
        assert_eq!(out.stdout, "");
    }

    #[test]
    fn timeout_kills_long_running_command() {
        let started = Instant::now();
        let err = execute_command_with_timeout("sleep 30", Duration::from_millis(150))
            .expect_err("sleep 30 must time out");
        let elapsed = started.elapsed();
        // Should return promptly after the timeout, not wait for the full sleep.
        assert!(elapsed < Duration::from_secs(2), "elapsed: {:?}", elapsed);
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("timed out") && msg.contains("sleep 30"),
            "unexpected error message: {}",
            msg
        );
    }

    #[test]
    fn short_command_completes_under_generous_timeout() {
        let out = execute_command_with_timeout("sleep 0.05", Duration::from_secs(5))
            .expect("short sleep should complete");
        assert_eq!(out.exit_status, 0);
    }
}
