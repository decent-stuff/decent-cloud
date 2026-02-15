# Hetzner Provisioner

Server-side infrastructure backend for Decent Recipes. The API server provisions and terminates VMs on an author's Hetzner account via the Hetzner Cloud API.

## Scope

This spec covers only the Hetzner backend. It does not change Proxmox provisioning (dc-agent), billing, or the marketplace.

## How It Fits

```
Decent Recipes spec (BYOC-SPEC.md)
├── Infrastructure backends
│   ├── Proxmox — dc-agent (exists, unchanged)
│   └── Hetzner — this spec
└── Recipes — shell scripts (backend-agnostic)
```

The Hetzner provisioner fulfills the infrastructure backend contract: given an offering's VM specs and a buyer's SSH pubkey, produce a VM with a public IP and root SSH access. Then hand off to recipe execution.

---

## Hetzner Cloud API

Base URL: `https://api.hetzner.cloud/v1`

Auth: `Authorization: Bearer <token>` header on every request.

### Endpoints used

| Operation | Method | Path | Purpose |
|-----------|--------|------|---------|
| Create SSH key | POST | `/ssh_keys` | Register buyer's pubkey |
| Delete SSH key | DELETE | `/ssh_keys/{id}` | Cleanup on termination |
| Create server | POST | `/servers` | Provision VM |
| Get server | GET | `/servers/{id}` | Poll status, get IP |
| Delete server | DELETE | `/servers/{id}` | Terminate VM |
| List server types | GET | `/server_types` | Validate offering config |
| List locations | GET | `/locations` | Validate offering config |
| List images | GET | `/images?type=system` | Validate offering config |

### Server creation request

```json
{
  "name": "dc-{contract_id_short}",
  "server_type": "cx22",
  "location": "fsn1",
  "image": "ubuntu-22.04",
  "ssh_keys": ["{ssh_key_id}"],
  "start_after_create": true
}
```

### Response handling

- **201**: Created. Response contains `server.id`, `server.public_net.ipv4.ip`.
- **402**: Payment required (insufficient Hetzner balance). Fail loudly.
- **403**: Token invalid or revoked. Fail loudly.
- **409**: Name conflict (retry with different suffix).
- **422**: Invalid params (bad server_type, location, image). Fail loudly.
- **429**: Rate limited. Retry after `Retry-After` header.
- **5xx**: Hetzner outage. Retry with backoff.

---

## Credential Storage

### New encryption scheme

The existing credential encryption (X25519 + XChaCha20Poly1305) is for client-side E2EE where the server can't read the data. Hetzner tokens need to be decryptable by the server during provisioning. Different threat model, different scheme.

| | Existing (VM credentials) | Hetzner tokens |
|---|---|---|
| **Encrypts** | Client (browser) | Server |
| **Decrypts** | Client (browser) | Server |
| **Key** | User's Ed25519 keypair | `CREDENTIAL_ENCRYPTION_KEY` env var |
| **Algorithm** | XChaCha20Poly1305 | AES-256-GCM |
| **Server can read** | No | Yes (by design) |

### Implementation

New module: `api/src/crypto/server_credential_encryption.rs`

```
encrypt(plaintext, key) → nonce || ciphertext || tag
decrypt(blob, key) → plaintext
```

- Key: 32 bytes from `CREDENTIAL_ENCRYPTION_KEY` (hex-encoded env var, fail on startup if missing/malformed when Hetzner features are enabled)
- Nonce: 12 random bytes per encryption (AES-256-GCM standard)
- Storage format: `base64(version_byte || nonce || ciphertext || tag)`
- Version byte: `0x01` (for future algorithm changes)

No key-per-token wrapping. No key derivation. One symmetric key for all tokens. Key rotation = re-encrypt all rows (migration script).

### Database

New column on `providers` (or `provider_credentials`) table:

```sql
ALTER TABLE providers ADD COLUMN hetzner_token_encrypted TEXT;
```

Single column. One Hetzner token per provider. If we support multiple tokens per provider later, that's a schema change then — not now.

---

## Provisioning Service

Background service in api-server, following the existing `CleanupService` pattern.

### Polling loop

