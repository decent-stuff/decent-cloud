# Provider Tunnel Relay for NAT'd Providers

**Status:** Draft
**Created:** 2025-12-25

## Problem Statement

Some providers do not have public IPv4 addresses. While their `dc-agent` can poll the API (outbound connections work), users cannot SSH into provisioned VMs because there's no publicly routable IP.

## Overview

Deploy a self-hosted tunnel relay using [frp](https://github.com/fatedier/frp) on a Hetzner VPS. Providers without public IPs run an frp client that establishes outbound connections to the relay. The relay forwards incoming SSH/TCP connections to the provider's VMs.

```
User SSH → vm-abc.tunnel.decent-cloud.org:22
                ↓
        Relay VPS (frps)
                ↓ (persistent outbound connection)
        Provider's frpc
                ↓
        Proxmox VM (192.168.x.x:22)
```

## Requirements

### Must-have
- [ ] frp server (frps) running on Hetzner VPS with public IP
- [ ] DNS wildcard `*.tunnel.decent-cloud.org` pointing to relay VPS
- [ ] TLS termination for frp control plane
- [ ] Dynamic port/subdomain allocation per provisioned VM
- [ ] Integration with dc-agent: auto-register tunnel on VM provision, deregister on destroy
- [ ] Authentication: providers authenticate to relay with token
- [ ] Basic monitoring: relay health check endpoint

### Nice-to-have
- [ ] Multiple relay regions (EU, US, Asia)
- [ ] Automatic failover between relays
- [ ] Bandwidth usage tracking per provider
- [ ] Rate limiting per provider

## Technical Design

### Infrastructure

**Hetzner VPS:**
- Model: CX22 (2 vCPU, 4GB RAM, 40GB disk)
- Cost: ~€4.50/month
- Location: Falkenstein (eu-central) or Nuremberg
- Included traffic: 20 TB/month
- OS: Ubuntu 24.04 LTS

**Network:**
- Public IPv4 (included)
- Ports: 22 (SSH admin), 7000 (frps bind), 7500 (frps dashboard), 20000-30000 (dynamic VM ports)
- Firewall: UFW allowing only required ports

### frp Configuration

**Server (frps.toml):**
```toml
bindAddr = "0.0.0.0"
bindPort = 7000

# Dashboard for monitoring
webServer.addr = "0.0.0.0"
webServer.port = 7500
webServer.user = "admin"
webServer.password = "{{ DASHBOARD_PASSWORD }}"

# Authentication
auth.method = "token"
auth.token = "{{ FRP_AUTH_TOKEN }}"

# Subdomain support
subDomainHost = "tunnel.decent-cloud.org"

# Port range for TCP proxies
allowPorts = [
  { start = 20000, end = 30000 }
]

# Logging
log.to = "/var/log/frps.log"
log.level = "info"
log.maxDays = 7
```

**Client (frpc.toml) - Template for providers:**
```toml
serverAddr = "relay.decent-cloud.org"
serverPort = 7000

auth.method = "token"
auth.token = "{{ PROVIDER_TOKEN }}"

# Each VM gets a proxy section added dynamically
[[proxies]]
name = "vm-{{ VM_ID }}-ssh"
type = "tcp"
localIP = "{{ VM_LOCAL_IP }}"
localPort = 22
remotePort = {{ ALLOCATED_PORT }}
```

### DNS Configuration

Add to decent-cloud.org DNS:
```
relay.tunnel    A       <VPS_IP>
*.tunnel        CNAME   relay.tunnel.decent-cloud.org.
```

### dc-agent Integration

Extend dc-agent to manage frp tunnels when provisioning VMs for providers without public IPs.

**New configuration fields in agent config:**
```toml
[tunnel]
enabled = true
relay_server = "relay.decent-cloud.org"
relay_port = 7000
auth_token = "{{ PROVIDER_TUNNEL_TOKEN }}"
```

**Provisioning flow changes:**

1. **VM Provisioned** → dc-agent checks if provider has public IP
2. **No public IP** → Allocate port from relay (API call or local counter)
3. **Register tunnel** → Add proxy to frpc config, reload frpc
4. **Report to API** → Include tunnel endpoint: `relay.tunnel.decent-cloud.org:PORT`
5. **VM Destroyed** → Remove proxy from frpc config, reload frpc

**frpc management options:**

Option A: **dc-agent manages frpc directly**
- dc-agent spawns/manages frpc subprocess
- Writes config file, sends SIGHUP to reload
- Simpler, single process to manage

Option B: **frpc as separate systemd service**
- frpc runs independently
- dc-agent modifies config and triggers reload via systemctl
- More robust, survives dc-agent restarts

**Recommendation:** Option A for initial implementation (simpler). => Yes, let's do it!

### API Changes

**New fields in OfferingInstance:**
```rust
pub struct OfferingInstance {
    // ... existing fields ...

    /// Tunnel endpoint if VM is behind NAT (e.g., "relay.tunnel.decent-cloud.org:20001")
    pub tunnel_endpoint: Option<String>,
}
```

**Provider registration:**
- Add `has_public_ip: bool` field to provider/agent registration
- Generate unique tunnel auth token per provider

### Security Considerations

1. **Authentication**: Each provider gets unique frp auth token
2. **Port isolation**: Providers can only register ports in allocated range
3. **No cross-provider access**: frp proxies are isolated by design
4. **TLS**: Consider adding TLS for frp control plane (frps supports it)
5. **Token rotation**: Implement token rotation mechanism for compromised tokens
6. **Firewall**: VPS firewall allows only required ports

### Monitoring

1. **frps dashboard**: Built-in web UI at port 7500 (internal only)
2. **Health endpoint**: Simple HTTP check on relay
3. **Metrics to collect**:
   - Active connections per provider
   - Bandwidth per provider
   - Connection errors/failures
4. **Alerts**: Disk space, memory, connection count thresholds

### Cost Analysis

| Component                 | Monthly Cost    |
|---------------------------|-----------------|
| Hetzner CX22 VPS          | €4.50           |
| Domain (already owned)    | €0              |
| Bandwidth (20TB included) | €0              |
| **Total**                 | **€4.50/month** |

**Scaling triggers:**
- \>100 concurrent tunnels → Consider CX32 (€7.50/month)
- \>500 concurrent tunnels → Multiple regional relays
- \>10TB/month traffic → Still within included quota

## Implementation Steps

### Step 1: Provision Hetzner VPS
**Success:** VPS running with SSH access, firewall configured
**Status:** Pending

Tasks:
- Create Hetzner Cloud account (if needed)
- Provision CX22 in eu-central
- Configure SSH key access
- Install UFW, allow ports: 22, 7000, 7500, 20000-30000
- Install Docker (for containerized deployment) or install frp directly

### Step 2: Deploy frps
**Success:** frps running, dashboard accessible (internally)
**Status:** Pending

Tasks:
- Create `/etc/frp/frps.toml` with configuration
- Create systemd service for frps
- Start and enable service
- Test dashboard access
- Generate initial auth token

### Step 3: Configure DNS
**Success:** `relay.tunnel.decent-cloud.org` resolves to VPS IP
**Status:** Pending

Tasks:
- Add A record for `relay.tunnel`
- Add wildcard CNAME for `*.tunnel`
- Verify DNS propagation

### Step 4: Test manual tunnel
**Success:** Can SSH to test VM through relay
**Status:** Pending

Tasks:
- Set up test frpc on a machine with private IP
- Configure proxy for local SSH
- Test SSH connection through relay
- Verify port allocation works

### Step 5: Extend dc-agent configuration
**Success:** dc-agent can parse tunnel config section
**Status:** Pending

Files to modify:
- `dc-agent/src/config.rs` - Add TunnelConfig struct
- `dc-agent/src/main.rs` - Load tunnel config

### Step 6: Implement frpc management in dc-agent
**Success:** dc-agent can start/stop/reload frpc
**Status:** Pending

Files to modify:
- `dc-agent/src/tunnel.rs` (new) - FrpcManager struct
- `dc-agent/src/main.rs` - Initialize FrpcManager

### Step 7: Integrate tunnel with VM provisioning
**Success:** VMs behind NAT get tunnel endpoints, reported to API
**Status:** Pending

Files to modify:
- `dc-agent/src/provisioners/proxmox.rs` - Add tunnel registration after VM start
- `dc-agent/src/provisioners/mod.rs` - ProvisionResult includes tunnel_endpoint
- `api/src/database/offerings.rs` - Store tunnel_endpoint in instance

### Step 8: Update API to handle tunnel endpoints
**Success:** API stores and returns tunnel endpoints
**Status:** Pending

Files to modify:
- `api/src/database/offerings.rs` - Add tunnel_endpoint field
- `api/src/openapi/offerings.rs` - Return tunnel_endpoint in responses

### Step 9: Update UI to display tunnel endpoints
**Success:** Contract details show tunnel endpoint for NAT'd VMs
**Status:** Pending

Files to modify:
- `website/src/routes/dashboard/contracts/[id]/+page.svelte` - Show tunnel endpoint

### Step 10: Documentation and provider onboarding
**Success:** Providers can enable tunnel relay via docs
**Status:** Pending

Files to modify:
- `docs/provider-agent-installation.md` - Add tunnel configuration section

## Alternatives Considered

### Cloudflare Tunnels
- **Pros:** Free tier, managed infrastructure
- **Cons:** ToS prohibits commercial use on free tier, SSH requires client-side setup
- **Verdict:** Not suitable for production commercial use

### Tailscale Funnel
- **Pros:** Easy setup, WireGuard-based
- **Cons:** Custom domains require Enterprise plan, not designed for this use case
- **Verdict:** Too expensive for custom domain requirement

### WireGuard mesh
- **Pros:** Lower latency, full control
- **Cons:** More complex setup, requires key management
- **Verdict:** Good future option, higher initial complexity

### ngrok
- **Pros:** Polished product, good DX
- **Cons:** $20+/month per endpoint, gets expensive at scale
- **Verdict:** Too expensive for multiple providers

## Open Questions

1. **Port allocation strategy**: Sequential from pool vs. hash-based vs. API-managed? => API-managed seems the simplest, but do whatever you think is best
2. **Token provisioning**: Manual per provider vs. automatic during agent registration? => automatic seems the simplest (UX), but do whatever you think is best
3. **Failover**: Single relay acceptable initially, or need redundancy from day 1? => single relay initially should be fine
4. **Bandwidth limits**: Should we enforce per-provider bandwidth limits? => not necessary for the MVP

## References

- [frp GitHub](https://github.com/fatedier/frp) => feel free to clone the repo
- [frp Documentation](https://gofrp.org/docs/)
- [Hetzner Cloud Pricing](https://www.hetzner.com/cloud)
