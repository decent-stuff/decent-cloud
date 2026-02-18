use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

// =============================================================================
// Platform API Client (admin operations)
// =============================================================================
// Uses CHATWOOT_PLATFORM_API_TOKEN from SuperAdmin → Applications → Platform App
// Required for:
//   - User management (create, update password)
//   - Agent bot management (create, update, assign to inbox)
//   - Inbox configuration
// The Account API (ChatwootClient below) only has agent-level permissions
// for sending messages, not for configuring bots or inboxes.
// =============================================================================

/// Chatwoot Platform API client for admin operations.
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
    #[allow(dead_code)]
    pub email: String,
}

#[derive(Debug, Serialize)]
struct AddAccountUserRequest {
    user_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_role_id: Option<i64>,
}

/// Custom role name for support agents with Help Center access.
const SUPPORT_AGENT_ROLE_NAME: &str = "Support Agent";

/// Permissions for the Support Agent custom role.
const SUPPORT_AGENT_PERMISSIONS: &[&str] = &[
    "conversation_manage",
    "contact_manage",
    "knowledge_base_manage",
];

#[derive(Debug, Serialize)]
struct UpdateUserPasswordRequest<'a> {
    password: &'a str,
}

impl ChatwootPlatformClient {
    #[cfg(test)]
    pub(crate) fn new_for_test(base_url: String, platform_token: String, account_id: u32) -> Self {
        Self {
            client: Client::new(),
            base_url,
            platform_token,
            account_id,
        }
    }

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
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!("Chatwoot Platform API error {}: {}", status, body);
        }

        resp.json()
            .await
            .context("Failed to parse platform user response")
    }

    /// Add a user to an account with the Support Agent custom role.
    /// The custom_role_id should be obtained from `ensure_support_agent_role()`.
    pub async fn add_user_to_account(&self, user_id: i64, custom_role_id: i64) -> Result<()> {
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
                role: None,
                custom_role_id: Some(custom_role_id),
            })
            .send()
            .await
            .context("Failed to send add user to account request")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!("Chatwoot Platform API error {}: {}", status, body);
        }

        Ok(())
    }

    /// Ensure the Support Agent custom role exists, creating it if needed.
    /// Returns the custom_role_id to use when adding users.
    /// This role has conversation_manage, contact_manage, and knowledge_base_manage permissions.
    pub async fn ensure_support_agent_role(&self, api_token: &str) -> Result<i64> {
        let url = format!(
            "{}/api/v1/accounts/{}/custom_roles",
            self.base_url, self.account_id
        );

        #[derive(Deserialize)]
        struct CustomRole {
            id: i64,
            name: String,
        }

        // List existing custom roles
        let resp = self
            .client
            .get(&url)
            .header("api_access_token", api_token)
            .send()
            .await
            .context("Failed to list custom roles")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!(
                "Chatwoot API error listing custom roles {}: {}",
                status,
                body
            );
        }

        let roles: Vec<CustomRole> = resp.json().await.context("Failed to parse custom roles")?;

        // Check if role already exists
        if let Some(existing) = roles.iter().find(|r| r.name == SUPPORT_AGENT_ROLE_NAME) {
            tracing::debug!("Support Agent role already exists with id={}", existing.id);
            return Ok(existing.id);
        }

        // Create the role
        #[derive(Serialize)]
        struct CreateCustomRoleRequest<'a> {
            name: &'a str,
            description: &'a str,
            permissions: &'a [&'a str],
        }

        let resp = self
            .client
            .post(&url)
            .header("api_access_token", api_token)
            .json(&CreateCustomRoleRequest {
                name: SUPPORT_AGENT_ROLE_NAME,
                description: "Support agent with Help Center access",
                permissions: SUPPORT_AGENT_PERMISSIONS,
            })
            .send()
            .await
            .context("Failed to create custom role")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!(
                "Chatwoot API error creating custom role {}: {}",
                status,
                body
            );
        }

        let created: CustomRole = resp.json().await.context("Failed to parse created role")?;
        tracing::info!(
            "Created Chatwoot custom role '{}' (id={})",
            SUPPORT_AGENT_ROLE_NAME,
            created.id
        );
        Ok(created.id)
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
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!("Chatwoot Platform API error {}: {}", status, body);
        }

        Ok(())
    }

    /// Configure an Agent Bot with the given webhook URL for this account.
    /// Uses Platform API: /platform/api/v1/agent_bots
    /// Creates the bot if it doesn't exist, or updates the outgoing_url if it does.
    pub async fn configure_agent_bot(&self, name: &str, webhook_url: &str) -> Result<i64> {
        let list_url = format!("{}/platform/api/v1/agent_bots", self.base_url);

        #[derive(Deserialize)]
        struct AgentBot {
            id: i64,
            name: String,
            account_id: Option<u32>,
        }

        let resp = self
            .client
            .get(&list_url)
            .header("api_access_token", &self.platform_token)
            .send()
            .await
            .context("Failed to list agent bots")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!("Platform API error listing agent bots {}: {}", status, body);
        }

        let bots: Vec<AgentBot> = resp.json().await.context("Failed to parse agent bots")?;

        // Check if bot already exists for this account
        if let Some(existing) = bots
            .iter()
            .find(|b| b.name == name && b.account_id == Some(self.account_id))
        {
            // Update existing bot
            let update_url = format!(
                "{}/platform/api/v1/agent_bots/{}",
                self.base_url, existing.id
            );

            #[derive(Serialize)]
            struct UpdateAgentBotRequest<'a> {
                outgoing_url: &'a str,
            }

            let resp = self
                .client
                .patch(&update_url)
                .header("api_access_token", &self.platform_token)
                .json(&UpdateAgentBotRequest {
                    outgoing_url: webhook_url,
                })
                .send()
                .await
                .context("Failed to update agent bot")?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp
                    .text()
                    .await
                    .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
                anyhow::bail!("Platform API error updating agent bot {}: {}", status, body);
            }

            tracing::info!(
                "Updated Chatwoot agent bot '{}' (id={}) with URL {}",
                name,
                existing.id,
                webhook_url
            );
            Ok(existing.id)
        } else {
            // Create new bot with account_id
            #[derive(Serialize)]
            struct CreateAgentBotRequest<'a> {
                name: &'a str,
                outgoing_url: &'a str,
                account_id: u32,
            }

            let resp = self
                .client
                .post(&list_url)
                .header("api_access_token", &self.platform_token)
                .json(&CreateAgentBotRequest {
                    name,
                    outgoing_url: webhook_url,
                    account_id: self.account_id,
                })
                .send()
                .await
                .context("Failed to create agent bot")?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp
                    .text()
                    .await
                    .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
                anyhow::bail!("Platform API error creating agent bot {}: {}", status, body);
            }

            let created: AgentBot = resp
                .json()
                .await
                .context("Failed to parse created agent bot")?;
            tracing::info!(
                "Created Chatwoot agent bot '{}' (id={}, account={}) with URL {}",
                name,
                created.id,
                self.account_id,
                webhook_url
            );
            Ok(created.id)
        }
    }
}

