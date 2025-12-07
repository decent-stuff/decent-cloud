# Provider Provisioning Agent Spec

**Status:** Draft (pending payment system completion)
**Priority:** HIGH - Critical for automated cloud platform vision
**Prerequisite:** Payments fully working (Stripe + ICPay)

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

## Credential Security

Instance credentials (passwords, keys) must be protected:

1. **Encrypt with requester's pubkey**: Agent encrypts root_password with user's Ed25519 pubkey
2. **Store encrypted**: API stores only encrypted blob
3. **Decrypt client-side**: User's client decrypts with their private key
4. **TTL**: Credentials auto-deleted after N days

```rust
// Agent side
let encrypted = encrypt_for_recipient(
    root_password.as_bytes(),
    &contract.requester_pubkey,
)?;

// Client side
let decrypted = decrypt_with_private_key(
    &encrypted,
    &user_private_key,
)?;
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

### Phase 1: API Extensions (Week 1)
- [ ] Add `provisioning` status to contract flow
- [ ] Implement `/contracts/pending-provision` endpoint
- [ ] Implement `/contracts/{id}/provision` endpoint
- [ ] Implement `/contracts/{id}/provision-failed` endpoint
- [ ] Add `contract_health_checks` table

### Phase 2: Agent MVP (Week 2)
- [ ] Agent skeleton with polling loop
- [ ] Configuration file parsing
- [ ] Ed25519 authentication with API
- [ ] Manual provisioner (notification + dashboard input)

### Phase 3: Hetzner Provisioner (Week 3)
- [ ] Hetzner Cloud API integration
- [ ] Offering → server_type mapping
- [ ] Instance lifecycle (create, terminate)
- [ ] Health checks via API

### Phase 4: Health & Reputation (Week 4)
- [ ] Health check reporting endpoint
- [ ] Health history storage
- [ ] Reputation score integration
- [ ] Dashboard uptime display

### Phase 5: Additional Provisioners (Future)
- [ ] Proxmox provisioner
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
