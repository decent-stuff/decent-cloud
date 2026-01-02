# DC Gateway: Per-Host Reverse Proxy Architecture

**Status:** Implemented
**Created:** 2025-12-31
**Updated:** 2026-01-01

## Problem Statement

Public IPv4 addresses are scarce and expensive (~$1.61 USD/month each). A typical datacenter with 5+ Proxmox servers can run 100-200+ VMs, but may only have 24 IPv4 addresses available. Assigning a dedicated public IP to each VM is not economically viable.

## Overview

Deploy a reverse proxy on each Proxmox host alongside dc-agent. Each host gets one public IPv4.

**Routing architecture:**
- **HTTP/HTTPS**: Traefik with SNI-based routing and wildcard TLS certificate
- **TCP/UDP**: iptables DNAT for port forwarding (SSH, databases, game servers)

**DNS architecture:**
- **Wildcard cert**: Traefik uses Cloudflare API token directly (stored in `/etc/traefik/env`)
- **Per-VM records**: dc-agent calls central API, which proxies to Cloudflare (agent never has CF credentials)

dc-agent manages Traefik config and iptables rules as part of VM provisioning lifecycle.

```
                         Internet
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
   203.0.113.1          203.0.113.2          203.0.113.N
   ┌──────────┐         ┌──────────┐         ┌──────────┐
   │ Proxmox 1│         │ Proxmox 2│         │ Proxmox N│
   │          │         │          │         │          │
   │ traefik  │         │ traefik  │         │ traefik  │
   │ dc-agent │         │ dc-agent │         │ dc-agent │
   │          │         │          │         │          │
   │ ┌──────┐ │         │ ┌──────┐ │         │ ┌──────┐ │
   │ │ VMs  │ │         │ │ VMs  │ │         │ │ VMs  │ │
   │ └──────┘ │         │ └──────┘ │         │ └──────┘ │
   └──────────┘         └──────────┘         └──────────┘

DNS (dynamic, per-VM):
  k7m2p4.dc-lk.decent-cloud.org → 203.0.113.1
  x9f3a2.dc-lk.decent-cloud.org → 203.0.113.2
```

## Requirements

### Must-have
- [x] Traefik running as systemd service on each Proxmox host (`dc-agent setup gateway` automates this)
- [x] Wildcard TLS certificate for `*.{dc}.decent-cloud.org` via Let's Encrypt DNS-01 (automated via Traefik)
- [x] dc-agent writes Traefik dynamic config on VM provision/destroy
- [x] dc-agent updates Cloudflare DNS on VM provision/destroy
- [x] HTTP/HTTPS routing via subdomain (SNI)
- [x] TCP port mapping for SSH and custom services
- [x] UDP port mapping for game servers and similar
- [x] Port range allocation per VM (default: 10 ports)
- [x] Cloudflare Zone ID auto-lookup from domain name

### Nice-to-have
- [ ] Custom domain support (user brings their own domain)
- [x] Per-VM bandwidth monitoring (via iptables accounting)
- [ ] Rate limiting per VM
- [ ] Premium tier: dedicated public IPv4 for specific VMs

## Technical Design

### Network Architecture

**Provider setup:**
- Provider has BGP peering with upstream (standard DC practice)
- Provider routes one IPv4 per Proxmox host
- VMs use private IPs (e.g., 10.0.0.0/16) internally
- Traefik on host receives public traffic, proxies to VMs

**Traffic flow:**
```
User HTTPS request
    │
    ▼ DNS: k7m2p4.dc-lk.decent-cloud.org → 203.0.113.1
┌─────────────────────────────────────────────────────┐
│ Proxmox Host (203.0.113.1)                          │
│                                                     │
│   Traefik (:443)                                    │
│      │                                              │
│      │ SNI match: k7m2p4.dc-lk.decent-cloud.org     │
│      │ TLS termination (wildcard cert)              │
│      ▼                                              │
│   Proxy to 10.0.1.5:80                              │
│      │                                              │
│      ▼                                              │
│   ┌─────────┐                                       │
│   │   VM    │                                       │
│   │10.0.1.5 │                                       │
│   └─────────┘                                       │
└─────────────────────────────────────────────────────┘
```

### DNS Configuration

**Zone:** `decent-cloud.org` (managed via Cloudflare)