```
every 10s:
  SELECT contracts WHERE status = 'accepted'
    AND offering.infra_backend = 'hetzner'
    AND provisioning_lock IS NULL
  ORDER BY created_at ASC
  LIMIT 5

  for each contract:
    acquire provisioning lock (row-level, with expiry)
    spawn tokio task → provision_one(contract)
```

Why polling (not event-driven): matches existing dc-agent pattern, survives api-server restarts, no message broker dependency.

### provision_one(contract)

```
1. Decrypt author's Hetzner token
2. Create SSH key on Hetzner
   - name: "dc-{contract_id_short}"
   - public_key: buyer's SSH pubkey from contract
   → save ssh_key_id
3. Create server on Hetzner
   - name: "dc-{contract_id_short}"
   - server_type, location, image from offering's infra_config
   - ssh_keys: [ssh_key_id]
   → save server_id, poll until status = "running"
4. Get server public IP from response
5. Wait for SSH reachable (port 22, timeout 120s, retry every 5s)
6. Execute recipe script via SSH
   - Reuse post_provision.rs logic (shebang detection, timeout 300s)
   - Script is read from contract (snapshotted at creation)
7. Record on contract:
   - instance IP
   - external_id = "hetzner:{server_id}"
   - hetzner_ssh_key_id (for cleanup)
   - connection instructions
8. Transition contract → active
9. Drop plaintext token from memory
```

### Error handling during provisioning

Every step can fail. The rule: **clean up what was created, then fail the contract.**

| Step fails at | Cleanup | Contract state |
|---------------|---------|----------------|
| Decrypt token | Nothing to clean | `failed` |
| Create SSH key | Nothing to clean | `failed` |
| Create server | Delete SSH key | `failed` |
| Wait for SSH | Delete server + SSH key | `failed` |
| Recipe script | Delete server + SSH key | `failed` |
| Record/transition | Server exists, contract active — no cleanup needed | `active` |

On any failure: log error with full context, notify author (email/Telegram), release provisioning lock.