// =============================================================================
// Account API Client
// =============================================================================
// Uses CHATWOOT_API_TOKEN - an agent or admin user's API token
// Used for:
//   - Agent bot CRUD (create/update/delete via /api/v1/accounts/:id/agent_bots)
//   - Sending messages to conversations
//   - Creating contacts and conversations
//   - Fetching Help Center articles
// Note: Inbox assignment requires Platform API above.
// =============================================================================

/// Chatwoot Account API client (agent bots, messages, contacts).
pub struct ChatwootClient {
    client: Client,
    base_url: String,
    /// Public URL for Help Center API (requires registered domain)
    frontend_url: String,
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

/// Response from creating an inbox.
#[derive(Debug, Deserialize)]
pub struct InboxResponse {
    pub id: u32,
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub channel_type: String,
}

/// Response from creating a team.
#[derive(Debug, Deserialize)]
pub struct TeamResponse {
    pub id: u32,
    pub name: String,
}

/// Response from creating a portal.
#[derive(Debug, Deserialize)]
pub struct PortalResponse {
    #[allow(dead_code)]
    pub id: i64,
    pub name: String,
    pub slug: String,
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
    #[cfg(test)]
    pub(crate) fn new_for_test(base_url: String, api_token: String, account_id: u32) -> Self {
        Self {
            client: Client::new(),
            frontend_url: base_url.clone(),
            base_url,
            api_token,
            account_id,
        }
    }

