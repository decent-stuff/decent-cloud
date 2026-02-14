# Self-Provisioning Platform Design

Transform Decent Cloud from a provider marketplace into a unified platform where any user can self-provision cloud resources and optionally list them on the marketplace.

## Vision

**Current**: Providers run dc-agent on their Proxmox hosts → VMs listed on marketplace → tenants rent them

**Target**: Any user connects their cloud accounts (Hetzner, Proxmox) → self-provision resources → optionally list on marketplace

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           User Dashboard                                 │
├─────────────────────────────────────────────────────────────────────────┤
│  Cloud Accounts          │  My Resources         │  Marketplace        │
│  ┌─────────────────────┐ │  ┌─────────────────┐  │  ┌────────────────┐  │
│  │ Hetzner ••••••••    │ │  │ vm-prod-01      │  │  │ My Offerings   │  │
│  │ Proxmox ••••••      │ │  │ vm-staging-01   │  │  │ │─ VPS Small   │  │
│  │ [+ Add Cloud]       │ │  │ [+ Provision]   │  │  │ └─ GPU Large  │  │
│  └─────────────────────┘ │  └─────────────────┘  │  └────────────────┘  │
│                          │                       │                      │
│  ┌──────────────────────┴───────────────────────┴─────────────────────┐│
│  │                     Provisioning Wizard                             ││
│  │  1. Select Cloud │ 2. Choose Specs │ 3. Configure │ 4. Deploy     ││
│  └─────────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Architecture

### Unified Provider Model

Every authenticated user is a potential provider. The distinction between "user" and "provider" becomes about **intent**, not identity:

| Mode | Purpose | Resources Visible To |
|------|---------|---------------------|
| Personal | Self-use only | Owner only |
| Marketplace | Rentable | All users |
| Hybrid | Some personal, some rentable | Owner + tenants |

### Cloud Backend Abstraction

```rust
trait CloudBackend: Send + Sync {
    async fn list_server_types(&self) -> Result<Vec<ServerType>>;
    async fn list_locations(&self) -> Result<Vec<Location>>;
    async fn list_images(&self) -> Result<Vec<Image>>;
    async fn create_server(&self, req: CreateServerRequest) -> Result<Server>;
    async fn get_server(&self, id: &str) -> Result<Server>;
    async fn start_server(&self, id: &str) -> Result<()>;
    async fn stop_server(&self, id: &str) -> Result<()>;
    async fn delete_server(&self, id: &str) -> Result<()>;
    async fn get_server_metrics(&self, id: &str) -> Result<ServerMetrics>;
}

enum BackendType {
    Hetzner,
    ProxmoxApi,
    // Future: Aws, DigitalOcean, Vultr, ...
}
```

### Credential Management

Reuse the encryption scheme from Hetzner spec:

| Backend | Credential | Encryption |
|---------|------------|------------|
| Hetzner | API token | AES-256-GCM (server decryptable) |
| Proxmox API | URL + API token | AES-256-GCM (server decryptable) |
| dc-agent | (unchanged) | N/A - agent polls API |

### Database Schema