**Static records:**
```
dc-lk.decent-cloud.org    A    203.0.113.1    ; Optional: points to first host
```

**Dynamic records (created per-VM by dc-agent):**
```
k7m2p4.dc-lk    A    203.0.113.1    ; VM on host 1
x9f3a2.dc-lk    A    203.0.113.2    ; VM on host 2
```

**Wildcard for TLS:**
- Certificate covers `*.dc-lk.decent-cloud.org`
- Each DC gets its own wildcard cert
- DNS-01 challenge via Cloudflare API

### Subdomain Format

**Pattern:** `{slug}.{dc}.decent-cloud.org`

| Component | Format | Example |
|-----------|--------|---------|
| slug | 6-char alphanumeric, lowercase | `k7m2p4` |
| dc | datacenter identifier | `dc-lk` (Sri Lanka) |
| domain | base domain | `decent-cloud.org` |

**Full example:** `k7m2p4.dc-lk.decent-cloud.org`

**Slug generation:**
- Generated at VM provisioning time
- 6 characters: `[a-z0-9]{6}`
- 2.1 billion combinations (36^6), collision-resistant
- Stored in database: `contract_sign_requests.gateway_slug`

### Port Allocation

**Range:** 20000-59999 (40,000 ports per host)

**Allocation scheme:** 10 ports per VM (configurable per offering)

```
VM 1: 20000-20009
  - 20000: SSH (mapped from VM:22)
  - 20001-20009: available for user services

VM 2: 20010-20019
VM 3: 20020-20029
...
```

**Capacity:** 4,000 VMs per host (40,000 / 10) - far exceeds realistic density

**Tracking:** dc-agent maintains local state file with allocated ranges

**Port allocation file:** `/var/lib/dc-agent/port-allocations.json`
```json
{
  "next_base": 20030,
  "allocations": {
    "k7m2p4": { "base": 20000, "count": 10, "contract_id": "..." },
    "x9f3a2": { "base": 20010, "count": 10, "contract_id": "..." }
  }
}
```

### Traefik Configuration (HTTP/HTTPS Only)

Traefik handles HTTP/HTTPS routing with SNI-based routing and TLS termination.
TCP/UDP port forwarding is handled by iptables DNAT (see below).

**Static config:** `/etc/traefik/traefik.yaml`

```yaml
global:
  checkNewVersion: false
  sendAnonymousUsage: false

log:
  level: INFO
  filePath: /var/log/traefik/traefik.log

api:
  dashboard: false  # Enable only if needed for debugging

entryPoints:
  web:
    address: ":80"
    http:
      redirections:
        entryPoint:
          to: websecure
          scheme: https

  websecure:
    address: ":443"
    http:
      tls:
        certResolver: letsencrypt
        domains:
          - main: "dc-lk.decent-cloud.org"
            sans:
              - "*.dc-lk.decent-cloud.org"

certificatesResolvers:
  letsencrypt:
    acme:
      email: admin@decent-cloud.org
      storage: /var/lib/traefik/acme.json
      dnsChallenge:
        provider: cloudflare
        resolvers:
          - "1.1.1.1:53"
          - "8.8.8.8:53"

providers:
  file:
    directory: /etc/traefik/dynamic
    watch: true
```

**Dynamic config (per-VM):** `/etc/traefik/dynamic/vm-k7m2p4.yaml`

```yaml
# Generated by dc-agent for VM k7m2p4
# Contract: c_abc123...
# Created: 2025-12-31T10:30:00Z
#
# HTTP/HTTPS: Handled by Traefik (this file)
# TCP/UDP ports 20000-20009: Handled by iptables DNAT

http:
  routers:
    k7m2p4-http:
      rule: "Host(`k7m2p4.dc-lk.decent-cloud.org`)"
      service: k7m2p4-http
      entryPoints:
        - websecure
      tls:
        certResolver: letsencrypt

  services:
    k7m2p4-http:
      loadBalancer:
        servers:
          - url: "http://10.0.1.5:80"
```

### iptables DNAT (TCP/UDP Port Forwarding)

TCP/UDP port forwarding uses kernel-level iptables DNAT rules. This is more efficient
than Traefik for raw TCP/UDP since there's no userspace proxy.