    /// Creates a new Chatwoot client from environment variables.
    pub fn from_env() -> Result<Self> {
        let base_url = std::env::var("CHATWOOT_BASE_URL").context("CHATWOOT_BASE_URL not set")?;
        // Help Center API requires public domain (internal hostnames rejected)
        let frontend_url =
            std::env::var("CHATWOOT_FRONTEND_URL").unwrap_or_else(|_| base_url.clone());
        let api_token =
            std::env::var("CHATWOOT_API_TOKEN").context("CHATWOOT_API_TOKEN not set")?;
        let account_id: u32 = std::env::var("CHATWOOT_ACCOUNT_ID")
            .context("CHATWOOT_ACCOUNT_ID not set")?
            .parse()
            .context("CHATWOOT_ACCOUNT_ID must be a number")?;

        Ok(Self {
            client: Client::new(),
            base_url,
            frontend_url,
            api_token,
            account_id,
        })
    }

    /// List all inboxes in the account.
    pub async fn list_inboxes(&self) -> Result<Vec<u32>> {
        let url = format!(
            "{}/api/v1/accounts/{}/inboxes",
            self.base_url, self.account_id
        );

        #[derive(Deserialize)]
        struct InboxesResponse {
            payload: Vec<Inbox>,
        }

        #[derive(Deserialize)]
        struct Inbox {
            id: u32,
        }

        let resp = self
            .client
            .get(&url)
            .header("api_access_token", &self.api_token)
            .send()
            .await
            .context("Failed to list inboxes")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!("Chatwoot API error listing inboxes {}: {}", status, body);
        }

        let response: InboxesResponse = resp.json().await.context("Failed to parse inboxes")?;
        Ok(response.payload.into_iter().map(|i| i.id).collect())
    }

    /// Create an API channel inbox for a provider.
    /// Returns (inbox_id, webhook_url) for the provider to receive messages.
    pub async fn create_inbox(&self, name: &str) -> Result<InboxResponse> {
        let url = format!(
            "{}/api/v1/accounts/{}/inboxes",
            self.base_url, self.account_id
        );

        #[derive(Serialize)]
        struct CreateInboxRequest<'a> {
            name: &'a str,
            channel: ChannelConfig,
        }

        #[derive(Serialize)]
        struct ChannelConfig {
            r#type: &'static str,
        }

        let resp = self
            .client
            .post(&url)
            .header("api_access_token", &self.api_token)
            .json(&CreateInboxRequest {
                name,
                channel: ChannelConfig { r#type: "api" },
            })
            .send()
            .await
            .context("Failed to create inbox")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!("Chatwoot API error creating inbox {}: {}", status, body);
        }

