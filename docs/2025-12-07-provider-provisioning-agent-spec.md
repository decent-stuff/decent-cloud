# Provider Provisioning Agent Spec

**Status:** In Progress (Proxmox MVP ~80% complete)
**Priority:** HIGH - Critical for automated cloud platform vision
**Prerequisite:** Payments fully working (Stripe + ICPay)

## Implementation Status (as of 2024-12-17)

### Completed ✓
- [x] Agent skeleton with polling loop (`dc-agent run`)
- [x] Configuration file parsing (TOML)
- [x] Ed25519 authentication with API (using provider main key - to be replaced)
- [x] Proxmox provisioner (full lifecycle: clone, configure, start, health check)
- [x] Setup wizard (`dc-agent setup proxmox`) - creates templates, API tokens
- [x] Test provisioning command (`dc-agent test-provision`)
- [x] Doctor command for config validation
- [x] API endpoint: `GET /providers/{pubkey}/contracts/pending-provision`
- [x] API endpoint: `PUT /provider/rental-requests/{id}/provisioning`

### Current Priority: Phase 4-5 (Delegated Keys + One-Liner UX)

**Phase 4: Delegated Agent Keys**
- [ ] Database: `provider_agent_delegations` table
- [ ] Database: `provider_agent_status` table
- [ ] API: Delegation CRUD endpoints
- [ ] API: Agent key authentication middleware
- [ ] Agent: `dc-agent init` command (keypair generation)
- [ ] Agent: `dc-agent register` command (delegation registration)

**Phase 5: One-Liner UX**
- [ ] Setup wizard: Generate agent keypair, prompt for delegation
- [ ] Doctor: `--verify-api` flag for API connectivity test
- [ ] API: `POST /providers/{pubkey}/heartbeat` endpoint
- [ ] Agent: Send heartbeat on startup and every 60s
- [ ] Dashboard: "online"/"offline" badge on provider cards

### Next: Phase 6-7
- [ ] Health check reporting and storage
- [ ] Uptime calculation for reputation
- [ ] Credential encryption (Ed25519 → X25519)

### Future
- [ ] Hetzner Cloud provisioner
- [ ] Docker provisioner
- [ ] DigitalOcean, Vultr provisioners

## Overview

The Provider Provisioning Agent is software that providers run to automatically provision services when contracts are accepted. This transforms Decent Cloud from "marketplace with manual fulfillment" to "automated cloud platform."

## Current State (Manual)

```
User pays → Contract created → Provider manually provisions → User waits (hours/days)
```

## Target State (Automated)

```
User pays → Contract created → Provider agent auto-provisions → User gets credentials (minutes)
```

## Architecture

### Components

```
┌─────────────────────────────────────────────────────────────────────┐
│                         PROVIDER INFRASTRUCTURE                      │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────────────┐     ┌─────────────────────────────────┐   │
│  │  Provisioning Agent │────▶│  Provider's Infrastructure      │   │
│  │  (dc-agent daemon)  │     │  - Own hardware (IPMI/PXE)      │   │
│  │                     │     │  - Cloud APIs (Hetzner, DO)     │   │
│  │  - Polls API        │     │  - Container runtime (Docker)   │   │
│  │  - Provisions VMs   │     │  - Virtualization (Proxmox)     │   │
│  │  - Reports status   │     │                                 │   │
│  │  - Health checks    │◀────│                                 │   │
│  └─────────────────────┘     └─────────────────────────────────┘   │
│           │                                                         │
└───────────│─────────────────────────────────────────────────────────┘
            │
            ▼ HTTPS
┌─────────────────────────────────────────────────────────────────────┐
│                         DECENT CLOUD API                            │
├─────────────────────────────────────────────────────────────────────┤
│  GET  /api/v1/providers/{pubkey}/contracts/pending                  │
│  POST /api/v1/contracts/{id}/provision                              │
│  POST /api/v1/contracts/{id}/health                                 │
└─────────────────────────────────────────────────────────────────────┘
```

### Agent Modes

1. **Poll Mode** (MVP): Agent polls API every N seconds for pending contracts
2. **Webhook Mode** (Future): API pushes events to agent via webhook
3. **WebSocket Mode** (Future): Persistent connection for real-time events

## One-Liner Provider Onboarding (Target UX)

The goal is for a provider to go from zero to "healthy on dashboard" with minimal commands.

### Target Experience (Proxmox)