```sql
-- Cloud account connections
CREATE TABLE cloud_accounts (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id          BYTEA NOT NULL REFERENCES accounts(pubkey),
    backend_type        TEXT NOT NULL,  -- 'hetzner', 'proxmox_api'
    name                TEXT NOT NULL,  -- User's label
    credentials_encrypted TEXT NOT NULL,
    default_location    TEXT,
    is_valid            BOOLEAN DEFAULT true,
    last_validated_at   TIMESTAMPTZ,
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    
    CONSTRAINT unique_account_backend_name UNIQUE (account_id, backend_type, name)
);

-- Self-provisioned resources
CREATE TABLE cloud_resources (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    cloud_account_id    UUID NOT NULL REFERENCES cloud_accounts(id),
    external_id         TEXT NOT NULL,  -- Backend's server ID
    name                TEXT NOT NULL,
    server_type         TEXT NOT NULL,
    location            TEXT NOT NULL,
    image               TEXT NOT NULL,
    status              TEXT NOT NULL,  -- 'provisioning', 'running', 'stopped', 'deleting'
    public_ip           TEXT,
    ssh_port            INTEGER DEFAULT 22,
    ssh_username        TEXT DEFAULT 'root',
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    
    -- Gateway routing (same pattern as dc-agent contracts)
    gateway_slug        TEXT UNIQUE,              -- e.g., "k7m2p4"
    gateway_ssh_port    INTEGER,                  -- e.g., 20001
    gateway_port_range_start INTEGER,              -- e.g., 20002
    gateway_port_range_end   INTEGER,              -- e.g., 20011
    
    -- Link to marketplace offering (if listed)
    offering_id         BIGINT REFERENCES provider_offerings(id),
    listing_mode        TEXT DEFAULT 'personal',  -- 'personal', 'marketplace'
    
    CONSTRAINT unique_external_id UNIQUE (cloud_account_id, external_id)
);

-- Provisioning lock (same pattern as dc-agent)
ALTER TABLE cloud_resources ADD COLUMN provisioning_locked_at TIMESTAMPTZ;
ALTER TABLE cloud_resources ADD COLUMN provisioning_locked_by TEXT;
```

---

## User Flows

### Flow 1: Connect Cloud Account

```
User clicks "Add Cloud Account"
    ↓
Select backend: [Hetzner] [Proxmox]
    ↓
Enter credentials:
  - Hetzner: API token
  - Proxmox: URL + API token + (optional) node name
    ↓
API validates credentials (list server types)
    ↓
Encrypt & store in cloud_accounts
    ↓
Show available regions/server types
```

### Flow 2: Self-Provision Resource

```
User clicks "Provision New Resource"
    ↓
Select cloud account (from connected accounts)
    ↓
Wizard:
  1. Location (populated from backend)
  2. Server type (populated from backend)
  3. Image/OS (populated from backend)
  4. Name, SSH key
    ↓
Create cloud_resources row (status: provisioning)
    ↓
Background service picks up, provisions via backend API
    ↓
Update status: running, fill in public_ip
    ↓
User sees resource in dashboard
```

### Flow 3: Convert to Marketplace Offering

```
User clicks "List on Marketplace" on a resource
    ↓
Offering creation form (pre-filled from resource)
    ↓
Create offering with infra_backend = 'self_provisioned'
    ↓
Link offering_id on cloud_resources
    ↓
Update listing_mode = 'marketplace'
```

---

## Backend Implementation

### Hetzner Backend

As specified in `docs/specs/2026-02-14-hetzner-provisioner.md`:

- API client in `api/src/hetzner/client.rs`
- Provisioning service polls for pending resources
- Encrypt tokens with `CREDENTIAL_ENCRYPTION_KEY`

### Proxmox API Backend

Similar to Hetzner but for user's own Proxmox cluster:

```rust
// api/src/proxmox/client.rs
pub struct ProxmoxApiClient {
    url: String,        // https://proxmox.example.com:8006
    token: String,      // API token
    node: Option<String>, // Specific node, or auto-select
}

impl CloudBackend for ProxmoxApiClient {
    // ...implementation
}
```

Key differences from dc-agent:
- **No agent**: API server calls Proxmox directly
- **User's cluster**: Not provider-hosted
- **Gateway included**: Same subdomain/TLS features as marketplace resources

Endpoints used:
| Operation | Method | Path |
|-----------|--------|------|
| List nodes | GET | `/nodes` |
| List VMs | GET | `/nodes/{node}/qemu` |
| Get VM | GET | `/nodes/{node}/qemu/{vmid}` |
| Create VM | POST | `/nodes/{node}/qemu` |
| Start VM | POST | `/nodes/{node}/qemu/{vmid}/status/start` |
| Stop VM | POST | `/nodes/{node}/qemu/{vmid}/status/stop` |
| Delete VM | DELETE | `/nodes/{node}/qemu/{vmid}` |

### Provisioning Service

Unified service for both backends:

```rust
// api/src/provisioning/service.rs
pub struct ProvisioningService {
    db: PgPool,
    backends: HashMap<BackendType, Arc<dyn CloudBackendFactory>>,
}

impl ProvisioningService {
    pub async fn run(&self) {
        loop {
            let pending = self.fetch_pending_resources().await;
            for resource in pending {
                if self.acquire_lock(&resource).await {
                    tokio::spawn(self.provision_one(resource));
                }
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    }
    
    async fn provision_one(&self, resource: CloudResource) {
        let account = self.get_cloud_account(resource.cloud_account_id).await;
        let backend = self.backends[&account.backend_type].create(&account.credentials);
        
        // Provision via backend
        let server = backend.create_server(CreateServerRequest { ... }).await?;
        
        // Update resource
        self.update_resource(resource.id, server).await;
        self.release_lock(&resource).await;
    }
}
```

### Gateway Integration

Self-provisioned resources get the same gateway features as marketplace VMs:

```
┌──────────────────────────────────────────────────────────────────┐
│                      Gateway Architecture                         │
├──────────────────────────────────────────────────────────────────┤
│                                                                   │
│   User (Browser)                                                  │
│        │                                                          │
│        ▼                                                          │
│   k7m2p4.dc-lk.dev-gw.decent-cloud.org                           │
│        │                                                          │
│        ▼                                                          │
│   ┌─────────────────┐                                             │
│   │  Caddy (TLS)    │  ← Wildcard cert via ACME-DNS              │
│   │  Port 443       │                                             │
│   └────────┬────────┘                                             │
│            │                                                      │
│   ┌────────▼────────┐                                             │
│   │  iptables NAT   │  ← Port forwarding to backend IP           │
│   │  SSH: 20000→22  │                                             │
│   │  TCP: 20001→X   │                                             │
│   └────────┬────────┘                                             │
│            │                                                      │
│            ▼                                                      │
│   ┌─────────────────┐     ┌─────────────────┐                    │
│   │  Hetzner VM     │ OR  │  Proxmox VM     │                    │
│   │  (public IP)    │     │  (user's host)  │                    │
│   └─────────────────┘     └─────────────────┘                    │
│                                                                   │
└──────────────────────────────────────────────────────────────────┘
```

**Implementation options:**

| Option | Pros | Cons |
|--------|------|------|
| **Reuse dc-agent gateway** | No new infra, existing code | Requires agent on same host (not applicable for Hetzner) |
| **Dedicated gateway servers** | Works for any backend | Additional infra cost, single point of failure |
| **Cloudflare Tunnel** | No open ports, works anywhere | Latency, external dependency |

**Recommended**: Dedicated gateway servers with Anycast IP (future). For MVP: Start with existing dc-agent gateways for Proxmox resources, add Cloudflare Tunnel for Hetzner.

**Gateway allocation flow:**
```
1. Resource provisioned → public_ip known
2. Generate unique 6-char slug → gateway_slug
3. Allocate SSH port (next available from 20000+) → gateway_ssh_port
4. Allocate port range (10 ports) → gateway_port_range_start/end
5. Create DNS record: {slug}.{dc_id}.{gw_prefix}.{domain} → gateway IP
6. Configure iptables: forward gateway_ssh_port → public_ip:22
7. Caddy auto-provisions TLS via existing wildcard cert
```

---

## UI Components

### New Pages

| Route | Purpose |
|-------|---------|
| `/dashboard/cloud-accounts` | Manage connected cloud accounts |
| `/dashboard/resources` | List self-provisioned resources |
| `/dashboard/resources/provision` | Provisioning wizard |

### New Components

| Component | Purpose |
|-----------|---------|
| `CloudAccountCard.svelte` | Display connected account, validate status |
| `CloudAccountDialog.svelte` | Add/edit cloud account |
| `ResourceCard.svelte` | Display provisioned resource with status |
| `ProvisioningWizard.svelte` | Multi-step provision flow |
| `BackendSelector.svelte` | Choose Hetzner/Proxmox/etc |
| `ServerTypeSelector.svelte` | Choose VM specs (populated from backend) |

### Modified Components

