# Proxmox Public IP Deployment Spec

**Status:** Complete
**Created:** 2026-01-16
**Updated:** 2026-02-13
**Depends On:** [DC Gateway Spec](./2025-12-31-dc-gateway-spec.md)

## Problem Statement

A Proxmox node has been assigned a real public IP address. We need to configure the host networking and complete the code integration to enable:
1. SSH access to VMs via port forwarding
2. TCP/UDP access to selected ports on VMs
3. HTTP/HTTPS website hosting from VMs via reverse proxy

The gateway architecture is designed (see DC Gateway Spec) and fully integrated into the provisioning flow.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           Public Internet                               │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                           Public IP (203.0.113.1)
                                    │
┌─────────────────────────────────────────────────────────────────────────┐
│                         Proxmox Host                                    │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ Caddy (ports 80, 443)                                           │   │
│  │                                                                 │   │
│  │  HTTP (80)  → Automatic redirect to HTTPS                       │   │
│  │  HTTPS (443) → TLS termination → HTTP proxy to VM:80            │   │
│  │               (per-provider wildcard cert via DNS-01 with acme-dns) │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ iptables DNAT (ports 20000-59999)                               │   │
│  │                                                                 │   │
│  │  SSH: 20000 → VM:22                                             │   │
│  │  TCP: 20001-20004 → VM:10001-10004                              │   │
│  │  UDP: 20005-20009 → VM:10005-10009                              │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                  │
│  │ VM (10.0.1.5)│  │ VM (10.0.1.6)│  │ VM (10.0.1.7)│                  │
│  │              │  │              │  │              │                  │
│  │ Runs HTTP    │  │ Runs HTTP    │  │ Runs HTTP    │                  │
│  │ on port 80   │  │ on port 80   │  │ on port 80   │                  │
│  └──────────────┘  └──────────────┘  └──────────────┘                  │
└─────────────────────────────────────────────────────────────────────────┘
```

**Key design decisions:**
- **TLS termination at Caddy**: Per-provider wildcard cert `*.{dc_id}.{gw_prefix}.{domain}` via DNS-01 challenge with acme-dns
- **VMs serve plain HTTP**: No TLS config needed on VMs - users get HTTPS automatically
- **acme-dns credentials on host**: Obtained from central API during gateway registration (scoped to provider's subdomain)
- **DNS managed centrally**: `{slug}.{dc_id}.{gw_prefix}.{domain}` records created via central API

## Current State

### Implemented ✅
- Port allocation system (`dc-agent/src/gateway/port_allocator.rs`)
- Caddy config generation (`dc-agent/src/gateway/caddy.rs`)
- iptables DNAT rule management (`dc-agent/src/gateway/iptables.rs`)
- Gateway manager orchestration (`dc-agent/src/gateway/mod.rs`)
- Gateway setup CLI (`dc-agent setup gateway`)
- Configuration parsing for `[gateway]` section
- Gateway setup called during VM provisioning (`dc-agent/src/main.rs`)
- Gateway cleanup called during VM termination (`dc-agent/src/main.rs`)
- DNS record creation/deletion via central API (`POST /api/v1/agents/dns`)
- Database columns for gateway fields (`gateway_slug`, `gateway_ssh_port`, `gateway_port_range_start`, `gateway_port_range_end`)
- API response fields for gateway info (`Contract` struct includes all gateway fields)
- UI display of connection details (`website/src/routes/dashboard/rentals/[contract_id]/+page.svelte`)
- Bandwidth monitoring via heartbeat (`dc-agent/src/gateway/bandwidth.rs`)

### Automated via `dc-agent setup gateway`
- IP forwarding (`net.ipv4.ip_forward=1`)
- NAT masquerade for all RFC1918 ranges (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16)
- Firewall rules (80, 443, 20000-59999 for VMs)
- FORWARD chain rules for all private ranges
- iptables persistence via `iptables-persistent`
- Caddy installation with acmedns plugin (per-provider wildcard TLS via DNS-01)

### Requires Manual Setup (Pre-requisites)
- Public IP must be assigned to host interface
- SSH access to host with root privileges
- Central API must have ACME_DNS_SERVER_URL configured (for acme-dns registration during gateway setup)

## Scope

This spec covers deploying **one Proxmox node** with public IP. Multi-node deployment follows the same pattern.

## Prerequisites

Before starting:
- [ ] Public IP assigned to Proxmox node (e.g., `203.0.113.1`)
- [ ] Proxmox host accessible via SSH
- [ ] `dc-agent` binary available
- [ ] Central API server running with ACME_DNS_SERVER_URL configured

## Implementation Plan

### Phase 1: Host Network Configuration

**Goal:** Configure the Proxmox host to route traffic between public IP and private VMs.

> **Note:** This phase is now **fully automated** by `dc-agent setup gateway`. The command will:
> - Detect the public interface automatically
> - Enable IP forwarding and persist it
> - Configure NAT masquerade for VM subnet (10.0.0.0/8)
> - Open firewall ports (80, 443, 20000-59999)
> - Add FORWARD rules for VM traffic
> - Install and persist iptables rules

#### What Gets Configured (for reference)

**IP Forwarding:**
```bash
# Enabled via sysctl and persisted to /etc/sysctl.d/99-dc-gateway.conf
net.ipv4.ip_forward = 1
```

**NAT Masquerade (all RFC1918 ranges):**
```bash
# Auto-detects public interface from provided IP
# Covers any private IP a DHCP server could assign to VMs
iptables -t nat -A POSTROUTING -s 10.0.0.0/8 -o <public_iface> -j MASQUERADE
iptables -t nat -A POSTROUTING -s 172.16.0.0/12 -o <public_iface> -j MASQUERADE
iptables -t nat -A POSTROUTING -s 192.168.0.0/16 -o <public_iface> -j MASQUERADE
```

**Firewall Rules:**
```bash
# HTTP/HTTPS for Caddy
iptables -A INPUT -p tcp --dport 80 -j ACCEPT
iptables -A INPUT -p tcp --dport 443 -j ACCEPT

