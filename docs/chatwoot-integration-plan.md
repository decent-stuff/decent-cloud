# Chatwoot Integration Plan

**Status:** Approved - Option A (Separate Credentials)
**Date:** 2025-12-02

## Executive Summary

Integrate Chatwoot Community Edition as the provider-customer messaging system, enabling:
- In-contract communication with response time tracking
- Multi-channel support (web, email, WhatsApp, Telegram, etc.)
- Provider "WOW" experience with real-time notifications

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Your Platform                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────────────┐ │
│  │   Website    │     │   Rust API   │     │   Existing Postgres  │ │
│  │  (SvelteKit) │────▶│   (Poem)     │────▶│   (accounts, etc.)   │ │
│  └──────┬───────┘     └──────┬───────┘     └──────────────────────┘ │
│         │                    │                                       │
│         │ Ed25519 Auth       │ HMAC Generation                      │
│         │                    │ Webhook Handler                       │
│         │                    │ Chatwoot API Client                   │
└─────────┼────────────────────┼───────────────────────────────────────┘
          │                    │
          ▼                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│              Chatwoot (support.decent-cloud.org)                       │
├─────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │  Chatwoot    │  │   Sidekiq    │  │    Redis     │              │
│  │    Web       │  │   (Worker)   │  │              │              │
│  └──────────────┘  └──────────────┘  └──────────────┘              │
│                           │                                          │
│                           ▼                                          │
│         ┌─────────────────────────────────────┐                     │
│         │  PostgreSQL (chatwoot_production)   │                     │
│         │  (separate DB, same cluster)        │                     │
│         └─────────────────────────────────────┘                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Authentication Architecture

### The Challenge

We have **two different auth contexts**:

| Context | Auth Method | Who |
|---------|-------------|-----|
| **Customers** (widget) | HMAC Identity Validation | Users contacting providers |
| **Providers** (agents) | Chatwoot credentials | Providers responding to tickets |

### How Each Works

#### 1. Customer Authentication (Widget) - SOLVED

Customers use the embedded Chatwoot widget on our platform:

```
Customer logs in (Ed25519) → Our API generates HMAC → Widget authenticated
```

- HMAC proves to Chatwoot the customer is authenticated by us
- No separate Chatwoot credentials needed
- Works seamlessly in browser

#### 2. Provider Authentication (Agent Dashboard & Mobile) - REQUIRES DECISION

Providers need to access Chatwoot as "agents" to respond to tickets:

**Option A: Separate Chatwoot Credentials (Simplest)**
```
Provider registers → We create Chatwoot agent account via API
Provider logs into Chatwoot separately with email/password
Mobile app uses same credentials
```
- Pros: Works today, no Enterprise needed
- Cons: Two logins, password sync issues

**Option B: Embedded Agent Dashboard (No Mobile)**
```
Provider logs in (Ed25519) → We proxy Chatwoot agent API
All interaction through our UI, no direct Chatwoot access
```
- Pros: Single auth, full control
- Cons: Must build agent UI ourselves, no mobile app

**Option C: SSO via SAML/OIDC (Enterprise Only - $19/agent/month)**
```
Provider clicks "Support" → Redirects to our IdP → Auto-login to Chatwoot
Mobile app also uses SSO flow
```
- Pros: True single sign-on, mobile works
- Cons: Requires Enterprise license, we must run SAML/OIDC IdP

**Option D: Auto-Login Token (Hybrid)**
```
Provider authenticated (Ed25519) → Backend generates Chatwoot session
Frontend opens Chatwoot in iframe/redirect with session token
```
- Pros: Seamless web experience
- Cons: Mobile app still needs credentials, requires custom Chatwoot code

### Decision: Option A (Separate Credentials) ✅

**Approved approach:**
1. When provider registers → auto-create Chatwoot agent via API
2. Generate secure random password
3. Send "Set your Chatwoot support password" email with reset link
4. Provider logs into `support.decent-cloud.org` with email/password
5. Same credentials work for mobile apps

**Future migration path:** If 20+ providers and mobile SSO critical → upgrade to Enterprise + SAML/OIDC

