# Decent Recipes

## What

A marketplace for deployment recipes. Authors write automation scripts that turn a bare VM into a working service (OpenClaw, vLLM, WordPress, anything). The platform handles infrastructure provisioning and billing. Buyers pay a flat monthly price and get a running service — no devops required.

## Why

Today the platform requires providers to run dc-agent on their own Proxmox hardware. This limits supply to technical infrastructure operators. Decent Recipes separates two concerns that are currently coupled:

1. **Infrastructure** — provisioning a VM with SSH access (Proxmox via dc-agent, Hetzner via cloud API, others in the future)
2. **Recipe** — what runs on top of that VM (a shell script that assumes root + SSH)

This separation means: anyone who can write a deployment script can become an author. The same recipe can run on any supported infrastructure backend. And buyers don't need to know or care what's underneath.

## Architecture

```
┌─────────────────────────────────────────────────┐
│                   Offering                       │
│  = Infrastructure Backend + Recipe + Price       │
├─────────────────────────────────────────────────┤
│                                                  │
│  ┌─────────────┐         ┌────────────────────┐ │
│  │  Recipe      │         │  Infra Backend     │ │
│  │  (shell      │   runs  │                    │ │
│  │   script)    │───on───▶│  VM with SSH +     │ │
│  │              │         │  root access       │ │
│  └─────────────┘         └────────────────────┘ │
│                                 ▲                │
│                    ┌────────────┴────────────┐   │
│                    │                         │   │
│              ┌─────┴─────┐           ┌──────┴──┐│
│              │  Hetzner   │           │ Proxmox ││
│              │  (API)     │           │(dc-agent)││
│              └───────────┘           └─────────┘│
└─────────────────────────────────────────────────┘
```

### Infrastructure Backend Contract

Every infrastructure backend must produce the same thing: **a VM with a public IP (or gateway route) and root SSH access**. How it gets there is the backend's business.

| Backend | Provisioner | Access model | Who runs it |
|---------|------------|--------------|-------------|
| **Hetzner** | API server calls Hetzner API directly | Public IP, direct SSH | Platform (server-side) |
| **Proxmox** | dc-agent on provider's host | Gateway subdomain + port mapping | Provider (dc-agent) |

The recipe script doesn't know or care which backend provisioned the VM. It receives a root shell over SSH and does its work.

## Roles

**Recipe Author** — Writes deployment scripts. May also be the infrastructure provider (owns Hetzner account or runs dc-agent on Proxmox). Sets price to cover infra costs + margin.

**Infrastructure Provider** — Provides compute capacity via a supported backend. For Hetzner: stores an API token. For Proxmox: runs dc-agent. An author and provider can be the same person (typical) or different entities (future: author writes recipe, multiple providers offer it on their infra).

**Buyer** — Pays a flat monthly price, gets a running service. No cloud account, no devops, no SSH required (though SSH access is available).

**Platform** — Marketplace, billing, provisioning orchestration, credential security.

---

## User Flows

### Author: Create and Publish a Recipe

1. **Register as provider**: Author provides infrastructure credentials — Hetzner API token (encrypted at rest, AES-256-GCM via `CREDENTIAL_ENCRYPTION_KEY`) or sets up dc-agent on Proxmox (existing flow).