```bash
# Step 1: Setup Proxmox node + generate config (interactive, ~5-10 mins)
dc-agent setup proxmox --host 192.168.1.100

# Step 2: Verify everything works
dc-agent doctor --verify-api

# Step 3: Test VM provisioning (optional but recommended)
dc-agent test-provision --ssh-pubkey "ssh-ed25519 AAAA..."

# Step 4: Start agent (provider shows "online" in dashboard)
dc-agent run
```

### What Each Command Does

#### `dc-agent setup proxmox --host <IP>`
1. Connects via SSH to Proxmox host
2. Creates cloud-init VM templates (Ubuntu 24.04, etc.)
3. Creates Proxmox API token with proper permissions
4. **NEW**: Generates Ed25519 keypair if not provided
5. **NEW**: Registers provider with Decent Cloud API
6. Writes complete `dc-agent.toml` (no manual editing required)

#### `dc-agent doctor --verify-api`
1. Validates config file syntax
2. Validates provisioner configuration
3. **NEW**: Tests API connectivity with signed request
4. **NEW**: Verifies provider is registered and keys match
5. Reports any issues with actionable fix instructions

#### `dc-agent test-provision`
1. Creates a test VM using the configured provisioner
2. Waits for IP address assignment
3. Runs health check
4. Terminates VM (unless `--keep` specified)
5. Reports success/failure with timing

#### `dc-agent run`
1. Starts polling loop
2. **NEW**: Sends periodic heartbeat to API (shows "online" in dashboard)
3. Provisions contracts as they arrive
4. Reports health checks for active instances

### Required API Additions for Full UX

```
# Provider heartbeat (agent sends every 60s when running)
POST /api/v1/providers/{pubkey}/heartbeat
Authorization: Ed25519 signature
{
  "version": "0.1.0",
  "provisioner_type": "proxmox",
  "capabilities": ["vm", "health_check"],
  "active_contracts": 5
}

Response:
{
  "success": true,
  "data": {
    "acknowledged": true,
    "next_heartbeat_seconds": 60
  }
}

# Get provider agent status (for dashboard)
GET /api/v1/providers/{pubkey}/agent-status
Response:
{
  "success": true,
  "data": {
    "online": true,
    "last_heartbeat_ns": 1734400000000000000,
    "version": "0.1.0",
    "provisioner_type": "proxmox",
    "active_contracts": 5
  }
}
```

### Database Addition for Provider Status

```sql
-- Track provider agent status
CREATE TABLE provider_agent_status (
    provider_pubkey BLOB PRIMARY KEY,
    online INTEGER NOT NULL DEFAULT 0,
    last_heartbeat_ns INTEGER,
    version TEXT,
    provisioner_type TEXT,
    capabilities TEXT,  -- JSON array
    active_contracts INTEGER DEFAULT 0,
    updated_at_ns INTEGER NOT NULL
);
```

## Agent Design

### Core Loop

```rust
loop {
    // 1. Fetch pending contracts (accepted, payment confirmed)
    let contracts = api.get_pending_contracts().await?;

    for contract in contracts {
        // 2. Provision based on offering type
        let result = match contract.product_type {
            "compute" => provision_vm(&contract).await,
            "dedicated" => provision_dedicated(&contract).await,
            "gpu" => provision_gpu(&contract).await,
            _ => Err("Unsupported product type"),
        };

        // 3. Report result back to API
        match result {
            Ok(instance) => {
                api.report_provisioned(&contract.id, instance).await?;
            }
            Err(e) => {
                api.report_failed(&contract.id, &e.to_string()).await?;
            }
        }
    }

    // 4. Health check active contracts
    let active = api.get_active_contracts().await?;
    for contract in active {
        let health = check_health(&contract).await;
        api.report_health(&contract.id, health).await?;
    }

    sleep(poll_interval).await;
}
```

### Configuration

```toml
# dc-agent.toml

[api]
endpoint = "https://api.decent-cloud.org"
provider_pubkey = "abc123..."
provider_secret_key = "..."  # Ed25519 private key for signing

[polling]
interval_seconds = 30
health_check_interval_seconds = 300

[provisioner]
type = "hetzner"  # or "proxmox", "docker", "manual"

[provisioner.hetzner]
api_token = "..."
default_location = "nbg1"
default_image = "ubuntu-22.04"

[provisioner.proxmox]
api_url = "https://proxmox.local:8006"
api_token = "..."
node = "pve1"
storage = "local-lvm"

[provisioner.docker]
socket = "/var/run/docker.sock"
network = "dc-network"
```