**Provider onboarding flow:**
```
Provider registers on platform
        │
        ▼
Backend creates Chatwoot agent via API (no password sent)
        │
        ▼
Chatwoot automatically sends "Set your password" email
        │
        ▼
Provider clicks link, sets their own password
        │
        ▼
Provider can access support.decent-cloud.org + mobile apps
```

**Security note:** Never send passwords via email. Chatwoot's built-in password
reset flow is the secure standard (same pattern as Slack, GitHub, etc.).

---

## Infrastructure

### Docker Compose

```yaml
# docker-compose.chatwoot.yml
version: '3.8'

services:
  chatwoot-web:
    image: chatwoot/chatwoot:latest
    container_name: chatwoot-web
    depends_on:
      - chatwoot-redis
    environment:
      # Database (your existing PostgreSQL cluster)
      POSTGRES_HOST: ${POSTGRES_HOST}
      POSTGRES_PORT: 5432
      POSTGRES_DATABASE: chatwoot_production
      POSTGRES_USERNAME: ${POSTGRES_USER}
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}

      # Redis
      REDIS_URL: redis://chatwoot-redis:6379

      # App settings
      RAILS_ENV: production
      SECRET_KEY_BASE: ${CHATWOOT_SECRET_KEY_BASE}
      FRONTEND_URL: https://support.decent-cloud.org

      # HMAC for customer identity validation
      CHATWOOT_INBOX_HMAC_SECRET_KEY: ${CHATWOOT_HMAC_SECRET}

      # Email notifications
      MAILER_SENDER_EMAIL: support@decent-cloud.org
      SMTP_ADDRESS: ${SMTP_HOST}
      SMTP_PORT: 587
      SMTP_USERNAME: ${SMTP_USER}
      SMTP_PASSWORD: ${SMTP_PASSWORD}
      SMTP_AUTHENTICATION: plain
      SMTP_ENABLE_STARTTLS_AUTO: "true"

      # OpenAI (optional - for AI suggestions)
      OPENAI_API_KEY: ${OPENAI_API_KEY}

      # Disable telemetry
      DISABLE_TELEMETRY: "true"
    command: bundle exec rails s -p 3000 -b '0.0.0.0'
    ports:
      - "3000:3000"
    restart: unless-stopped
    networks:
      - chatwoot-net

  chatwoot-worker:
    image: chatwoot/chatwoot:latest
    container_name: chatwoot-worker
    depends_on:
      - chatwoot-redis
    environment:
      POSTGRES_HOST: ${POSTGRES_HOST}
      POSTGRES_PORT: 5432
      POSTGRES_DATABASE: chatwoot_production
      POSTGRES_USERNAME: ${POSTGRES_USER}
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
      REDIS_URL: redis://chatwoot-redis:6379
      RAILS_ENV: production
      SECRET_KEY_BASE: ${CHATWOOT_SECRET_KEY_BASE}
      FRONTEND_URL: https://support.decent-cloud.org
      SMTP_ADDRESS: ${SMTP_HOST}
      SMTP_PORT: 587
      SMTP_USERNAME: ${SMTP_USER}
      SMTP_PASSWORD: ${SMTP_PASSWORD}
    command: bundle exec sidekiq -C config/sidekiq.yml
    restart: unless-stopped
    networks:
      - chatwoot-net

  chatwoot-redis:
    image: redis:7-alpine
    container_name: chatwoot-redis
    volumes:
      - chatwoot-redis-data:/data
    restart: unless-stopped
    networks:
      - chatwoot-net

volumes:
  chatwoot-redis-data:

networks:
  chatwoot-net:
```

### Database Setup

```sql
-- Run on existing PostgreSQL cluster
CREATE DATABASE chatwoot_production;
CREATE USER chatwoot WITH PASSWORD 'secure_password_here';
GRANT ALL PRIVILEGES ON DATABASE chatwoot_production TO chatwoot;
```

### Nginx/Reverse Proxy

