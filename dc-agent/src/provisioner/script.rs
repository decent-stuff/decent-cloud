use super::{HealthStatus, Instance, ProvisionRequest, Provisioner};
use crate::config::ScriptConfig;
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

pub struct ScriptProvisioner {
    config: ScriptConfig,
}

#[derive(Serialize)]
struct ScriptInput {
    action: String,
    #[serde(flatten)]
    request: Option<ProvisionRequest>,
    external_id: Option<String>,
}

#[derive(Deserialize)]
struct ScriptOutput {
    success: bool,
    instance: Option<Instance>,
    health: Option<HealthStatus>,
    error: Option<String>,
    #[allow(dead_code)]
    retry_possible: Option<bool>,
}

impl ScriptProvisioner {
    pub fn new(config: ScriptConfig) -> Self {
        Self { config }
    }

    async fn run_script(&self, script_path: &str, input: &ScriptInput) -> Result<ScriptOutput> {
        let input_json = serde_json::to_string(input).context("Failed to serialize input")?;

        let mut child = Command::new(script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn script: {}", script_path))?;

        let mut stdin = child
            .stdin
            .take()
            .context("Failed to open stdin for script")?;

        // Write input JSON to stdin
        stdin
            .write_all(input_json.as_bytes())
            .await
            .context("Failed to write to script stdin")?;
        drop(stdin); // Close stdin to signal EOF

        // Wait for script with timeout
        let timeout = tokio::time::Duration::from_secs(self.config.timeout_seconds);
        let output = tokio::time::timeout(timeout, child.wait_with_output())
            .await
            .context("Script execution timed out")?
            .with_context(|| format!("Failed to execute script: {}", script_path))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "Script {} failed with exit code {:?}: {}",
                script_path,
                output.status.code(),
                stderr
            );
        }

        let stdout =
            String::from_utf8(output.stdout).context("Script output is not valid UTF-8")?;

        let script_output: ScriptOutput = serde_json::from_str(&stdout)
            .with_context(|| format!("Failed to parse script output as JSON: {}", stdout))?;

        if !script_output.success {
            let error_msg = script_output
                .error
                .as_deref()
                .unwrap_or("Unknown error from script");
            bail!("Script reported failure: {}", error_msg);
        }

        Ok(script_output)
    }
}

#[async_trait]
impl Provisioner for ScriptProvisioner {
    async fn provision(&self, request: &ProvisionRequest) -> Result<Instance> {
        let input = ScriptInput {
            action: "provision".to_string(),
            request: Some(request.clone()),
            external_id: None,
        };

        let output = self
            .run_script(&self.config.provision, &input)
            .await
            .context("Provision script failed")?;

        output
            .instance
            .context("Script succeeded but did not return instance details")
    }

    async fn terminate(&self, external_id: &str) -> Result<()> {
        let input = ScriptInput {
            action: "terminate".to_string(),
            request: None,
            external_id: Some(external_id.to_string()),
        };

        self.run_script(&self.config.terminate, &input)
            .await
            .context("Terminate script failed")?;

        Ok(())
    }

    async fn health_check(&self, external_id: &str) -> Result<HealthStatus> {
        let input = ScriptInput {
            action: "health_check".to_string(),
            request: None,
            external_id: Some(external_id.to_string()),
        };

        let output = self
            .run_script(&self.config.health_check, &input)
            .await
            .context("Health check script failed")?;

        output
            .health
            .context("Script succeeded but did not return health status")
    }

    async fn get_instance(&self, external_id: &str) -> Result<Option<Instance>> {
        let input = ScriptInput {
            action: "get_instance".to_string(),
            request: None,
            external_id: Some(external_id.to_string()),
        };

        let output = self
            .run_script(&self.config.provision, &input)
            .await
            .context("Get instance script failed")?;

        Ok(output.instance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_output_parse_success() {
        let json = r#"{
            "success": true,
            "instance": {
                "external_id": "vm-123",
                "ip_address": "10.0.0.100",
                "ipv6_address": null,
                "ssh_port": 22,
                "root_password": "test123",
                "additional_details": null
            }
        }"#;

        let output: ScriptOutput = serde_json::from_str(json).unwrap();
        assert!(output.success);
        assert!(output.instance.is_some());

        let instance = output.instance.unwrap();
        assert_eq!(instance.external_id, "vm-123");
        assert_eq!(instance.ip_address, Some("10.0.0.100".to_string()));
        assert_eq!(instance.ssh_port, 22);
    }

    #[test]
    fn test_script_output_parse_error() {
        let json = r#"{
            "success": false,
            "error": "Out of storage space",
            "retry_possible": true
        }"#;

        let output: ScriptOutput = serde_json::from_str(json).unwrap();
        assert!(!output.success);
        assert_eq!(output.error, Some("Out of storage space".to_string()));
        assert_eq!(output.retry_possible, Some(true));
        assert!(output.instance.is_none());
    }

    #[test]
    fn test_script_output_parse_health_healthy() {
        let json = r#"{
            "success": true,
            "health": {
                "status": "healthy",
                "uptime_seconds": 3600
            }
        }"#;

        let output: ScriptOutput = serde_json::from_str(json).unwrap();
        assert!(output.success);
        assert!(output.health.is_some());

        match output.health.unwrap() {
            HealthStatus::Healthy { uptime_seconds } => {
                assert_eq!(uptime_seconds, 3600);
            }
            _ => panic!("Expected Healthy status"),
        }
    }

    #[test]
    fn test_script_output_parse_health_unhealthy() {
        let json = r#"{
            "success": true,
            "health": {
                "status": "unhealthy",
                "reason": "Network unreachable"
            }
        }"#;

        let output: ScriptOutput = serde_json::from_str(json).unwrap();
        assert!(output.success);
        assert!(output.health.is_some());

        match output.health.unwrap() {
            HealthStatus::Unhealthy { reason } => {
                assert_eq!(reason, "Network unreachable");
            }
            _ => panic!("Expected Unhealthy status"),
        }
    }

    #[test]
    fn test_script_output_parse_health_unknown() {
        let json = r#"{
            "success": true,
            "health": {
                "status": "unknown"
            }
        }"#;

        let output: ScriptOutput = serde_json::from_str(json).unwrap();
        assert!(output.success);
        assert!(output.health.is_some());

        match output.health.unwrap() {
            HealthStatus::Unknown => {}
            _ => panic!("Expected Unknown status"),
        }
    }

    #[test]
    fn test_script_input_serialize_provision() {
        let request = ProvisionRequest {
            contract_id: "abc123".to_string(),
            offering_id: "off-123".to_string(),
            cpu_cores: Some(2),
            memory_mb: Some(4096),
            storage_gb: Some(50),
            requester_ssh_pubkey: Some("ssh-ed25519 AAAA...".to_string()),
            instance_config: None,
            post_provision_script: None,
        };

        let input = ScriptInput {
            action: "provision".to_string(),
            request: Some(request),
            external_id: None,
        };

        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"action\":\"provision\""));
        assert!(json.contains("\"contract_id\":\"abc123\""));
        assert!(json.contains("\"cpu_cores\":2"));
    }

    #[test]
    fn test_script_input_serialize_terminate() {
        let input = ScriptInput {
            action: "terminate".to_string(),
            request: None,
            external_id: Some("vm-123".to_string()),
        };

        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"action\":\"terminate\""));
        assert!(json.contains("\"external_id\":\"vm-123\""));
        assert!(!json.contains("request"));
    }
}
