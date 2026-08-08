//! Gateway subcommand: ssh/tcp/contract connectivity checks.
use crate::api_cli::{Identity, SignedClient};
use crate::Contract;
use anyhow::{Context, Result};
use clap::Subcommand;
#[derive(Subcommand)]
pub(crate) enum GatewayAction {
    /// Test SSH connectivity via gateway
    Ssh {
        /// Gateway hostname
        #[arg(long)]
        host: String,
        /// SSH port
        #[arg(long)]
        port: u16,
        /// Path to SSH identity file
        #[arg(long)]
        identity_file: String,
    },
    /// Test TCP port connectivity
    Tcp {
        /// Gateway hostname
        #[arg(long)]
        host: String,
        /// External port
        #[arg(long)]
        external_port: u16,
        /// Expected response (optional)
        #[arg(long)]
        expect_response: Option<String>,
    },
    /// Test all ports for a contract
    Contract {
        /// Contract ID (UUID)
        contract_id: String,
        /// Identity to use for signing
        #[arg(long)]
        identity: String,
    },
}
// =============================================================================
// Gateway handlers
// =============================================================================

/// Run an SSH connectivity check against `host:port` using `identity_file`,
/// bounded by an overall `timeout`.
///
/// `ssh -o ConnectTimeout=10` only bounds the TCP connect phase; this wrapper
/// wraps the whole ssh invocation in [`tokio::time::timeout`] so a stuck SSH
/// handshake / auth / banner-wait cannot hang the gateway check indefinitely.
/// On Elapsed the returned error names what timed out so the operator can see
/// the failing host and port.
async fn check_ssh_connectivity(
    host: &str,
    port: u16,
    identity_file: &str,
    timeout: std::time::Duration,
) -> Result<()> {
    println!("Testing SSH connectivity to {}:{}", host, port);

    let ssh = tokio::process::Command::new("ssh")
        .args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "ConnectTimeout=10",
            "-i",
            identity_file,
            "-p",
            &port.to_string(),
            &format!("root@{}", host),
            "echo",
            "SSH_CONNECTION_OK",
        ])
        .output();

    let output = tokio::time::timeout(timeout, ssh)
        .await
        .with_context(|| {
            format!(
                "SSH connectivity check to {}:{} timed out after {:?} \
                 (ssh -o ConnectTimeout=10 only bounds the TCP connect phase; \
                 the overall budget bounds the SSH handshake/auth/echo round-trip)",
                host, port, timeout
            )
        })??;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("SSH_CONNECTION_OK") {
            println!("SSH connection successful!");
        } else {
            println!("SSH connected but unexpected output: {}", stdout);
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("SSH connection failed: {}", stderr);
    }
    Ok(())
}