```nginx
# support.decent-cloud.org
server {
    listen 443 ssl http2;
    server_name support.decent-cloud.org;

    ssl_certificate /etc/ssl/certs/decent-cloud.org.pem;
    ssl_certificate_key /etc/ssl/private/decent-cloud.org.key;

    location / {
        proxy_pass http://chatwoot-web:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

---

## Backend Integration (Rust)

### New Files

```
api/src/
├── chatwoot/
│   ├── mod.rs          # Module exports
│   ├── client.rs       # Chatwoot API client
│   ├── hmac.rs         # HMAC identity hash generation
│   └── webhook.rs      # Webhook event handler
└── routes/
    └── chatwoot.rs     # API endpoints for frontend
```

### HMAC Generation

```rust
// api/src/chatwoot/hmac.rs
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Generate HMAC hash for Chatwoot identity validation
pub fn generate_identity_hash(identifier: &str, hmac_secret: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(hmac_secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(identifier.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}
```

### Chatwoot API Client

```rust
// api/src/chatwoot/client.rs
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct ChatwootClient {
    client: Client,
    base_url: String,
    api_token: String,
    account_id: u32,
}

impl ChatwootClient {
    pub fn new(base_url: &str, api_token: &str, account_id: u32) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
            api_token: api_token.to_string(),
            account_id,
        }
    }

    /// Create agent account for provider
    pub async fn create_agent(
        &self,
        email: &str,
        name: &str,
    ) -> Result<AgentResponse> {
        let resp = self.client
            .post(format!(
                "{}/api/v1/accounts/{}/agents",
                self.base_url, self.account_id
            ))
            .header("api_access_token", &self.api_token)
            .json(&serde_json::json!({
                "email": email,
                "name": name,
                "role": "agent"
            }))
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    /// Create or update contact (customer)
    pub async fn upsert_contact(
        &self,
        inbox_id: u32,
        identifier: &str,
        name: &str,
        email: Option<&str>,
    ) -> Result<ContactResponse> {
        let resp = self.client
            .post(format!(
                "{}/api/v1/accounts/{}/contacts",
                self.base_url, self.account_id
            ))
            .header("api_access_token", &self.api_token)
            .json(&serde_json::json!({
                "inbox_id": inbox_id,
                "identifier": identifier,
                "name": name,
                "email": email
            }))
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    /// Create conversation for a contract
    pub async fn create_conversation(
        &self,
        inbox_id: u32,
        contact_id: u64,
        contract_id: &str,
    ) -> Result<ConversationResponse> {
        let resp = self.client
            .post(format!(
                "{}/api/v1/accounts/{}/conversations",
                self.base_url, self.account_id
            ))
            .header("api_access_token", &self.api_token)
            .json(&serde_json::json!({
                "inbox_id": inbox_id,
                "contact_id": contact_id,
                "custom_attributes": {
                    "contract_id": contract_id
                }
            }))
            .send()
            .await?;
        Ok(resp.json().await?)
    }
}

#[derive(Debug, Deserialize)]
pub struct AgentResponse {
    pub id: u64,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct ContactResponse {
    pub id: u64,
    pub identifier: String,
}

#[derive(Debug, Deserialize)]
pub struct ConversationResponse {
    pub id: u64,
}
```

### API Endpoint for Widget Auth

```rust
// api/src/routes/chatwoot.rs
use poem_openapi::{payload::Json, Object, OpenApi};

#[derive(Object)]
pub struct ChatwootIdentityResponse {
    identifier: String,
    identifier_hash: String,
}

pub struct ChatwootApi;

#[OpenApi]
impl ChatwootApi {
    /// Get Chatwoot identity hash for authenticated user (customer widget)
    #[oai(path = "/chatwoot/identity", method = "get")]
    async fn get_identity(
        &self,
        user: ApiAuthenticatedUser,
    ) -> Json<ChatwootIdentityResponse> {
        let identifier = hex::encode(&user.pubkey);
        let hmac_secret = std::env::var("CHATWOOT_HMAC_SECRET")
            .expect("CHATWOOT_HMAC_SECRET required");

        let identifier_hash = generate_identity_hash(&identifier, &hmac_secret);

        Json(ChatwootIdentityResponse { identifier, identifier_hash })
    }
}
```

---

## Frontend Integration (SvelteKit)

### Chatwoot Widget (Customer Side)

```svelte
<!-- src/lib/components/ChatwootWidget.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { authStore } from '$lib/stores/auth';
  import { signedFetch } from '$lib/services/api';
  import { PUBLIC_CHATWOOT_WEBSITE_TOKEN } from '$env/static/public';

  onMount(async () => {
    // Load Chatwoot SDK
    const script = document.createElement('script');
    script.src = 'https://support.decent-cloud.org/packs/js/sdk.js';
    script.defer = true;
    script.async = true;
    document.head.appendChild(script);

    script.onload = async () => {
      window.chatwootSettings = {
        hideMessageBubble: false,
        position: 'right',
        locale: 'en',
        type: 'standard'
      };

      window.chatwootSDK.run({
        websiteToken: PUBLIC_CHATWOOT_WEBSITE_TOKEN,
        baseUrl: 'https://support.decent-cloud.org'
      });

      // Authenticate if user is logged in
      const identity = $authStore.activeIdentity;
      if (identity?.account) {
        const resp = await signedFetch('/api/v1/chatwoot/identity');
        const { identifier, identifier_hash } = await resp.json();

        window.$chatwoot.setUser(identifier, {
          identifier_hash,
          name: identity.account.username,
          email: identity.account.email
        });
      }
    };
  });
</script>
```

### Provider Support Link

```svelte
<!-- src/routes/dashboard/+layout.svelte -->
<script>
  // Link to Chatwoot agent dashboard
  const supportUrl = 'https://support.decent-cloud.org/app/accounts/1/dashboard';
</script>

<nav>
  <a href={supportUrl} target="_blank" rel="noopener">
    Support Dashboard
  </a>
</nav>
```

---

## Response Time Tracking

### Database Schema

```sql
-- api/migrations/XXX_chatwoot_tracking.sql

-- Track message events from webhooks
CREATE TABLE chatwoot_message_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id TEXT NOT NULL,
    chatwoot_conversation_id INTEGER NOT NULL,
    chatwoot_message_id INTEGER NOT NULL UNIQUE,
    sender_type TEXT NOT NULL CHECK (sender_type IN ('customer', 'provider')),
    created_at INTEGER NOT NULL -- Unix timestamp
);

CREATE INDEX idx_chatwoot_events_contract ON chatwoot_message_events(contract_id);
CREATE INDEX idx_chatwoot_events_conversation ON chatwoot_message_events(chatwoot_conversation_id);
```

### Webhook Handler

```rust
// api/src/chatwoot/webhook.rs

#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    pub event: String,
    pub conversation: Option<Conversation>,
    pub message: Option<Message>,
}

pub async fn handle_webhook(
    payload: WebhookPayload,
    db: &Database,
) -> Result<()> {
    if payload.event == "message_created" {
        if let (Some(conv), Some(msg)) = (payload.conversation, payload.message) {
            let contract_id = conv.custom_attributes
                .and_then(|a| a.get("contract_id"))
                .and_then(|v| v.as_str());

            if let Some(contract_id) = contract_id {
                let sender_type = match msg.message_type.as_str() {
                    "incoming" => "customer",
                    "outgoing" => "provider",
                    _ => return Ok(()),
                };

                db.insert_chatwoot_event(
                    contract_id,
                    conv.id,
                    msg.id,
                    sender_type,
                    msg.created_at,
                ).await?;
            }
        }
    }
    Ok(())
}
```

### Response Time Query

```sql
-- Calculate average response time per provider
SELECT
    p.username as provider,
    AVG(
        CASE
            WHEN response.created_at IS NOT NULL
            THEN response.created_at - customer_msg.created_at
            ELSE NULL
        END
    ) as avg_response_seconds,
    COUNT(DISTINCT customer_msg.id) as total_inquiries
FROM chatwoot_message_events customer_msg
JOIN contracts c ON c.id = customer_msg.contract_id
JOIN accounts p ON p.id = c.provider_id
LEFT JOIN chatwoot_message_events response ON
    response.chatwoot_conversation_id = customer_msg.chatwoot_conversation_id
    AND response.sender_type = 'provider'
    AND response.created_at > customer_msg.created_at
    AND response.id = (
        SELECT MIN(id) FROM chatwoot_message_events
        WHERE chatwoot_conversation_id = customer_msg.chatwoot_conversation_id
          AND sender_type = 'provider'
          AND created_at > customer_msg.created_at
    )
WHERE customer_msg.sender_type = 'customer'
GROUP BY p.id;
```

---

## Features Included (Community Edition)

### Channels (All Free)
- Website live chat widget
- Email integration
- Facebook Messenger
- Instagram DMs
- Twitter/X DMs
- WhatsApp (via Cloud API, Twilio, or 360dialog)
- Telegram
- Line
- SMS (via Twilio/Bandwidth)
- Custom API channel

### Agent Features (All Free)
- Unified omnichannel inbox
- Canned responses
- Private notes & @mentions
- Labels and custom attributes
- Keyboard shortcuts
- Auto-assignment
- Business hours & auto-responders
- Teams
- Automations (rules-based)

### AI Features (Free with BYOK)
- Reply suggestions
- Message tone improvement
- Conversation summarization
- (Requires your own OpenAI API key)

### Reporting (All Free)
- Conversation reports
- Agent performance
- First Response Time (FRT)
- Resolution time
- CSAT surveys
- Downloadable reports

### NOT Included (Enterprise Only)
- Captain AI Agent (auto-responses)
- SAML/OIDC SSO
- Custom roles
- Audit logs
- SLA management
- Priority support

---

## Deployment Checklist

### 1. Prerequisites
- [ ] PostgreSQL cluster accessible
- [ ] SMTP server configured
- [ ] Domain `support.decent-cloud.org` ready
- [ ] SSL certificate for subdomain

### 2. Secrets Generation
```bash
export CHATWOOT_SECRET_KEY_BASE=$(openssl rand -hex 64)
export CHATWOOT_HMAC_SECRET=$(openssl rand -hex 32)
```

### 3. Database Setup
```bash
psql -h $POSTGRES_HOST -U postgres -c "CREATE DATABASE chatwoot_production;"
psql -h $POSTGRES_HOST -U postgres -c "CREATE USER chatwoot WITH PASSWORD 'xxx';"
psql -h $POSTGRES_HOST -U postgres -c "GRANT ALL ON DATABASE chatwoot_production TO chatwoot;"
```

### 4. Container Deployment
```bash
docker compose -f docker-compose.chatwoot.yml up -d
docker compose exec chatwoot-web bundle exec rails db:chatwoot_prepare
```

### 5. Initial Configuration
```bash
# Create super admin
docker compose exec chatwoot-web bundle exec rails console
# > SuperAdmin.create!(email: 'admin@decent-cloud.org', password: 'secure')

# Then via web UI:
# 1. Create account
# 2. Create inbox (API channel for contracts)
# 3. Get inbox identifier for widget
# 4. Enable HMAC in inbox settings
```

### 6. Backend Integration
- [ ] Add `chatwoot` module to Rust API
- [ ] Add `/api/v1/chatwoot/identity` endpoint
- [ ] Add `/api/v1/webhooks/chatwoot` endpoint
- [ ] Add agent creation on provider registration
- [ ] Add conversation creation on contract creation

### 7. Frontend Integration
- [ ] Add ChatwootWidget component
- [ ] Add to customer-facing pages
- [ ] Add provider support dashboard link

---

## Open Questions

1. ~~**Provider mobile app usage**~~ → **DECIDED: Option A (separate credentials)**

2. **WhatsApp integration** - Do providers need WhatsApp?
   - Meta charges after free tier (1000 conversations/month)
   - Requires business verification
   - Decision: TBD

3. **Response time SLAs** - Should we enforce SLAs?
   - Community Edition has FRT reporting but not SLA enforcement
   - Could build our own SLA alerts via webhooks
   - Decision: TBD

---

## Cost Estimate

| Component | Cost |
|-----------|------|
| Chatwoot Community | Free |
| Redis container | ~$5/month (minimal VPS resources) |
| PostgreSQL | Already have |
| OpenAI API (optional) | ~$0.01-0.10 per suggestion |
| WhatsApp (if used) | ~$0.005-0.08 per message after free tier |
| **Total** | **~$5-20/month** |

Enterprise (if needed later): $19-99/agent/month
