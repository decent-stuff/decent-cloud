use super::{HealthStatus, Instance, ProvisionRequest, Provisioner};
use crate::config::ManualConfig;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;

/// Manual provisioner - logs requests but requires human intervention
pub struct ManualProvisioner {
    config: ManualConfig,
    client: Client,
}

#[derive(Serialize)]
struct WebhookPayload<'a> {
    event: &'a str,
    contract_id: Option<&'a str>,
    external_id: Option<&'a str>,
    offering_id: Option<&'a str>,
    message: &'a str,
}

impl ManualProvisioner {
    pub fn new(config: ManualConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    async fn send_webhook(&self, payload: &WebhookPayload<'_>) -> Result<()> {
        let webhook_url = match &self.config.notification_webhook {
            Some(url) => url,
            None => return Ok(()),
        };

        self.client
            .post(webhook_url)
            .json(payload)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
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

        let message = format!(
            "Manual provisioning required for contract {}",
            request.contract_id
        );

        if let Err(e) = self
            .send_webhook(&WebhookPayload {
                event: "provision_required",
                contract_id: Some(&request.contract_id),
                external_id: None,
                offering_id: Some(&request.offering_id),
                message: &message,
            })
            .await
        {
            tracing::error!(
                contract_id = %request.contract_id,
                error = %e,
                "Webhook notification failed! Operator may not be alerted about this provisioning request."
            );
        }

        anyhow::bail!("{} - human intervention needed", message)
    }

    async fn terminate(&self, external_id: &str) -> Result<()> {
        tracing::warn!(
            external_id = %external_id,
            "Manual termination required - human intervention needed"
        );

        let message = format!("Manual termination required for instance {}", external_id);

        if let Err(e) = self
            .send_webhook(&WebhookPayload {
                event: "terminate_required",
                contract_id: None,
                external_id: Some(external_id),
                offering_id: None,
                message: &message,
            })
            .await
        {
            tracing::error!(
                external_id = %external_id,
                error = %e,
                "Webhook notification failed! Operator may not be alerted about this termination request."
            );
        }

        anyhow::bail!("{} - human intervention needed", message)
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