### Provisioner Interface

```rust
#[async_trait]
trait Provisioner {
    /// Provision a new instance based on contract requirements
    async fn provision(&self, contract: &Contract, offering: &Offering) -> Result<Instance>;

    /// Terminate an instance (on contract end/cancellation)
    async fn terminate(&self, instance_id: &str) -> Result<()>;

    /// Check instance health
    async fn health_check(&self, instance_id: &str) -> Result<HealthStatus>;

    /// Get instance details (IP, status, etc.)
    async fn get_instance(&self, instance_id: &str) -> Result<Option<Instance>>;
}

struct Instance {
    external_id: String,      // Provider's instance ID
    ip_address: String,       // Public IP
    ipv6_address: Option<String>,
    root_password: Option<String>,  // For initial access
    ssh_port: u16,
    status: InstanceStatus,
    provisioned_at: DateTime<Utc>,
}

enum HealthStatus {
    Healthy { uptime_seconds: u64, last_check: DateTime<Utc> },
    Unhealthy { reason: String, since: DateTime<Utc> },
    Unknown,
}
```

### Built-in Provisioners

#### 1. Hetzner Cloud Provisioner
```rust
impl Provisioner for HetznerProvisioner {
    async fn provision(&self, contract: &Contract, offering: &Offering) -> Result<Instance> {
        let server_type = self.map_offering_to_server_type(offering)?;
        let location = offering.datacenter_city.as_ref()
            .and_then(|c| self.city_to_location(c))
            .unwrap_or(&self.default_location);

        let response = self.client.create_server(CreateServerRequest {
            name: format!("dc-{}", contract.contract_id),
            server_type,
            location,
            image: &self.default_image,
            ssh_keys: vec![&contract.requester_ssh_pubkey],
            user_data: self.generate_cloud_init(contract),
        }).await?;

        Ok(Instance {
            external_id: response.server.id.to_string(),
            ip_address: response.server.public_net.ipv4.ip,
            root_password: response.root_password,
            // ...
        })
    }
}
```

#### 2. Proxmox Provisioner
For providers with own hardware running Proxmox VE.

#### 3. Docker Provisioner
For container-based offerings (cheaper, faster provisioning).

#### 4. Manual Provisioner
Sends notification to provider, waits for manual input via dashboard.

## API Extensions

### New Endpoints

```
# Get contracts ready for provisioning (accepted + payment confirmed)
GET /api/v1/providers/{pubkey}/contracts/pending-provision
Authorization: Ed25519 signature

Response:
{
  "contracts": [
    {
      "contract_id": "...",
      "offering_id": "...",
      "offering": { /* full offering details */ },
      "requester_ssh_pubkey": "ssh-ed25519 ...",
      "payment_confirmed_at_ns": 1234567890,
      "instance_config": { /* custom config if any */ }
    }
  ]
}

# Report successful provisioning
POST /api/v1/contracts/{contract_id}/provision
Authorization: Ed25519 signature (provider)
{
  "external_instance_id": "hcloud-12345",
  "ip_address": "1.2.3.4",
  "ipv6_address": "2001:db8::1",
  "root_password": "...",  // Encrypted with requester's pubkey
  "ssh_port": 22,
  "additional_details": { /* provider-specific */ }
}

# Report provisioning failure
POST /api/v1/contracts/{contract_id}/provision-failed
Authorization: Ed25519 signature (provider)
{
  "error": "Out of stock in requested location",
  "retry_possible": true
}

# Report health check
POST /api/v1/contracts/{contract_id}/health
Authorization: Ed25519 signature (provider)
{
  "status": "healthy",  // or "unhealthy", "unknown"
  "uptime_seconds": 86400,
  "checked_at_ns": 1234567890,
  "details": { /* optional metrics */ }
}

# Get active contracts for health monitoring
GET /api/v1/providers/{pubkey}/contracts/active
Authorization: Ed25519 signature
```

### Contract Status Flow Update

```
requested → pending → accepted → provisioning → provisioned → active → completed
                                     ↓
                              provision_failed
```

New status: `provisioning` - payment confirmed, agent is working on it.

## Agent Keypair Security (Delegated Sub-Key Model)

The agent uses a **separate keypair** from the provider's main identity key to limit blast radius if compromised.

### Security Model