# VM port range
iptables -A INPUT -p tcp --dport 20000:59999 -j ACCEPT
iptables -A INPUT -p udp --dport 20000:59999 -j ACCEPT

# FORWARD chain for VM traffic (all RFC1918 ranges)
iptables -A FORWARD -m state --state RELATED,ESTABLISHED -j ACCEPT
iptables -A FORWARD -s 10.0.0.0/8 -j ACCEPT
iptables -A FORWARD -d 10.0.0.0/8 -j ACCEPT
iptables -A FORWARD -s 172.16.0.0/12 -j ACCEPT
iptables -A FORWARD -d 172.16.0.0/12 -j ACCEPT
iptables -A FORWARD -s 192.168.0.0/16 -j ACCEPT
iptables -A FORWARD -d 192.168.0.0/16 -j ACCEPT
```

#### Verify Network Configuration (after setup)

```bash
# Test from a VM (after provisioning one)
# VM should be able to ping external IPs
ping 8.8.8.8

# Test from external
# Should be able to reach Proxmox web UI
curl -k https://203.0.113.1:8006
```

### Phase 2: Gateway Infrastructure Setup

**Goal:** Install and configure Caddy for automatic TLS termination.

#### 2.1 Run Gateway Setup

```bash
dc-agent setup token \
  --token <AGENT_TOKEN> \
  --proxmox-host <PROXMOX_HOST> \
  --gateway-dc-id <DC_ID> \
  --gateway-public-ip 203.0.113.1
```

This command:
- Registers with central API for acme-dns credentials
- Downloads Caddy binary with acmedns plugin
- Creates systemd service with EnvironmentFile for acme-dns credentials
- Configures per-provider wildcard TLS via DNS-01 with acme-dns
- Creates required directories

#### 2.2 Configure dc-agent

Add to `dc-agent.toml`:

```toml
[gateway]
dc_id = "a3x9f2b1"  # Unique datacenter identifier (2-20 chars [a-z0-9-])
public_ip = "203.0.113.1"
port_range_start = 20000
port_range_end = 59999
ports_per_vm = 10
caddy_sites_dir = "/etc/caddy/sites"
port_allocations_path = "/var/lib/dc-agent/port-allocations.json"
```

#### 2.3 Verify Caddy

```bash
# Check service status
sudo systemctl status caddy

# Check logs
sudo journalctl -u caddy -f

# Verify listening on ports 80 and 443
ss -tlnp | grep caddy
```

Note: Caddy obtains a wildcard certificate via DNS-01 challenge on startup. All VM subdomains are covered by this single cert.

### Phase 3: Testing

#### 3.1 End-to-End Tests

```bash
# 1. Provision a test VM
dc-agent provision --contract-id test-123