| Component | Changes |
|-----------|---------|
| `DashboardSidebar.svelte` | Add Cloud Accounts, Resources menu items |
| `OfferingsEditor.svelte` | Add "Link to Self-Provisioned Resource" option |
| `RentalRequestDialog.svelte` | Support both marketplace and self-provisioned |

---

## API Endpoints

### Cloud Accounts

```
GET    /cloud-accounts                    # List user's cloud accounts
POST   /cloud-accounts                    # Add new cloud account
GET    /cloud-accounts/:id                # Get account details
DELETE /cloud-accounts/:id                # Remove account
POST   /cloud-accounts/:id/validate       # Re-validate credentials
GET    /cloud-accounts/:id/server-types   # Get available server types
GET    /cloud-accounts/:id/locations      # Get available locations
GET    /cloud-accounts/:id/images         # Get available images
```

### Cloud Resources

```
GET    /resources                         # List user's resources
POST   /resources                         # Provision new resource
GET    /resources/:id                     # Get resource details
DELETE /resources/:id                     # Delete resource
POST   /resources/:id/start               # Start VM
POST   /resources/:id/stop                # Stop VM
POST   /resources/:id/list-on-marketplace # Convert to offering
```

---

## Migration Path

### Phase 1: Backend Foundation (Week 1-2)

1. Create `server_credential_encryption.rs` (from Hetzner spec)
2. Add `cloud_accounts` table migration
3. Create `CloudBackend` trait
4. Implement Hetzner client and backend
5. Implement Proxmox API client and backend
6. Add cloud account API endpoints
7. Write unit tests

### Phase 2: Provisioning Service (Week 2-3)

1. Add `cloud_resources` table migration
2. Create unified provisioning service
3. Implement resource API endpoints
4. Add provisioning lock mechanism
5. Implement start/stop/delete operations
6. Write integration tests

### Phase 3: UI Implementation (Week 3-4)

1. Add sidebar menu items
2. Create Cloud Accounts page
3. Create Cloud Account dialog
4. Create Resources page
5. Create Provisioning Wizard
6. Add "List on Marketplace" flow
7. E2E testing

### Phase 4: Integration (Week 4-5)

1. Link resources to offerings
2. Update marketplace to show self-provisioned
3. Implement listing mode transitions
4. Add billing for marketplace-listed resources
5. Documentation

---

## Security Considerations

### Credential Storage

- **AES-256-GCM** encryption with `CREDENTIAL_ENCRYPTION_KEY`
- Key rotation requires re-encrypting all credentials
- Tokens never logged or exposed to frontend after storage

### API Access Control

- Users can only access their own cloud accounts and resources
- Validation endpoints don't expose full credentials
- Rate limiting on provision/delete operations

### Backend Isolation

- Each backend call uses fresh credential decryption
- No credential caching in memory
- Failed auth invalidates cloud account (`is_valid = false`)

---

## Decisions Made

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Billing | **Free (BYO)** | Users pay Hetzner/Proxmox directly. No platform fee for self-provisioned. Marketplace billing unchanged. |
| Gateway | **Included** | All resources get subdomain, SSH port allocation, TLS. Unified experience. |

---

## Open Questions

1. **Resource quotas**: Should we limit how many resources a user can provision? (Suggested: 10 per cloud account initially)

2. **Shared resources**: Can multiple users share one cloud account (team feature)? (Out of scope for MVP)

3. **Proxmox templates**: How do we handle OS templates for Proxmox API backend? Require user to pre-create, or auto-upload? (Suggested: require pre-created for MVP)

4. **Gateway IP allocation**: For self-provisioned resources, do we need a dedicated gateway server, or can we reuse existing dc-agent gateways? (Suggested: new gateway type "api-gateway" that doesn't require agent)

---

## Success Metrics

| Metric | Target |
|--------|--------|
| Time to provision (click to running) | < 60 seconds (Hetzner), < 120 seconds (Proxmox) |
| Self-service onboarding | < 5 minutes from signup to first VM |
| Marketplace conversion | 20% of self-provisioned resources listed within 30 days |