2. **Create offering**: Author goes to "My Offerings", creates a new recipe offering. Specifies:
   - Infrastructure backend (Hetzner or Proxmox)
   - VM specs: server type, location, OS image (for Hetzner: selected from live catalog; for Proxmox: from available templates)
   - Recipe script (shell script that runs after VM boots)
   - Monthly price (must cover infra cost + author's margin)
   - Description

3. **Test deploy**: Author rents their own offering (self-rental = free). Platform provisions a VM and runs the recipe. Author SSHes in, verifies everything works, iterates. This is the only validation mechanism.

4. **Publish**: Author sets visibility to "public". It appears in the marketplace.

### Buyer: Rent a Recipe

1. **Discover**: Buyer browses marketplace. Each offering shows: what it deploys, monthly price, VM specs. Infrastructure backend is shown but buyer doesn't need to interact with it.

2. **Purchase**: Buyer clicks "Rent", provides SSH public key, completes Stripe checkout for recurring monthly billing.

3. **Automatic provisioning**: Payment triggers provisioning immediately. The platform provisions a VM on the appropriate backend and runs the recipe script. No manual steps.

4. **Use**: Buyer sees connection details (IP/hostname, SSH command, any service URLs). Contract is `active`.

5. **Cancel**: Buyer cancels the subscription. VM stays active until the current billing period expires, then the platform terminates it automatically.

---

## Recipes

A recipe is a shell script that turns a bare VM into a working service. It runs via SSH as root after the VM boots.

### Contract

The recipe assumes:
- Root SSH access to a freshly provisioned VM
- A supported base OS (Ubuntu 22.04+ by default)
- Internet connectivity

The recipe does NOT assume:
- Any specific cloud provider
- Any pre-installed software beyond the base OS
- Any specific networking setup (use the VM's public IP or gateway route as given)

### Snapshotting

The recipe script is copied from the offering onto the contract at creation time. If the author later edits the script, existing contracts are unaffected — only new purchases get the updated version.

### Example

```bash
#!/bin/bash
set -euo pipefail

# Install runtime
apt-get update && apt-get install -y docker.io docker-compose

# Deploy service
git clone https://github.com/example/openclaw.git /opt/openclaw
cd /opt/openclaw && docker-compose up -d

# Verify
curl --retry 10 --retry-delay 5 http://localhost:8080/health
```

---

## Credential Security

**Scope**: Applies to cloud API tokens (Hetzner). Proxmox credentials are managed by dc-agent locally on the host — they never reach the platform.

**Encryption at rest**: Hetzner API tokens are encrypted server-side using AES-256-GCM with a key derived from `CREDENTIAL_ENCRYPTION_KEY`. The encrypted blob is stored in the database.

**Decryption**: Tokens are decrypted in memory only during provisioning and termination. Once the API call completes (or fails), the plaintext is dropped. Never logged or written to disk.

**Access control**: Only the provisioning/termination code path can request decryption. No API endpoint exposes the plaintext token to any user, including the author who stored it.

**Key rotation**: Rotating `CREDENTIAL_ENCRYPTION_KEY` requires re-encrypting all stored tokens — a migration script handles this.

---

## Offerings

An offering bundles an infrastructure backend configuration with a recipe:

| Field | Description |
|-------|-------------|
| `infra_backend` | Infrastructure backend: `hetzner` or `proxmox` |
| `infra_config` | Backend-specific VM config (server type, location, image, template) |
| `recipe_script` | Shell script that runs after VM boot |
| `monthly_price` / `currency` | What the buyer pays |
| `author_id` | Author who created the offering (and whose credentials are used for Hetzner) |
| `visibility` | `public` / `shared` / `private` |

Existing fields (`provisioner_type`, `provisioner_config`, `post_provision_script`, hardware specs) map directly to this model.

### Marketplace Visibility

- **Hetzner offerings**: No agent required. Marketplace checks that the author's token is valid.
- **Proxmox offerings**: Requires dc-agent heartbeat (existing behavior).

Both types appear in the same marketplace, distinguished by a badge showing the infrastructure backend.

---

## Provisioning

### Flow by backend

**Hetzner** (API server provisions directly):

1. Buyer completes Stripe checkout. Contract created in `accepted` state.
2. Recipe script snapshotted onto contract.
3. Background task:
   a. Decrypts author's Hetzner token in memory.
   b. Creates SSH key on Hetzner (buyer's pubkey).
   c. Creates server with offering specs.
   d. Waits for SSH-reachable.
   e. Executes recipe script via SSH.
   f. Records instance details (IP, Hetzner server ID).
   g. Transitions contract to `active`.
4. Plaintext token dropped from memory.

**Proxmox** (dc-agent provisions):

1. Buyer completes Stripe checkout. Contract created in `accepted` state.
2. Recipe script snapshotted onto contract.
3. dc-agent picks up the contract (existing polling mechanism).
4. dc-agent provisions VM, configures gateway routing, runs recipe script.
5. dc-agent reports back, contract transitions to `active`.

Same buyer experience. Same contract lifecycle. Different provisioning path.

### Termination

**Hetzner**: Platform decrypts token, deletes server + SSH key via API, transitions to `terminated`.

**Proxmox**: dc-agent picks up cancellation, destroys VM, cleans up gateway routes (existing flow).

Both are fully automatic — buyer takes no action.

---

## Billing

**Recurring monthly**: Flat price via Stripe. Automatic charges each period.

**Cancellation**: VM stays active until billing period ends, then auto-terminated.

**Payment failure**: Stripe retries per its schedule. After retries exhausted, contract moves to `payment_failed`, VM terminated.

**Self-rental**: Free for authors testing their own recipes. No Stripe subscription created.

## Revenue Model

| Recipient | Share | Example ($10/mo) |
|-----------|-------|-------------------|
| Author | `author_commission_percent` (default 80%) | $8.00 |
| Platform | remainder | $2.00 |

- Commissions tracked per contract (`author_commission_e9s`, `platform_fee_e9s`)
- Payouts to authors: tracked but manual for MVP
- Authors set price to cover their infra costs + margin. Platform does not track provider-side spending.

---

## Failure Modes

| Failure | Behavior |
|---------|----------|
| **Hetzner token invalid/expired** | Provisioning fails. Contract → `failed`. Author notified: "Your Hetzner token is invalid. Update it to resume." Offering pulled from marketplace until token is valid. |
| **Insufficient Hetzner balance** | Hetzner returns 402. Contract → `failed`. Author notified: "Hetzner rejected server creation — check billing." |
| **dc-agent offline** | Proxmox offering: contract stays in `accepted` until agent comes back (existing behavior). Marketplace hides offerings with stale heartbeats. |
| **Recipe script failure** | Script returned non-zero. Contract → `failed`. VM terminated (cleanup). Author notified with exit code + stderr. |
| **Termination failure** | Retry with exponential backoff (3 attempts, 1 hour). If all fail, alert author: "VM [id] could not be deleted — manual cleanup required." Contract → `termination_failed`. |
| **Cloud API timeout/5xx** | Retry with exponential backoff. If persistent, treat as termination failure — alert author. |

All failure notifications include actionable details: server ID, contract ID, error message.

---

## What's NOT in Scope (MVP)

- **Additional cloud providers**: Only Hetzner + Proxmox. Architecture supports more via the backend abstraction.
- **Cross-provider recipes**: A recipe is tied to one offering with one backend. No "deploy this recipe on any backend" selector for buyers.
- **Server lifecycle actions**: No reboot, rebuild, resize, snapshot. Create and delete only.
- **Automated payouts**: Commissions tracked, payouts manual.
- **Recipe templates**: No starter templates. Authors write their own scripts.
- **Script validation**: No platform-side validation. Authors test via self-rental.