**Port mapping scheme:**
```
Base port (e.g., 20000):
  - +0: SSH (external:20000 → VM:22)
  - +1 to +4: TCP (external:20001-20004 → VM:10001-10004)
  - +5 to +9: UDP (external:20005-20009 → VM:10005-10009)
```

**iptables rules (auto-generated by dc-agent):**
```bash
# NAT chain for gateway rules
iptables -t nat -N DC_GATEWAY
iptables -t nat -I PREROUTING -j DC_GATEWAY

# Per-VM rules (example for slug k7m2p4)
iptables -t nat -A DC_GATEWAY -p tcp --dport 20000 -j DNAT --to-destination 10.0.1.5:22 -m comment --comment "DC_VM_k7m2p4_20000"
iptables -t nat -A DC_GATEWAY -p tcp --dport 20001 -j DNAT --to-destination 10.0.1.5:10001 -m comment --comment "DC_VM_k7m2p4_20001"
iptables -t nat -A DC_GATEWAY -p udp --dport 20005 -j DNAT --to-destination 10.0.1.5:10005 -m comment --comment "DC_VM_k7m2p4_20005"
```

**Why iptables instead of Traefik for TCP/UDP:**
- Traefik requires static entrypoint definitions (can't dynamically add ports)
- iptables DNAT is kernel-level with zero userspace overhead
- Rules are isolated per-VM using comments for easy cleanup

### dc-agent Integration

**Configuration in dc-agent.toml:**

```toml
[gateway]
datacenter = "dc-lk"
domain = "decent-cloud.org"
public_ip = "203.0.113.1"  # This host's public IP

# Port allocation
port_range_start = 20000
port_range_end = 59999
ports_per_vm = 10

# Traefik integration
traefik_dynamic_dir = "/etc/traefik/dynamic"
port_allocations_path = "/var/lib/dc-agent/port-allocations.json"

# DNS management is handled via central API (no Cloudflare credentials needed)
```

**Note:** Cloudflare credentials are NOT stored in dc-agent config. The dc-agent calls the central API (`POST /api/v1/agents/dns`) which proxies DNS operations to Cloudflare. This is more secure since individual hosts don't need Cloudflare access.

**Traefik environment (for wildcard cert only):** `/etc/traefik/env`
```bash
CF_API_EMAIL=admin@decent-cloud.org
CF_API_TOKEN=<token>  # Only needs DNS edit for ACME challenges
```

**Provisioning flow changes:**

```
1. VM Provisioned (existing flow)
   └── dc-agent creates VM on Proxmox
   └── VM gets internal IP (e.g., 10.0.1.5)

2. Gateway Setup (new)
   ├── Generate slug: k7m2p4
   ├── Allocate port range: 20000-20009
   ├── Write Traefik config: /etc/traefik/dynamic/vm-k7m2p4.yaml (HTTP/HTTPS only)
   ├── Traefik auto-reloads (watches directory)
   ├── Setup iptables DNAT rules (TCP/UDP port forwarding)
   └── Call API: POST /api/v1/agents/dns → API creates Cloudflare A record

3. Report to API (modified)
   └── Include in provisioned response:
       - gateway_slug: k7m2p4
       - gateway_subdomain: k7m2p4.dc-lk.decent-cloud.org
       - ssh_port: 20000
       - port_range: 20000-20009
```

**Destroy flow:**

```
1. VM Termination (existing)
   └── dc-agent destroys VM on Proxmox

2. Gateway Cleanup (new)
   ├── Delete Traefik config: rm /etc/traefik/dynamic/vm-k7m2p4.yaml
   ├── Traefik auto-reloads
   ├── Remove iptables DNAT rules for this slug
   ├── Call API: POST /api/v1/agents/dns (action: delete) → API deletes Cloudflare record
   └── Free port range in allocation file
```

### Cloudflare DNS Integration

DNS management is centralized in the API server for security. Individual dc-agent hosts never have Cloudflare credentials (except for Traefik's ACME certificate).

**Architecture:**
```
dc-agent                         Central API                      Cloudflare
   │                                 │                                │
   │ POST /api/v1/agents/dns         │                                │
   │ { action: "create",             │                                │
   │   slug: "k7m2p4",               │                                │
   │   datacenter: "dc-lk",          │                                │
   │   publicIp: "203.0.113.1" }     │                                │
   │ ──────────────────────────────▶ │                                │
   │                                 │ POST /zones/{zone}/dns_records │
   │                                 │ ──────────────────────────────▶ │
   │                                 │                                │
   │                                 │ ◀────────────────────────────── │
   │ ◀────────────────────────────── │                                │
   │ { subdomain: "k7m2p4.dc-lk..." }│                                │
```

**API server config (environment variables):**
```bash
CF_API_TOKEN=...     # Cloudflare API token with Zone.DNS edit permission
CF_ZONE_ID=...       # Zone ID for decent-cloud.org
CF_DOMAIN=decent-cloud.org  # Optional, defaults to decent-cloud.org
```

**Agent endpoint:** `POST /api/v1/agents/dns`
- Requires agent authentication (DnsManage permission)
- Validates slug format (6 lowercase alphanumeric)
- Validates datacenter (1-20 chars)
- Creates/deletes A record via Cloudflare API

**Security benefits:**
- Cloudflare token never leaves central API server
- Compromised host cannot hijack other subdomains
- Audit trail of DNS changes in API logs

### systemd Services

**dc-agent service:** `/etc/systemd/system/dc-agent.service`

```ini
[Unit]
Description=Decent Cloud Agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/dc-agent run
Restart=always
RestartSec=5
User=dc-agent
Group=dc-agent

# Environment
EnvironmentFile=/etc/dc-agent/env

# Hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/var/lib/dc-agent /etc/traefik/dynamic

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=dc-agent

[Install]
WantedBy=multi-user.target
```

**Traefik service:** `/etc/systemd/system/traefik.service`

```ini
[Unit]
Description=Traefik Reverse Proxy
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/traefik --configFile=/etc/traefik/traefik.yaml
Restart=always
RestartSec=5
User=traefik
Group=traefik

# Environment (for Cloudflare DNS challenge)
EnvironmentFile=/etc/traefik/env

# Bind to privileged ports
AmbientCapabilities=CAP_NET_BIND_SERVICE

# Hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/etc/traefik /var/lib/traefik /var/log/traefik

[Install]
WantedBy=multi-user.target
```

### API Changes

**New fields in contract/instance response:**

```rust
pub struct ProvisionedInstance {
    // Existing fields...
    pub internal_ip: String,          // "10.0.1.5"

    // New gateway fields
    pub gateway_slug: Option<String>,           // "k7m2p4"
    pub gateway_subdomain: Option<String>,      // "k7m2p4.dc-lk.decent-cloud.org"
    pub gateway_ssh_port: Option<u16>,          // 20000
    pub gateway_port_range_start: Option<u16>,  // 20000
    pub gateway_port_range_end: Option<u16>,    // 20009
}
```

**Database migration:**

```sql
ALTER TABLE contract_sign_requests ADD COLUMN gateway_slug TEXT;
ALTER TABLE contract_sign_requests ADD COLUMN gateway_ssh_port INTEGER;
ALTER TABLE contract_sign_requests ADD COLUMN gateway_port_range_start INTEGER;
ALTER TABLE contract_sign_requests ADD COLUMN gateway_port_range_end INTEGER;

CREATE UNIQUE INDEX idx_gateway_slug ON contract_sign_requests(gateway_slug)
  WHERE gateway_slug IS NOT NULL;
```

### User Experience

**Contract details page shows:**

```
Connection Details
──────────────────
Web Access:    https://k7m2p4.dc-lk.decent-cloud.org
SSH Access:    ssh user@dc-lk.decent-cloud.org -p 20000
               or: ssh user@k7m2p4.dc-lk.decent-cloud.org -p 20000

Additional Ports: 20001-20009 available for your services
```

**SSH config suggestion:**
```
Host myvm
    HostName dc-lk.decent-cloud.org
    Port 20000
    User root
```

## Security Considerations

1. **TLS everywhere**: All HTTP traffic forced to HTTPS via redirect
2. **Wildcard cert isolation**: Each DC has own cert, compromise limited to that DC
3. **No cross-VM access**: Traefik routes are isolated per VM
4. **Cloudflare API token**: Scoped to DNS edit only, not full account access
5. **systemd hardening**: NoNewPrivileges, ProtectSystem, ProtectHome
6. **Port range isolation**: Each VM only gets its allocated ports

## Cost Analysis

| Component | Cost |
|-----------|------|
| IPv4 per host (5 hosts) | 5 x $1.61 = $8.05/month |
| Remaining IPs (19) | Buffer for growth/premium |
| Let's Encrypt certs | Free |
| Cloudflare DNS | Free tier sufficient |

**Savings vs 1:1 IP assignment:**
- 100 VMs with dedicated IPs: $161/month
- Gateway approach (5 hosts): $8.05/month
- **Savings: ~$153/month**

## Implementation Steps

### Phase 1: Infrastructure Setup (Automated)

**Step 1.1: Create Cloudflare API Token (Manual - One Time)**
- Log into Cloudflare dashboard: https://dash.cloudflare.com/profile/api-tokens
- Click "Create Token" → Use template "Edit zone DNS"
- Scope to zone: decent-cloud.org
- Copy the token (this is the only manual step)

**Step 1.2: Run Gateway Setup (Automated)**
```bash
dc-agent setup gateway \
  --host 203.0.113.1 \
  --datacenter dc-lk \
  --domain decent-cloud.org \
  --public-ip 203.0.113.1 \
  --cloudflare-token <YOUR_TOKEN> \
  --append-config dc-agent.toml
```

This automatically:
- Downloads and installs Traefik v3.x
- Creates traefik user/group
- Sets up directories: /etc/traefik, /var/lib/traefik, /var/log/traefik
- Generates static config with DNS-01 challenge
- Creates systemd service
- Enables and starts Traefik
- Auto-detects Cloudflare Zone ID from domain
- Appends [gateway] config to dc-agent.toml

**Step 1.3: Wildcard Certificate (Automatic)**
- Traefik auto-obtains wildcard cert on first HTTPS request
- Certificate covers *.{dc}.decent-cloud.org

### Phase 2: dc-agent Integration

**Step 2.1: Gateway Configuration**
- Add `[gateway]` section to dc-agent config
- Parse and validate on startup

**Step 2.2: Port Allocation**
- Implement port allocation file management
- Add allocate/free functions

**Step 2.3: Traefik Config Generation**
- Implement template for dynamic config YAML
- Write on provision, delete on destroy

**Step 2.4: Cloudflare DNS Integration**
- Add cloudflare API client (or use crate)
- Create DNS record on provision
- Delete DNS record on destroy

**Step 2.5: API Reporting**
- Include gateway fields in provisioned response
- Store in database

### Phase 3: API & UI

**Step 3.1: Database Migration**
- Add gateway columns to contract_sign_requests

**Step 3.2: API Response**
- Include gateway fields in contract detail endpoint

**Step 3.3: UI Updates**
- Display connection details on contract page
- Show subdomain, SSH command, port range

### Phase 4: Testing & Documentation

**Step 4.1: End-to-End Test**
- Provision VM
- Verify DNS record created
- Verify HTTPS accessible via subdomain
- Verify SSH accessible via port
- Destroy VM
- Verify cleanup

**Step 4.2: Documentation**
- Update provider setup guide
- Add user guide for connecting to VMs

## Alternatives Considered

### Centralized Gateway VM
- Single gateway VM handles all traffic
- **Rejected:** Single point of failure, extra hop latency, VM overhead

### frp/rathole Tunnel
- Tunnel from host to external relay
- **Rejected:** Unnecessary when hosts have public IPs; adds complexity

### BGP on Proxmox Hosts
- Each host announces its own IPs via BGP
- **Rejected:** Overkill; provider handles BGP, we just need routing

### Nginx instead of Traefik
- More mature, widely deployed
- **Rejected:** No native dynamic config; requires reload on changes

## Open Questions

1. **Custom domains**: Should users be able to bring their own domain? (Future enhancement)
2. **Port visibility**: Should users see/manage their port range in UI? (Start simple, add if requested)
3. **Multi-DC routing**: When we add more DCs, should there be geographic DNS routing? (Future)

## References

- [Traefik Documentation](https://doc.traefik.io/traefik/)
- [Traefik File Provider](https://doc.traefik.io/traefik/providers/file/)
- [Let's Encrypt DNS-01 Challenge](https://letsencrypt.org/docs/challenge-types/#dns-01-challenge)
- [Cloudflare API](https://developers.cloudflare.com/api/)
- [systemd Service Hardening](https://www.freedesktop.org/software/systemd/man/systemd.exec.html)