If cleanup itself fails (e.g., can't delete server after script failure): log error, notify author with server ID for manual cleanup. Do not retry cleanup indefinitely.

---

## Termination Service

Separate polling loop (or same service, different query):

```
every 60s:
  SELECT contracts WHERE status IN ('cancelled', 'expired', 'payment_failed')
    AND offering.infra_backend = 'hetzner'
    AND external_id IS NOT NULL
    AND terminated_at IS NULL
    AND termination_lock IS NULL
```

### terminate_one(contract)

```
1. Decrypt author's Hetzner token
2. Parse external_id → hetzner server_id
3. DELETE /servers/{server_id}
   - 200: success
   - 404: already deleted (idempotent, continue)
4. DELETE /ssh_keys/{ssh_key_id}
   - 200: success
   - 404: already deleted (idempotent, continue)
5. Record terminated_at on contract
6. Drop plaintext token
```

### Retry on failure

- 3 attempts with exponential backoff: 1min, 5min, 30min
- After 3 failures: mark contract `termination_failed`, alert author with server ID
- Idempotent: if server already deleted (404), treat as success

---

## Hetzner API Client

New module: `api/src/hetzner/client.rs`

Thin HTTP wrapper. No business logic.

```rust
pub struct HetznerClient {
    http: reqwest::Client,
    token: String,  // plaintext, short-lived in memory
}

impl HetznerClient {
    pub fn new(token: String) -> Self;
    pub async fn create_ssh_key(&self, name: &str, public_key: &str) -> Result<SshKey>;
    pub async fn delete_ssh_key(&self, id: i64) -> Result<()>;
    pub async fn create_server(&self, req: CreateServerRequest) -> Result<Server>;
    pub async fn get_server(&self, id: i64) -> Result<Server>;
    pub async fn delete_server(&self, id: i64) -> Result<()>;
    pub async fn list_server_types(&self) -> Result<Vec<ServerType>>;
    pub async fn list_locations(&self) -> Result<Vec<Location>>;
    pub async fn list_images(&self) -> Result<Vec<Image>>;
}
```

- `HetznerClient` is created per-operation (provision or terminate), dropped after
- All methods return `Result<T>` — no silent failures, no retries inside the client
- Rate limit handling: caller retries, client surfaces the 429 + Retry-After
- Timeout per request: 30s

### Response types

Minimal structs matching only the fields we use:

```rust
pub struct Server {
    pub id: i64,
    pub name: String,
    pub status: String,  // "initializing", "running", "off", ...
    pub public_net: PublicNet,
}

pub struct PublicNet {
    pub ipv4: IpAddress,
}

pub struct IpAddress {
    pub ip: String,
}

pub struct SshKey {
    pub id: i64,
    pub name: String,
}

pub struct ServerType {
    pub id: i64,
    pub name: String,
    pub cores: u32,
    pub memory: f64,
    pub disk: u32,
}
```

---

## Offering Validation

When an author creates a Hetzner offering, the API server validates `infra_config` against the live Hetzner catalog:

```json
{
  "server_type": "cx22",
  "location": "fsn1",
  "image": "ubuntu-22.04"
}
```

Validation calls (using the author's token):
1. `GET /server_types` — verify `server_type` exists
2. `GET /locations` — verify `location` exists
3. `GET /images?type=system` — verify `image` exists

Fail offering creation if any value is invalid. Cache catalog responses for 1 hour (per-author, in memory) to avoid hammering the API.

---

## Database Changes

### New columns

```sql
-- Author's encrypted Hetzner token
ALTER TABLE providers ADD COLUMN hetzner_token_encrypted TEXT;

-- Hetzner-specific instance tracking on contracts
ALTER TABLE contract_provisioning_details
  ADD COLUMN hetzner_server_id BIGINT,
  ADD COLUMN hetzner_ssh_key_id BIGINT,
  ADD COLUMN terminated_at TIMESTAMPTZ;
```

### Provisioning lock

```sql
ALTER TABLE contracts
  ADD COLUMN provisioning_locked_at TIMESTAMPTZ,
  ADD COLUMN provisioning_locked_by TEXT;  -- api-server instance ID
```

Lock expiry: 10 minutes (if api-server crashes mid-provision, lock auto-expires and another instance picks it up).

---

## Configuration

| Env var | Required | Purpose |
|---------|----------|---------|
| `CREDENTIAL_ENCRYPTION_KEY` | Yes (if Hetzner enabled) | 32-byte hex key for AES-256-GCM |
| `HETZNER_PROVISIONING_ENABLED` | No (default: false) | Feature gate for the provisioning service |
| `HETZNER_POLL_INTERVAL_SECS` | No (default: 10) | Polling interval for new contracts |
| `HETZNER_TERMINATION_POLL_INTERVAL_SECS` | No (default: 60) | Polling interval for terminations |

On startup, if `HETZNER_PROVISIONING_ENABLED=true` and `CREDENTIAL_ENCRYPTION_KEY` is missing or malformed: **fail to start**. Do not silently disable.

If `HETZNER_PROVISIONING_ENABLED` is not set: log `warn!("HETZNER_PROVISIONING_ENABLED not set — Hetzner provisioning will NOT work. Set HETZNER_PROVISIONING_ENABLED=true to enable.")`.

---

## Files to Create

| File | Purpose |
|------|---------|
| `api/src/hetzner/mod.rs` | Module root |
| `api/src/hetzner/client.rs` | Hetzner API HTTP client |
| `api/src/hetzner/provisioner.rs` | Provisioning + termination service |
| `api/src/crypto/server_credential_encryption.rs` | AES-256-GCM encrypt/decrypt |
| Migration file | Schema changes |

## Files to Modify

| File | Change |
|------|--------|
| `api/src/main.rs` | Spawn Hetzner provisioning service |
| `api/src/lib.rs` or `api/src/mod.rs` | Register `hetzner` module |
| Offering creation endpoint | Add `infra_config` validation for Hetzner |
| Contract creation endpoint | Snapshot recipe script onto contract |

## What This Spec Does NOT Cover

- Marketplace UI changes (badges, filtering by backend)
- Billing / Stripe integration
- Author onboarding UI
- Proxmox provisioning changes (none needed)
- Recipe script format or execution (reuses existing `post_provision.rs`)