pub(crate) async fn handle_gateway_action(action: GatewayAction, api_url: &str) -> Result<()> {
    match action {
        GatewayAction::Ssh {
            host,
            port,
            identity_file,
        } => {
            // 20s overall budget. ssh -o ConnectTimeout=10 only bounds the TCP
            // connect phase; without an outer deadline a stuck SSH handshake,
            // auth, or banner wait would hang the gateway test indefinitely.
            check_ssh_connectivity(&host, port, &identity_file, std::time::Duration::from_secs(20))
                .await?;
        }
        GatewayAction::Tcp {
            host,
            external_port,
            expect_response,
        } => {
            println!("Testing TCP connectivity to {}:{}", host, external_port);

            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            use tokio::net::TcpStream;

            let addr = format!("{}:{}", host, external_port);
            let mut stream = TcpStream::connect(&addr)
                .await
                .with_context(|| format!("Failed to connect to {}", addr))?;

            println!("TCP connection established.");

            if let Some(expected) = expect_response {
                // Send a simple ping and wait for response
                stream.write_all(b"ping\n").await?;

                let mut buffer = [0u8; 1024];
                let n = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    stream.read(&mut buffer),
                )
                .await
                .context("Timeout waiting for response")??;

                let response = String::from_utf8_lossy(&buffer[..n]);
                if response.contains(&expected) {
                    println!("Expected response received: {}", response.trim());
                } else {
                    anyhow::bail!(
                        "Unexpected response: expected '{}', got '{}'",
                        expected,
                        response.trim()
                    );
                }
            } else {
                println!("TCP connectivity OK (no response check requested).");
            }
        }
        GatewayAction::Contract {
            contract_id,
            identity,
        } => {
            let id = Identity::load(&identity)?;
            let client = SignedClient::new(&id, api_url)?;

            let path = format!("/contracts/{}", contract_id);
            let contract: Contract = client.get_api(&path).await?;

            println!("Testing gateway connectivity for contract: {}", contract_id);

            let gateway_host = contract
                .gateway_subdomain
                .context("Contract has no gateway subdomain")?;

            if let Some(ssh_port) = contract.gateway_ssh_port {
                println!("\nTesting SSH on port {}...", ssh_port);
                // Just test TCP connectivity to SSH port
                use tokio::net::TcpStream;
                let addr = format!("{}:{}", gateway_host, ssh_port);
                match TcpStream::connect(&addr).await {
                    Ok(_) => println!("  SSH port {} is reachable", ssh_port),
                    Err(e) => println!("  SSH port {} not reachable: {}", ssh_port, e),
                }
            }

            if let (Some(start), Some(end)) = (
                contract.gateway_port_range_start,
                contract.gateway_port_range_end,
            ) {
                println!("\nTesting port range {}-{}...", start, end);
                use tokio::net::TcpStream;
                for port in start..=end.min(start + 5) {
                    // Test first 5 ports max
                    let addr = format!("{}:{}", gateway_host, port);
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(2),
                        TcpStream::connect(&addr),
                    )
                    .await
                    {
                        Ok(Ok(_)) => println!("  Port {} is reachable", port),
                        Ok(Err(e)) => println!("  Port {} not reachable: {}", port, e),
                        Err(_) => println!("  Port {} connection timeout", port),
                    }
                }
            }

            println!("\nGateway connectivity test complete.");
        }
    }
    Ok(())
}

#[cfg(test)]
mod gateway_tests {
    use super::*;

    /// `check_ssh_connectivity` must enforce its overall timeout when the SSH
    /// server accepts the TCP connection but never sends the SSH banner —
    /// exactly the stuck-peer case `ssh -o ConnectTimeout=10` does NOT cover
    /// (that flag only bounds the TCP connect phase). Uses a 500ms budget so
    /// the test stays fast; the production caller passes 20s.
    #[tokio::test]
    async fn check_ssh_connectivity_times_out_against_hung_server() {
        // Hung TCP server: accepts the connection but never writes an SSH
        // banner. ssh will clear TCP connect quickly then wait forever for the
        // banner — only the outer tokio::time::timeout can terminate it.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind hung-server listener");
        let port = listener.local_addr().expect("local_addr").port();
        let accepted_tx = tokio::sync::oneshot::channel::<()>();
        let mut accepted_rx = accepted_tx.1;
        tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.expect("accept");
            let _ = accepted_tx.0.send(());
            // Hold the task open forever — never write an SSH banner so ssh's
            // banner-wait hangs until the outer tokio::time::timeout fires.
            std::future::pending::<()>().await;
        });

        // A readable identity file ssh will reject ("invalid format") but still
        // proceed to attempt the connection — which is what we want to hang.
        let tmp_key = std::env::temp_dir().join(format!(
            "dc-api-cli-ssh-test-key-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&tmp_key, b"not-a-real-key").expect("write tmp key");

        let started = std::time::Instant::now();
        let result =
            check_ssh_connectivity("127.0.0.1", port, tmp_key.to_str().unwrap(), std::time::Duration::from_millis(500))
                .await;
        if let Err(e) = std::fs::remove_file(&tmp_key) {
            tracing::debug!(path = %tmp_key.display(), error = %e, "cleanup: failed to remove temp file");
        }

        let elapsed = started.elapsed();
        // Should return promptly after the 500ms budget — well under ssh's
        // ConnectTimeout=10s, proving the outer timeout fired.
        assert!(
            elapsed < std::time::Duration::from_secs(3),
            "expected outer timeout to fire within ~500ms; elapsed: {:?}",
            elapsed
        );
        let err = result.expect_err("hung ssh server must produce a timeout error");
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("timed out") && msg.contains(&port.to_string()),
            "expected timeout error naming host:port, got: {}",
            msg
        );
        let _ = accepted_rx.try_recv();
    }
}