```
┌─────────────────────────────────────────────────────────────────┐
│  Provider Main Key (high security, rarely used)                 │
│  - Signs offerings, contracts                                   │
│  - Stored securely (hardware key, air-gapped)                  │
│  - Used to create delegations                                   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ signs delegation
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Agent Key (lower security, used frequently)                    │
│  - Signs API requests for provisioning operations only          │
│  - Stored on server running dc-agent                            │
│  - Can be revoked without affecting main identity               │
│  - Scoped permissions: provision, health_check, heartbeat       │
└─────────────────────────────────────────────────────────────────┘
```

### Delegation Structure

```rust
struct AgentDelegation {
    /// The agent's public key being delegated to
    agent_pubkey: [u8; 32],
    /// The provider's main public key (delegator)
    provider_pubkey: [u8; 32],
    /// Permissions granted to this agent key
    permissions: Vec<AgentPermission>,
    /// Expiration timestamp (nanoseconds)
    expires_at_ns: Option<u64>,
    /// Human-readable label (e.g., "proxmox-agent-1")
    label: String,
    /// Signature by provider's main key over the above fields
    signature: [u8; 64],
}

enum AgentPermission {
    /// Can report provisioning status
    Provision,
    /// Can report health checks
    HealthCheck,
    /// Can send heartbeats
    Heartbeat,
    /// Can fetch pending contracts
    FetchContracts,
}
```

### API Authentication Flow

```
1. Agent generates request with its own keypair
2. Request includes:
   - X-Agent-Pubkey: <agent's pubkey>
   - X-Timestamp: <unix timestamp>
   - X-Signature: <signature by agent key>
3. API looks up delegation for agent_pubkey
4. API verifies:
   - Delegation signature is valid (signed by provider main key)
   - Delegation hasn't expired
   - Requested operation is in permissions list
   - Request signature is valid (signed by agent key)
5. If all pass, request is authorized as acting for provider
```

### Delegation Management

```
# Create delegation (requires main key signature)
POST /api/v1/providers/{pubkey}/agent-delegations
{
  "agent_pubkey": "abc123...",
  "permissions": ["provision", "health_check", "heartbeat", "fetch_contracts"],
  "expires_at_ns": null,  // or timestamp for expiry
  "label": "proxmox-server-1",
  "signature": "..."  // signed by provider main key
}

# List delegations
GET /api/v1/providers/{pubkey}/agent-delegations

# Revoke delegation
DELETE /api/v1/providers/{pubkey}/agent-delegations/{agent_pubkey}
```

### Database Schema

```sql
CREATE TABLE provider_agent_delegations (
    id INTEGER PRIMARY KEY,
    provider_pubkey BLOB NOT NULL,
    agent_pubkey BLOB NOT NULL UNIQUE,
    permissions TEXT NOT NULL,  -- JSON array
    expires_at_ns INTEGER,
    label TEXT,
    signature BLOB NOT NULL,
    created_at_ns INTEGER NOT NULL,
    revoked_at_ns INTEGER,  -- NULL if active
    FOREIGN KEY (provider_pubkey) REFERENCES node_providers(pubkey)
);

CREATE INDEX idx_agent_delegations_agent ON provider_agent_delegations(agent_pubkey);
CREATE INDEX idx_agent_delegations_provider ON provider_agent_delegations(provider_pubkey);
```

### Blast Radius if Agent Key Compromised

| Action | Main Key | Agent Key |
|--------|----------|-----------|
| Create/modify offerings | ✓ | ✗ |
| Sign contracts | ✓ | ✗ |
| Withdraw funds | ✓ | ✗ |
| Report provisioning | ✓ | ✓ (scoped) |
| Send health checks | ✓ | ✓ (scoped) |
| Revoke agent delegation | ✓ | ✗ |

**Recovery from compromise:** Provider revokes the delegation, generates new agent keypair, creates new delegation. Main identity remains intact.

## Instance Credential Security

Instance credentials (passwords, keys) must be protected:

1. **Encrypt with requester's pubkey**: Agent encrypts root_password with user's Ed25519 pubkey (converted to X25519)
2. **Store encrypted**: API stores only encrypted blob
3. **Decrypt client-side**: User's client decrypts with their private key
4. **TTL**: Credentials auto-deleted after N days

