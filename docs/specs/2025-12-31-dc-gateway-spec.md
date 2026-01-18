# DC Gateway: Per-Host Reverse Proxy Architecture

**Status:** Implemented
**Created:** 2025-12-31
**Updated:** 2026-01-18

## Problem Statement

Public IPv4 addresses are scarce and expensive (~$1.61 USD/month each). A typical datacenter with 5+ Proxmox servers can run 100-200+ VMs, but may only have 24 IPv4 addresses available. Assigning a dedicated public IP to each VM is not economically viable.

## Overview

Deploy a reverse proxy on each Proxmox host alongside dc-agent. Each host gets one public IPv4.

**Routing architecture:**
- **HTTP/HTTPS**: Caddy with automatic TLS via Let's Encrypt HTTP-01 challenge
- **TCP/UDP**: iptables DNAT for port forwarding (SSH, databases, game servers)

**Key benefits:**
- VMs serve plain HTTP on port 80 - users get HTTPS automatically
- No Cloudflare credentials needed on hosts - Caddy handles TLS via HTTP-01
- DNS managed via central API (agents never have Cloudflare access)

dc-agent manages Caddy config and iptables rules as part of VM provisioning lifecycle.

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
   │  caddy   │         │  caddy   │         │  caddy   │
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
- [x] Caddy running as systemd service on each Proxmox host (`dc-agent setup gateway` automates this)
- [x] Automatic TLS via Let's Encrypt HTTP-01 challenge (per-subdomain certificates)
- [x] dc-agent writes Caddy site config on VM provision/destroy
- [x] dc-agent updates Cloudflare DNS via central API on VM provision/destroy
- [x] HTTP/HTTPS routing via subdomain
- [x] TCP port mapping for SSH and custom services
- [x] UDP port mapping for game servers and similar
- [x] Port range allocation per VM (default: 10 ports)

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
- Caddy on host receives public traffic, terminates TLS, proxies HTTP to VMs

**Traffic flow:**
```
User HTTPS request
    │
    ▼ DNS: k7m2p4.dc-lk.decent-cloud.org → 203.0.113.1
┌─────────────────────────────────────────────────────┐
│ Proxmox Host (203.0.113.1)                          │
│                                                     │
│   Caddy (:443)                                      │
│      │                                              │
│      │ TLS termination (auto Let's Encrypt cert)   │
│      │                                              │
│      ▼                                              │
│   Proxy HTTP to 10.0.1.5:80                         │
│      │                                              │
│      ▼                                              │
│   ┌─────────┐                                       │
│   │   VM    │  ← runs plain HTTP (WordPress, etc.) │
│   │10.0.1.5 │                                       │
│   └─────────┘                                       │
└─────────────────────────────────────────────────────┘
```

### DNS Configuration

**Zone:** `decent-cloud.org` (managed via Cloudflare, accessed only by central API)

**Dynamic records (created per-VM by dc-agent via central API):**
```
k7m2p4.dc-lk    A    203.0.113.1    ; VM on host 1
x9f3a2.dc-lk    A    203.0.113.2    ; VM on host 2
```

**TLS certificates:**
- Per-subdomain certificates via HTTP-01 challenge
- Caddy obtains certs automatically when first request arrives
- No wildcard cert needed, no Cloudflare credentials on hosts

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
  "allocations": {
    "k7m2p4": { "base": 20000, "count": 10, "contract_id": "..." },
    "x9f3a2": { "base": 20010, "count": 10, "contract_id": "..." }
  }
}
```

### Caddy Configuration

Caddy handles HTTP/HTTPS routing with automatic TLS termination.
TCP/UDP port forwarding is handled by iptables DNAT (see below).

**Main config:** `/etc/caddy/Caddyfile`

```
# Caddy configuration for DC Gateway
# Generated by dc-agent setup gateway

{
    admin off
    persist_config off
    storage file_system /var/lib/caddy
}

