use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

// =============================================================================
// Platform API Client (for user management with password control)
// =============================================================================

/// Chatwoot Platform API client for user management.
/// Uses Platform App token from SuperAdmin console.
pub struct ChatwootPlatformClient {
    client: Client,
    base_url: String,
    platform_token: String,
    account_id: u32,
}

impl std::fmt::Debug for ChatwootPlatformClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatwootPlatformClient")
            .field("base_url", &self.base_url)
            .field("account_id", &self.account_id)
            .finish()
    }
}

#[derive(Debug, Serialize)]
struct CreatePlatformUserRequest<'a> {
    name: &'a str,
    email: &'a str,
    password: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct PlatformUserResponse {
    pub id: i64,
    pub email: String,
}

#[derive(Debug, Serialize)]
struct AddAccountUserRequest {
    user_id: i64,
    role: String,
}

#[derive(Debug, Serialize)]
struct UpdateUserPasswordRequest<'a> {
    password: &'a str,
}

impl ChatwootPlatformClient {
    /// Creates a new Platform client from environment variables.
    pub fn from_env() -> Result<Self> {
        let base_url = std::env::var("CHATWOOT_BASE_URL").context("CHATWOOT_BASE_URL not set")?;
        let platform_token = std::env::var("CHATWOOT_PLATFORM_API_TOKEN")
            .context("CHATWOOT_PLATFORM_API_TOKEN not set")?;
        let account_id: u32 = std::env::var("CHATWOOT_ACCOUNT_ID")
            .context("CHATWOOT_ACCOUNT_ID not set")?
            .parse()
            .context("CHATWOOT_ACCOUNT_ID must be a number")?;

        Ok(Self {
            client: Client::new(),
            base_url,
            platform_token,
            account_id,
        })
    }

    /// Check if Platform API is configured.
    pub fn is_configured() -> bool {
        std::env::var("CHATWOOT_PLATFORM_API_TOKEN").is_ok()
            && std::env::var("CHATWOOT_BASE_URL").is_ok()
            && std::env::var("CHATWOOT_ACCOUNT_ID").is_ok()
    }

    /// Create or find a user via Platform API.
    /// Returns the user ID which should be stored for future password resets.
    pub async fn create_user(
        &self,
        email: &str,
        name: &str,
        password: &str,
    ) -> Result<PlatformUserResponse> {
        let url = format!("{}/platform/api/v1/users", self.base_url);

        let resp = self
            .client
            .post(&url)
            .header("api_access_token", &self.platform_token)
            .json(&CreatePlatformUserRequest {
                name,
                email,
                password,
            })
            .send()
            .await
            .context("Failed to send create user request")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Chatwoot Platform API error {}: {}", status, body);
        }

        resp.json()
            .await
            .context("Failed to parse platform user response")
    }

    /// Add a user to an account as an agent.
    pub async fn add_user_to_account(&self, user_id: i64) -> Result<()> {
        let url = format!(
            "{}/platform/api/v1/accounts/{}/account_users",
            self.base_url, self.account_id
        );

        let resp = self
            .client
            .post(&url)
            .header("api_access_token", &self.platform_token)
            .json(&AddAccountUserRequest {
                user_id,
                role: "agent".to_string(),
            })
            .send()
            .await
            .context("Failed to send add user to account request")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Chatwoot Platform API error {}: {}", status, body);
        }

        Ok(())
    }

    /// Update a user's password.
    pub async fn update_user_password(&self, user_id: i64, new_password: &str) -> Result<()> {
        let url = format!("{}/platform/api/v1/users/{}", self.base_url, user_id);

        let resp = self
            .client
            .patch(&url)
            .header("api_access_token", &self.platform_token)
            .json(&UpdateUserPasswordRequest {
                password: new_password,
            })
            .send()
            .await
            .context("Failed to send update password request")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Chatwoot Platform API error {}: {}", status, body);
        }

        Ok(())
    }
}

// =============================================================================
// Account API Client (for contacts and conversations)
// =============================================================================

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

#[derive(Debug, Deserialize)]
struct ListHelpCenterArticlesResponse {
    payload: Vec<HelpCenterArticle>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HelpCenterArticle {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub slug: String,
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

    /// Fetch help center articles for a portal.
    pub async fn fetch_help_center_articles(
        &self,
        portal_slug: &str,
    ) -> Result<Vec<HelpCenterArticle>> {
        let url = format!("{}/hc/{}/en/articles", self.base_url, portal_slug);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send fetch articles request")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Chatwoot Help Center API error {}: {}", status, body);
        }

        let response: ListHelpCenterArticlesResponse = resp
            .json()
            .await
            .context("Failed to parse help center articles response")?;

        Ok(response.payload)
    }

    /// Send a message to a conversation.
    pub async fn send_message(&self, conversation_id: u64, content: &str) -> Result<()> {
        let url = format!(
            "{}/api/v1/accounts/{}/conversations/{}/messages",
            self.base_url, self.account_id, conversation_id
        );

        #[derive(Serialize)]
        struct SendMessageRequest<'a> {
            content: &'a str,
            message_type: &'a str,
            private: bool,
        }

        let resp = self
            .client
            .post(&url)
            .header("api_access_token", &self.api_token)
            .json(&SendMessageRequest {
                content,
                message_type: "outgoing",
                private: false,
            })
            .send()
            .await
            .context("Failed to send message request")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Chatwoot API error {}: {}", status, body);
        }

        Ok(())
    }

    /// Update conversation status.
    pub async fn update_conversation_status(
        &self,
        conversation_id: u64,
        status: &str,
    ) -> Result<()> {
        let url = format!(
            "{}/api/v1/accounts/{}/conversations/{}",
            self.base_url, self.account_id, conversation_id
        );

        #[derive(Serialize)]
        struct UpdateConversationRequest<'a> {
            status: &'a str,
        }

        let resp = self
            .client
            .patch(&url)
            .header("api_access_token", &self.api_token)
            .json(&UpdateConversationRequest { status })
            .send()
            .await
            .context("Failed to update conversation status request")?;

        if !resp.status().is_success() {
            let status_code = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Chatwoot API error {}: {}", status_code, body);
        }

        Ok(())
    }
}