```rust
// Agent side - Ed25519 to X25519 conversion for encryption
use ed25519_dalek::VerifyingKey;
use x25519_dalek::PublicKey as X25519PublicKey;

fn encrypt_for_requester(plaintext: &[u8], requester_ed25519_pubkey: &[u8; 32]) -> Result<Vec<u8>> {
    // Convert Ed25519 public key to X25519 for encryption
    let ed_key = VerifyingKey::from_bytes(requester_ed25519_pubkey)?;
    let x25519_pubkey = ed_key.to_montgomery();

    // Use crypto_box or similar for authenticated encryption
    let encrypted = crypto_box::seal(plaintext, &x25519_pubkey)?;
    Ok(encrypted)
}

// Client side
let decrypted = crypto_box::open(&encrypted, &user_x25519_secret_key)?;
```

## Health & Reputation Integration

Health check data feeds into reputation:

1. **Uptime tracking**: Healthy checks increase provider reliability score
2. **Downtime penalties**: Extended unhealthy status affects reputation
3. **SLA enforcement**: Contract specifies uptime guarantee, violations trigger disputes

```sql
-- Track health history
CREATE TABLE contract_health_checks (
    id INTEGER PRIMARY KEY,
    contract_id BLOB NOT NULL,
    status TEXT NOT NULL,  -- 'healthy', 'unhealthy', 'unknown'
    uptime_seconds INTEGER,
    checked_at_ns INTEGER NOT NULL,
    details TEXT,  -- JSON
    FOREIGN KEY (contract_id) REFERENCES contract_sign_requests(contract_id)
);

-- Aggregate for reputation
SELECT
    provider_pubkey,
    COUNT(*) as total_checks,
    SUM(CASE WHEN status = 'healthy' THEN 1 ELSE 0 END) as healthy_checks,
    (healthy_checks * 100.0 / total_checks) as uptime_percent
FROM contract_health_checks
JOIN contract_sign_requests USING (contract_id)
WHERE checked_at_ns > ?  -- Last 30 days
GROUP BY provider_pubkey;
```

## Implementation Phases

### Phase 1: API Extensions ✓ (Mostly Complete)
- [x] Add `provisioning` status to contract flow
- [x] Implement `GET /providers/{pubkey}/contracts/pending-provision` endpoint
- [x] Implement `PUT /provider/rental-requests/{id}/provisioning` endpoint
- [ ] Implement health check reporting endpoint
- [ ] Add `contract_health_checks` table
- [ ] Add `provider_agent_status` table

### Phase 2: Agent MVP ✓ (Complete)
- [x] Agent skeleton with polling loop
- [x] Configuration file parsing (TOML)
- [x] Ed25519 authentication with API
- [x] Manual provisioner stub
- [x] Script provisioner (for custom backends)

### Phase 3: Proxmox Provisioner ✓ (Complete)
- [x] Proxmox VE API integration
- [x] VM cloning from cloud-init templates
- [x] Instance lifecycle (create, configure, start, terminate)
- [x] Health checks via Proxmox API
- [x] Setup wizard with template creation
- [x] API token auto-generation

### Phase 4: Delegated Agent Keys (Current Priority)

**Goal:** Implement the delegated sub-key security model so agent uses separate keypair with limited permissions.

#### 4.1 Database Schema
- [ ] Add `provider_agent_delegations` table (migration)
- [ ] Add `provider_agent_status` table (migration)

#### 4.2 API: Delegation Management
- [ ] `POST /api/v1/providers/{pubkey}/agent-delegations` - Create delegation
- [ ] `GET /api/v1/providers/{pubkey}/agent-delegations` - List delegations
- [ ] `DELETE /api/v1/providers/{pubkey}/agent-delegations/{agent_pubkey}` - Revoke

#### 4.3 API: Agent Authentication
- [ ] Modify auth middleware to accept agent keys via `X-Agent-Pubkey` header
- [ ] Lookup delegation, verify signature chain, check permissions
- [ ] Scope endpoints by required permission (e.g., `Provision`, `HealthCheck`)

#### 4.4 Agent: Keypair Generation
- [ ] `dc-agent init` command - generates Ed25519 agent keypair
- [ ] Outputs agent pubkey for delegation creation
- [ ] Stores secret key in config or separate file

#### 4.5 Agent: Delegation Registration
- [ ] `dc-agent register` command - creates delegation via API
- [ ] Requires provider main key to sign delegation
- [ ] Option 1: Provider signs offline, pastes signature
- [ ] Option 2: Provider main key available locally (less secure)

### Phase 5: One-Liner UX

**Goal:** Provider goes from zero to "healthy on dashboard" with minimal commands.

#### 5.1 Setup Wizard Improvements
- [ ] Generate agent keypair during `dc-agent setup proxmox`
- [ ] Prompt for delegation signature (provider signs with main key)
- [ ] Write complete config (no manual editing needed)
- [ ] Print next steps clearly