# 2. Verify DNS record exists
dig k7m2p4.a3x9f2b1.dev-gw.decent-cloud.org

# 3. Start a simple HTTP server on the VM
# On VM: python3 -m http.server 80

# 4. Verify HTTPS works (wildcard cert already covers subdomain)
curl https://k7m2p4.a3x9f2b1.dev-gw.decent-cloud.org

# 5. Verify SSH works
ssh -p 20000 root@k7m2p4.a3x9f2b1.dev-gw.decent-cloud.org

# 6. Verify port forwarding works
# Start listener on VM port 10001
nc -l 10001  # on VM

# Connect from external
nc k7m2p4.a3x9f2b1.dev-gw.decent-cloud.org 20001

# 7. Terminate VM
dc-agent terminate --contract-id test-123

# 8. Verify cleanup
dig k7m2p4.a3x9f2b1.dev-gw.decent-cloud.org  # Should return NXDOMAIN
```

## Task Checklist

### Pre-requisites (Per Deployment)
- [ ] Assign public IP to Proxmox host interface
- [ ] Ensure SSH access to host with root privileges
- [ ] Ensure central API has ACME_DNS_SERVER_URL configured

### Infrastructure (Automated by `dc-agent setup gateway`)
- [x] Enable IP forwarding (`net.ipv4.ip_forward=1`)
- [x] Configure NAT masquerade for all RFC1918 ranges
- [x] Open firewall ports (80, 443, 20000-59999)
- [x] Configure FORWARD rules for all private ranges
- [x] Persist iptables rules
- [x] Install Caddy binary
- [x] Configure Caddy for per-provider wildcard TLS (DNS-01 with acme-dns)
- [x] Start Caddy systemd service

### To Deploy
```bash
dc-agent setup token \
  --token <AGENT_TOKEN> \
  --proxmox-host <PROXMOX_HOST> \
  --gateway-dc-id <DC_ID> \
  --gateway-public-ip <PUBLIC_IP>
```

### Code Integration
- [x] Wire `GatewayManager::setup_gateway()` into provisioning
- [x] Wire `GatewayManager::cleanup_gateway()` into termination
- [x] Implement `POST /api/v1/agents/dns` endpoint in api-server
- [x] Implement Cloudflare client for DNS record management (central API only)
- [x] Add database migration for gateway columns
- [x] Update `Contract` API response
- [x] Update frontend to display connection details

### Testing
- [x] Add unit tests for gateway components
- [x] Add unit tests for DNS API validation
- [x] Add unit tests for gateway fields in contracts
- [x] Add unit tests for Caddy config generation

## Security Considerations

1. **Firewall before gateway**: Ensure host firewall is configured before exposing gateway ports
2. **acme-dns credentials on host**: Scoped to provider's subdomain; stored in `/etc/caddy/env` with mode 600
3. **Agent authentication**: DNS endpoint requires agent auth with DnsManage permission
4. **Rate limiting**: Consider rate limiting DNS operations to prevent abuse
5. **Audit logging**: Log all DNS changes for security review

## Rollback Plan

If issues arise:

1. **Disable gateway for new VMs**: Remove `[gateway]` section from config
2. **Existing VMs continue working**: Gateway rules remain in place
3. **Manual cleanup if needed**:
   ```bash
   # Remove all gateway iptables rules
   sudo iptables -t nat -F DC_GATEWAY

   # Remove Caddy configs
   sudo rm /etc/caddy/sites/*.caddy

   # Stop Caddy
   sudo systemctl stop caddy
   ```

## Future Enhancements

- Custom domain support (user brings their own domain, we route via SNI)
- Per-VM bandwidth monitoring and limits
- Premium tier with dedicated public IP
- Geographic DNS routing for multi-DC

## References

- [DC Gateway Spec](./2025-12-31-dc-gateway-spec.md) - Architecture details
- [Caddy Documentation](https://caddyserver.com/docs/)
- [Let's Encrypt DNS-01](https://letsencrypt.org/docs/challenge-types/#dns-01-challenge)
- [Caddy acmedns Plugin](https://github.com/caddy-dns/acmedns)
- [acme-dns](https://github.com/joohoi/acme-dns)
