use super::{HealthStatus, Instance, ProvisionRequest, Provisioner};
use crate::config::ManualConfig;
use anyhow::Result;
use async_trait::async_trait;

/// Manual provisioner - logs requests but requires human intervention
pub struct ManualProvisioner {
    config: ManualConfig,
}

impl ManualProvisioner {
    pub fn new(config: ManualConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Provisioner for ManualProvisioner {
    async fn provision(&self, request: &ProvisionRequest) -> Result<Instance> {
        tracing::warn!(
            contract_id = %request.contract_id,
            offering_id = %request.offering_id,
            "Manual provisioning required - human intervention needed"
        );

        if let Some(webhook) = &self.config.notification_webhook {
            tracing::info!(webhook = %webhook, "Would send notification to webhook");
            // TODO: Implement webhook notification
        }

        anyhow::bail!(
            "Manual provisioning not implemented - requires human intervention for contract {}",
            request.contract_id
        )
    }

    async fn terminate(&self, external_id: &str) -> Result<()> {
        tracing::warn!(
            external_id = %external_id,
            "Manual termination required - human intervention needed"
        );

        if let Some(webhook) = &self.config.notification_webhook {
            tracing::info!(webhook = %webhook, "Would send termination notification");
            // TODO: Implement webhook notification
        }

        anyhow::bail!(
            "Manual termination not implemented - requires human intervention for instance {}",
            external_id
        )
    }

    async fn health_check(&self, _external_id: &str) -> Result<HealthStatus> {
        // Manual provisioner cannot automatically check health
        Ok(HealthStatus::Unknown)
    }

    async fn get_instance(&self, _external_id: &str) -> Result<Option<Instance>> {
        // Manual provisioner cannot automatically get instance details
        Ok(None)
    }
}