#### 5.2 Doctor Command Improvements
- [ ] Add `--verify-api` flag to test actual API connectivity
- [ ] Verify delegation is registered and valid
- [ ] Check provider exists in system
- [ ] Test signed request succeeds

#### 5.3 Provider Heartbeat
- [ ] Add `POST /api/v1/providers/{pubkey}/heartbeat` endpoint
- [ ] Store in `provider_agent_status` table
- [ ] Agent sends heartbeat on startup and every 60s
- [ ] Mark provider "offline" if no heartbeat for 5 minutes

#### 5.4 Dashboard Integration
- [ ] Show "online"/"offline" badge on provider cards
- [ ] Show last heartbeat timestamp
- [ ] Show agent version and provisioner type

### Phase 6: Health & Reputation

**Goal:** Track instance health, feed into provider reputation.

#### 6.1 Health Check Storage
- [ ] Add `POST /api/v1/contracts/{id}/health` endpoint
- [ ] Store in `contract_health_checks` table
- [ ] Agent reports health every 5 minutes for active contracts

#### 6.2 Health History & Aggregation
- [ ] Calculate uptime percentage per provider (last 30 days)
- [ ] Expose via `GET /api/v1/providers/{pubkey}/health-summary`
- [ ] Include in provider stats response

#### 6.3 Dashboard Health Display
- [ ] Show uptime percentage on provider profile
- [ ] Show health history graph (optional)
- [ ] Highlight providers with <99% uptime

### Phase 7: Instance Credential Encryption

**Goal:** Encrypt VM credentials so only requester can read them.

- [ ] Add `x25519-dalek` and `crypto_box` dependencies to agent
- [ ] Convert requester Ed25519 pubkey to X25519 for encryption
- [ ] Encrypt root_password before sending to API
- [ ] Frontend: decrypt credentials with user's private key
- [ ] Auto-delete credentials after 7 days

### Phase 8: Hetzner Provisioner (Future)
- [ ] Hetzner Cloud API integration
- [ ] Offering → server_type mapping
- [ ] Instance lifecycle (create, terminate)
- [ ] Health checks via API

### Phase 9: Additional Provisioners (Future)
- [ ] Docker provisioner
- [ ] DigitalOcean provisioner
- [ ] Vultr provisioner

## Distribution

### Docker Image
```dockerfile
FROM rust:1.75-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin dc-agent

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/dc-agent /usr/local/bin/
COPY dc-agent.toml.example /etc/dc-agent/
ENTRYPOINT ["dc-agent"]
```

### Provider Onboarding
1. Provider downloads/runs agent
2. Agent generates keypair or uses existing provider key
3. Provider configures provisioner (Hetzner API key, Proxmox creds, etc.)
4. Provider tests with staging contract
5. Provider goes live

## Security Considerations

1. **Agent authentication**: Ed25519 signatures on all API calls
2. **Credential encryption**: User credentials never stored in plaintext
3. **Rate limiting**: Prevent agent from overwhelming API
4. **Audit logging**: All provisioning actions logged
5. **Sandboxing**: Agent runs with minimal privileges

## Success Metrics

- **Time to provision**: < 5 minutes for cloud APIs, < 15 minutes for bare metal
- **Provisioning success rate**: > 95%
- **Health check coverage**: 100% of active contracts
- **Agent adoption**: 50%+ of providers running agent within 3 months

## Open Questions

1. **Credential rotation**: Should we support rotating root passwords periodically?
2. **Multi-instance**: One contract = one instance, or support clusters?
3. **Custom images**: Allow providers to offer custom OS images?
4. **Bandwidth monitoring**: Track/report bandwidth usage for metered offerings?
5. **Backup integration**: Should agent handle backup provisioning?

---

## Appendix: Comparison with Akash

| Aspect | Akash | Decent Cloud Agent |
|--------|-------|-------------------|
| Deployment | Kubernetes-native | Multi-backend (VMs, containers, bare metal) |
| Payment | AKT escrow on-chain | Stripe + ICPay (already paid) |
| Provider stack | Heavy (K8s + Akash provider) | Light (single binary) |
| Workloads | Containers only | VMs, containers, bare metal |
| User UX | SDL manifests | Simple: pick offering, pay |

Decent Cloud's agent is intentionally simpler - providers can use existing infrastructure without adopting a full Kubernetes stack.