# Import all VM-specific site configurations
import /etc/caddy/sites/*.caddy
```

**Per-VM config:** `/etc/caddy/sites/k7m2p4.caddy`

```
# Generated by dc-agent for VM k7m2p4
# Contract: c_abc123...
# Created: 2026-01-18T10:30:00Z
#
# HTTPS: TLS terminated at gateway, proxied to VM:80 (plain HTTP)
# TCP/UDP ports 20000-20009: iptables DNAT

k7m2p4.dc-lk.decent-cloud.org {
    reverse_proxy 10.0.1.5:80
}
```

### iptables DNAT (TCP/UDP Port Forwarding)

TCP/UDP port forwarding uses kernel-level iptables DNAT rules. This is more efficient
than userspace proxies since there's no application overhead.

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

**Why iptables instead of Caddy for TCP/UDP:**
- Caddy's TCP/UDP proxy requires static config (can't dynamically add ports)
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

# Caddy integration
caddy_sites_dir = "/etc/caddy/sites"
port_allocations_path = "/var/lib/dc-agent/port-allocations.json"

# DNS management is handled via central API (no Cloudflare credentials needed)
# TLS certificates are managed automatically by Caddy via HTTP-01 challenge
```

**Provisioning flow:**

```
1. VM Provisioned (existing flow)
   └── dc-agent creates VM on Proxmox
   └── VM gets internal IP (e.g., 10.0.1.5)

2. Gateway Setup
   ├── Generate slug: k7m2p4
   ├── Allocate port range: 20000-20009
   ├── Create DNS record via central API (must exist before Caddy config)
   ├── Setup iptables DNAT rules (TCP/UDP port forwarding)
   └── Write Caddy config: /etc/caddy/sites/k7m2p4.caddy
       └── Caddy reloads, obtains Let's Encrypt cert via HTTP-01

3. Report to API
   └── Include in provisioned response:
       - gateway_slug: k7m2p4
       - gateway_subdomain: k7m2p4.dc-lk.decent-cloud.org
       - ssh_port: 20000
       - port_range: 20000-20009
```

**Destroy flow:**

```
1. Gateway Cleanup
   ├── Delete Caddy config: rm /etc/caddy/sites/k7m2p4.caddy
   ├── Caddy reloads
   ├── Remove iptables DNAT rules for this slug
   ├── Delete DNS record via central API
   └── Free port range in allocation file

2. VM Termination (existing)
   └── dc-agent destroys VM on Proxmox
```

### Cloudflare DNS Integration

DNS management is centralized in the API server for security. Individual dc-agent hosts never have Cloudflare credentials.

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
CF_DOMAIN=decent-cloud.org
```

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
ReadWritePaths=/var/lib/dc-agent /etc/caddy/sites

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=dc-agent

[Install]
WantedBy=multi-user.target
```

**Caddy service:** `/etc/systemd/system/caddy.service`

```ini
[Unit]
Description=Caddy Web Server
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/caddy run --config /etc/caddy/Caddyfile
ExecReload=/usr/local/bin/caddy reload --config /etc/caddy/Caddyfile
Restart=always
RestartSec=5
User=caddy
Group=caddy

# Bind to privileged ports
AmbientCapabilities=CAP_NET_BIND_SERVICE

# Hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/etc/caddy /var/lib/caddy

[Install]
WantedBy=multi-user.target
```

### API Changes

**New fields in contract/instance response:**

```rust
pub struct ProvisionedInstance {
    // Existing fields...
    pub internal_ip: String,          // "10.0.1.5"

    // Gateway fields
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
SSH Access:    ssh user@k7m2p4.dc-lk.decent-cloud.org -p 20000

Additional Ports: 20001-20009 available for your services
```

**SSH config suggestion:**
```
Host myvm
    HostName k7m2p4.dc-lk.decent-cloud.org
    Port 20000
    User root
```

## Security Considerations

1. **TLS everywhere**: All HTTP traffic forced to HTTPS via Caddy's automatic redirect
2. **Per-subdomain certs**: Each VM gets its own certificate (no shared wildcard)
3. **No cross-VM access**: Caddy routes are isolated per VM
4. **Cloudflare credentials centralized**: Only the API server has Cloudflare access
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

## Gateway Setup

**Run on Proxmox host:**
```bash
dc-agent setup token \
  --token <AGENT_TOKEN> \
  --proxmox-host <HOST_IP> \
  --gateway-datacenter dc-lk \
  --gateway-public-ip <PUBLIC_IP>
```

This automatically:
- Downloads and installs Caddy
- Creates caddy user/group
- Sets up directories: /etc/caddy, /var/lib/caddy
- Generates Caddyfile with site import
- Creates systemd service
- Enables and starts Caddy
- Configures IP forwarding and NAT masquerade
- Opens firewall ports (80, 443, 20000-59999)
- Persists iptables rules

## Alternatives Considered

### Traefik instead of Caddy
- More complex configuration
- DNS-01 challenge requires Cloudflare credentials on each host
- **Rejected:** Caddy is simpler and HTTP-01 eliminates credential distribution

### Centralized Gateway VM
- Single gateway VM handles all traffic
- **Rejected:** Single point of failure, extra hop latency, VM overhead

### frp/rathole Tunnel
- Tunnel from host to external relay
- **Rejected:** Unnecessary when hosts have public IPs; adds complexity

### Nginx
- More mature, widely deployed
- **Rejected:** No native dynamic config; requires reload on changes

## Open Questions

1. **Custom domains**: Should users be able to bring their own domain? (Future enhancement)
2. **Port visibility**: Should users see/manage their port range in UI? (Start simple, add if requested)
3. **Multi-DC routing**: When we add more DCs, should there be geographic DNS routing? (Future)

## References

- [Caddy Documentation](https://caddyserver.com/docs/)
- [Let's Encrypt HTTP-01 Challenge](https://letsencrypt.org/docs/challenge-types/#http-01-challenge)
- [Cloudflare API](https://developers.cloudflare.com/api/)
- [systemd Service Hardening](https://www.freedesktop.org/software/systemd/man/systemd.exec.html)
