use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Chatwoot API client for managing agents, contacts, and conversations.
pub struct ChatwootClient {
    client: Client,
    base_url: String,
    api_token: String,
    account_id: u32,
}

impl std::fmt::Debug for ChatwootClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatwootClient")
            .field("base_url", &self.base_url)
            .field("account_id", &self.account_id)
            .finish()
    }
}

#[derive(Debug, Serialize)]
struct CreateAgentRequest<'a> {
    email: &'a str,
    name: &'a str,
    role: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct AgentResponse {
    pub id: u64,
    pub email: String,
}

#[derive(Debug, Serialize)]
struct CreateContactRequest<'a> {
    inbox_id: u32,
    identifier: &'a str,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
pub struct ContactResponse {
    pub id: u64,
    pub identifier: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateConversationRequest<'a> {
    inbox_id: u32,
    contact_id: u64,
    custom_attributes: ConversationAttributes<'a>,
}

#[derive(Debug, Serialize)]
struct ConversationAttributes<'a> {
    contract_id: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct ConversationResponse {
    pub id: u64,
}

impl ChatwootClient {
    /// Creates a new Chatwoot client from environment variables.
    pub fn from_env() -> Result<Self> {
        let base_url = std::env::var("CHATWOOT_BASE_URL").context("CHATWOOT_BASE_URL not set")?;
        let api_token =
            std::env::var("CHATWOOT_API_TOKEN").context("CHATWOOT_API_TOKEN not set")?;
        let account_id: u32 = std::env::var("CHATWOOT_ACCOUNT_ID")
            .context("CHATWOOT_ACCOUNT_ID not set")?
            .parse()
            .context("CHATWOOT_ACCOUNT_ID must be a number")?;

        Ok(Self {
            client: Client::new(),
            base_url,
            api_token,
            account_id,
        })
    }

    /// Creates a new Chatwoot client with explicit configuration.
    #[cfg(test)]
    pub fn new(base_url: &str, api_token: &str, account_id: u32) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
            api_token: api_token.to_string(),
            account_id,
        }
    }

    /// Create an agent account for a provider.
    pub async fn create_agent(&self, email: &str, name: &str) -> Result<AgentResponse> {
        let url = format!(
            "{}/api/v1/accounts/{}/agents",
            self.base_url, self.account_id
        );

        let resp = self
            .client
            .post(&url)
            .header("api_access_token", &self.api_token)
            .json(&CreateAgentRequest {
                email,
                name,
                role: "agent",
            })
            .send()
            .await
            .context("Failed to send create agent request")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Chatwoot API error {}: {}", status, body);
        }

        resp.json().await.context("Failed to parse agent response")
    }

    /// Create or update a contact (customer).
    pub async fn create_contact(
        &self,
        inbox_id: u32,
        identifier: &str,
        name: &str,
        email: Option<&str>,
    ) -> Result<ContactResponse> {
        let url = format!(
            "{}/api/v1/accounts/{}/contacts",
            self.base_url, self.account_id
        );

        let resp = self
            .client
            .post(&url)
            .header("api_access_token", &self.api_token)
            .json(&CreateContactRequest {
                inbox_id,
                identifier,
                name,
                email,
            })
            .send()
            .await
            .context("Failed to send create contact request")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Chatwoot API error {}: {}", status, body);
        }

        resp.json()
            .await
            .context("Failed to parse contact response")
    }

    /// Create a conversation for a contract.
    pub async fn create_conversation(
        &self,
        inbox_id: u32,
        contact_id: u64,
        contract_id: &str,
    ) -> Result<ConversationResponse> {
        let url = format!(
            "{}/api/v1/accounts/{}/conversations",
            self.base_url, self.account_id
        );

        let resp = self
            .client
            .post(&url)
            .header("api_access_token", &self.api_token)
            .json(&CreateConversationRequest {
                inbox_id,
                contact_id,
                custom_attributes: ConversationAttributes { contract_id },
            })
            .send()
            .await
            .context("Failed to send create conversation request")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Chatwoot API error {}: {}", status, body);
        }

        resp.json()
            .await
            .context("Failed to parse conversation response")
    }
}