        resp.json().await.context("Failed to parse inbox response")
    }

    /// Create a team for a provider.
    pub async fn create_team(&self, name: &str, description: &str) -> Result<TeamResponse> {
        let url = format!(
            "{}/api/v1/accounts/{}/teams",
            self.base_url, self.account_id
        );

        // Chatwoot expects nested `team` object
        #[derive(Serialize)]
        struct TeamData<'a> {
            name: &'a str,
            description: &'a str,
            allow_auto_assign: bool,
        }

        #[derive(Serialize)]
        struct CreateTeamRequest<'a> {
            team: TeamData<'a>,
        }

        let resp = self
            .client
            .post(&url)
            .header("api_access_token", &self.api_token)
            .json(&CreateTeamRequest {
                team: TeamData {
                    name,
                    description,
                    allow_auto_assign: true,
                },
            })
            .send()
            .await
            .context("Failed to create team")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!(
                "Chatwoot API error creating team (status {}): {}",
                status,
                body
            );
        }

        resp.json().await.context("Failed to parse team response")
    }

    /// Add agents to a team.
    pub async fn add_agents_to_team(&self, team_id: u32, user_ids: &[i64]) -> Result<()> {
        let url = format!(
            "{}/api/v1/accounts/{}/teams/{}/team_members",
            self.base_url, self.account_id, team_id
        );

        #[derive(Serialize)]
        struct AddAgentsRequest<'a> {
            user_ids: &'a [i64],
        }

        let resp = self
            .client
            .post(&url)
            .header("api_access_token", &self.api_token)
            .json(&AddAgentsRequest { user_ids })
            .send()
            .await
            .context("Failed to add agents to team")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!(
                "Chatwoot API error adding agents to team {}: {}",
                status,
                body
            );
        }

        Ok(())
    }

    /// Create a Help Center portal for a provider.
    /// Uses `frontend_url` as the custom domain for the portal.
    pub async fn create_portal(&self, name: &str, slug: &str) -> Result<PortalResponse> {
        let url = format!(
            "{}/api/v1/accounts/{}/portals",
            self.base_url, self.account_id
        );

        // Extract domain from frontend_url (e.g., "https://support.decent-cloud.org" -> "support.decent-cloud.org")
        // Chatwoot has a bug where it always calls URI.parse on custom_domain even when nil
        let custom_domain = self
            .frontend_url
            .strip_prefix("https://")
            .or_else(|| self.frontend_url.strip_prefix("http://"))
            .unwrap_or(&self.frontend_url);

        // Chatwoot expects nested `portal` object
        #[derive(Serialize)]
        struct PortalData<'a> {
            name: &'a str,
            slug: &'a str,
            custom_domain: &'a str,
        }

        #[derive(Serialize)]
        struct CreatePortalRequest<'a> {
            portal: PortalData<'a>,
        }

        let resp = self
            .client
            .post(&url)
            .header("api_access_token", &self.api_token)
            .json(&CreatePortalRequest {
                portal: PortalData {
                    name,
                    slug,
                    custom_domain,
                },
            })
            .send()
            .await
            .context("Failed to create portal")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!(
                "Chatwoot API error creating portal (status {}): {}",
                status,
                body
            );
        }

        resp.json().await.context("Failed to parse portal response")
    }

    /// List all Help Center portal slugs in the account.
    /// Excludes archived portals.
    pub async fn list_portals(&self) -> Result<Vec<String>> {
        let url = format!(
            "{}/api/v1/accounts/{}/portals",
            self.base_url, self.account_id
        );

        #[derive(Deserialize)]
        struct PortalsResponse {
            payload: Vec<Portal>,
        }

        #[derive(Deserialize)]
        struct Portal {
            slug: String,
            archived: bool,
        }

        let resp = self
            .client
            .get(&url)
            .header("api_access_token", &self.api_token)
            .send()
            .await
            .context("Failed to list portals")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!("Chatwoot API error listing portals {}: {}", status, body);
        }

        let response: PortalsResponse = resp.json().await.context("Failed to parse portals")?;
        Ok(response
            .payload
            .into_iter()
            .filter(|p| !p.archived)
            .map(|p| p.slug)
            .collect())
    }

    /// Find or create an API channel inbox for a provider (idempotent).
    /// Returns existing inbox if one with the same name exists.
    pub async fn find_or_create_inbox(&self, name: &str) -> Result<InboxResponse> {
        let url = format!(
            "{}/api/v1/accounts/{}/inboxes",
            self.base_url, self.account_id
        );

        // First, list existing inboxes to check for duplicates
        #[derive(Deserialize)]
        struct InboxesResponse {
            payload: Vec<InboxResponse>,
        }

        let resp = self
            .client
            .get(&url)
            .header("api_access_token", &self.api_token)
            .send()
            .await
            .context("Failed to list inboxes")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!("Chatwoot API error listing inboxes {}: {}", status, body);
        }

        let response: InboxesResponse = resp.json().await.context("Failed to parse inboxes")?;
        if let Some(existing) = response.payload.into_iter().find(|i| i.name == name) {
            tracing::debug!("Inbox '{}' already exists with id={}", name, existing.id);
            return Ok(existing);
        }

        // Create new inbox
        self.create_inbox(name).await
    }

    /// Find or create a team for a provider (idempotent).
    /// Returns existing team if one with the same name exists.
    pub async fn find_or_create_team(&self, name: &str, description: &str) -> Result<TeamResponse> {
        let url = format!(
            "{}/api/v1/accounts/{}/teams",
            self.base_url, self.account_id
        );

        // First, list existing teams to check for duplicates
        let resp = self
            .client
            .get(&url)
            .header("api_access_token", &self.api_token)
            .send()
            .await
            .context("Failed to list teams")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!("Chatwoot API error listing teams {}: {}", status, body);
        }

        let teams: Vec<TeamResponse> = resp.json().await.context("Failed to parse teams")?;

        // Check if team already exists (case-insensitive - Chatwoot lowercases names)
        let name_lower = name.to_lowercase();
        if let Some(existing) = teams
            .into_iter()
            .find(|t| t.name.to_lowercase() == name_lower)
        {
            tracing::debug!("Team '{}' already exists with id={}", name, existing.id);
            return Ok(existing);
        }

        // Create new team
        self.create_team(name, description).await
    }

    /// Find or create a Help Center portal for a provider (idempotent).
    /// Returns existing portal if one with the same slug exists.
    pub async fn find_or_create_portal(&self, name: &str, slug: &str) -> Result<PortalResponse> {
        let url = format!(
            "{}/api/v1/accounts/{}/portals",
            self.base_url, self.account_id
        );

        // First, list existing portals to check for duplicates
        #[derive(Deserialize)]
        struct PortalsResponse {
            payload: Vec<PortalListItem>,
        }

        #[derive(Deserialize)]
        struct PortalListItem {
            id: i64,
            name: String,
            slug: String,
            archived: bool,
        }

        let resp = self
            .client
            .get(&url)
            .header("api_access_token", &self.api_token)
            .send()
            .await
            .context("Failed to list portals")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!("Chatwoot API error listing portals {}: {}", status, body);
        }

        let response: PortalsResponse = resp.json().await.context("Failed to parse portals")?;

        // Check if portal already exists (by slug, not archived)
        if let Some(existing) = response
            .payload
            .into_iter()
            .filter(|p| !p.archived)
            .find(|p| p.slug == slug)
        {
            tracing::debug!("Portal '{}' already exists with id={}", slug, existing.id);
            return Ok(PortalResponse {
                id: existing.id,
                name: existing.name,
                slug: existing.slug,
            });
        }

        // Create new portal
        self.create_portal(name, slug).await
    }

    /// Assign an agent bot to an inbox via Account API.
    /// Uses POST /api/v1/accounts/:account_id/inboxes/:inbox_id/set_agent_bot
    pub async fn assign_agent_bot_to_inbox(&self, inbox_id: u32, agent_bot_id: i64) -> Result<()> {
        let url = format!(
            "{}/api/v1/accounts/{}/inboxes/{}/set_agent_bot",
            self.base_url, self.account_id, inbox_id
        );

        let resp = self
            .client
            .post(&url)
            .header("api_access_token", &self.api_token)
            .json(&serde_json::json!({ "agent_bot": agent_bot_id }))
            .send()
            .await
            .context("Failed to assign agent bot to inbox")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!(
                "Chatwoot API error assigning agent bot to inbox {}: {}",
                status,
                body
            );
        }

        tracing::info!(
            "Assigned agent bot {} to inbox {} (account {})",
            agent_bot_id,
            inbox_id,
            self.account_id
        );
        Ok(())
    }

    /// Fetch help center articles for a portal.
    pub async fn fetch_help_center_articles(
        &self,
        portal_slug: &str,
    ) -> Result<Vec<HelpCenterArticle>> {
        // Use frontend_url - Help Center API rejects internal hostnames
        let url = format!("{}/hc/{}/en/articles.json", self.frontend_url, portal_slug);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send fetch articles request")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!("Chatwoot Help Center API error {}: {}", status, body);
        }

        let response: ListHelpCenterArticlesResponse = resp
            .json()
            .await
            .context("Failed to parse help center articles response")?;

        Ok(response.payload)
    }

    /// Get the current user's profile (to obtain author_id for article creation).
    pub async fn get_profile(&self) -> Result<i64> {
        let url = format!("{}/api/v1/profile", self.base_url);

        let resp = self
            .client
            .get(&url)
            .header("api_access_token", &self.api_token)
            .send()
            .await
            .context("Failed to get profile")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!("Chatwoot API error getting profile {}: {}", status, body);
        }

        #[derive(Deserialize)]
        struct ProfileResponse {
            id: i64,
        }

        let response: ProfileResponse = resp
            .json()
            .await
            .context("Failed to parse profile response")?;

        Ok(response.id)
    }

    /// List all articles in a portal (for sync operations).
    /// Returns articles with id, title, slug for matching.
    pub async fn list_articles(&self, portal_slug: &str) -> Result<Vec<HelpCenterArticle>> {
        let url = format!(
            "{}/api/v1/accounts/{}/portals/{}/articles",
            self.base_url, self.account_id, portal_slug
        );

        let resp = self
            .client
            .get(&url)
            .header("api_access_token", &self.api_token)
            .send()
            .await
            .context("Failed to list articles")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!("Chatwoot API error listing articles {}: {}", status, body);
        }

        let response: ListHelpCenterArticlesResponse =
            resp.json().await.context("Failed to parse articles")?;

        Ok(response.payload)
    }

    /// Create a new Help Center article.
    /// `author_id` is required - use the ID of the user who owns the API token.
    pub async fn create_article(
        &self,
        portal_slug: &str,
        title: &str,
        slug: &str,
        content: &str,
        description: &str,
        author_id: i64,
    ) -> Result<i64> {
        let url = format!(
            "{}/api/v1/accounts/{}/portals/{}/articles",
            self.base_url, self.account_id, portal_slug
        );

        #[derive(Serialize)]
        struct CreateArticleRequest<'a> {
            title: &'a str,
            slug: &'a str,
            content: &'a str,
            description: &'a str,
            status: i32,
            author_id: i64,
        }

        let resp = self
            .client
            .post(&url)
            .header("api_access_token", &self.api_token)
            .json(&CreateArticleRequest {
                title,
                slug,
                content,
                description,
                status: 1, // 1 = published
                author_id,
            })
            .send()
            .await
            .context("Failed to create article")?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!(
                "Chatwoot API error creating article (status {}): {}",
                status,
                body
            );
        }

        // Response is wrapped in "payload" field
        #[derive(Deserialize)]
        struct ArticlePayload {
            id: i64,
        }
        #[derive(Deserialize)]
        struct CreateArticleResponse {
            payload: ArticlePayload,
        }

        let body = resp
            .text()
            .await
            .context("Failed to read create article response body")?;

        let response: CreateArticleResponse = serde_json::from_str(&body)
            .with_context(|| format!("Failed to parse create article response: {}", body))?;

        Ok(response.payload.id)
    }

    /// Update an existing Help Center article.
    pub async fn update_article(
        &self,
        portal_slug: &str,
        article_id: i64,
        title: &str,
        content: &str,
        description: &str,
    ) -> Result<()> {
        let url = format!(
            "{}/api/v1/accounts/{}/portals/{}/articles/{}",
            self.base_url, self.account_id, portal_slug, article_id
        );

        #[derive(Serialize)]
        struct UpdateArticleRequest<'a> {
            title: &'a str,
            content: &'a str,
            description: &'a str,
            status: i32,
        }

        let resp = self
            .client
            .patch(&url)
            .header("api_access_token", &self.api_token)
            .json(&UpdateArticleRequest {
                title,
                content,
                description,
                status: 1, // 1 = published
            })
            .send()
            .await
            .context("Failed to update article")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!("Chatwoot API error updating article {}: {}", status, body);
        }

        Ok(())
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
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!("Chatwoot API error {}: {}", status, body);
        }

        Ok(())
    }

    /// Update conversation status via toggle_status endpoint.
    /// When called by an AgentBot to change status from pending to open,
    /// This triggers bot_handoff in Chatwoot which notifies all inbox agents.
    pub async fn update_conversation_status(
        &self,
        conversation_id: u64,
        status: &str,
    ) -> Result<()> {
        let url = format!(
            "{}/api/v1/accounts/{}/conversations/{}/toggle_status",
            self.base_url, self.account_id, conversation_id
        );

        #[derive(Serialize)]
        struct ToggleStatusRequest<'a> {
            status: &'a str,
        }

        let resp = self
            .client
            .post(&url)
            .header("api_access_token", &self.api_token)
            .json(&ToggleStatusRequest { status })
            .send()
            .await
            .context("Failed to toggle conversation status")?;

        if !resp.status().is_success() {
            let status_code = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!("Chatwoot API error {}: {}", status_code, body);
        }

        Ok(())
    }

    /// Fetch recent messages from a conversation for context.
    /// Returns (role, content) pairs where role is "customer" or "bot".
    pub async fn fetch_conversation_messages(
        &self,
        conversation_id: u64,
    ) -> Result<Vec<(String, String)>> {
        let url = format!(
            "{}/api/v1/accounts/{}/conversations/{}/messages",
            self.base_url, self.account_id, conversation_id
        );

        #[derive(Deserialize)]
        struct MessagesResponse {
            payload: Vec<Message>,
        }

        #[derive(Deserialize)]
        struct Message {
            content: Option<String>,
            message_type: i32, // 0 = incoming (customer), 1 = outgoing (bot/agent)
        }

        let resp = self
            .client
            .get(&url)
            .header("api_access_token", &self.api_token)
            .send()
            .await
            .context("Failed to fetch conversation messages")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {}>", e));
            anyhow::bail!("Chatwoot API error {}: {}", status, body);
        }

        let response: MessagesResponse = resp
            .json()
            .await
            .context("Failed to parse messages response")?;

        Ok(response
            .payload
            .into_iter()
            .filter_map(|m| {
                let content = m.content?;
                if content.trim().is_empty() {
                    return None;
                }
                let role = if m.message_type == 0 {
                    "customer"
                } else {
                    "bot"
                };
                Some((role.to_string(), content))
            })
            .collect())
    }
}
