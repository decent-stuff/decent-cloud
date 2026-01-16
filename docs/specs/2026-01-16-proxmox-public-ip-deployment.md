# Proxmox Public IP Deployment Spec

**Status:** Implemented
**Created:** 2026-01-16
**Updated:** 2026-01-16
**Depends On:** [DC Gateway Spec](./2025-12-31-dc-gateway-spec.md)

## Problem Statement

A Proxmox node has been assigned a real public IP address. We need to configure the host networking and complete the code integration to enable:
1. SSH access to VMs via port forwarding
2. TCP/UDP access to selected ports on VMs
3. HTTP/HTTPS website hosting from VMs via reverse proxy

The gateway architecture is designed (see DC Gateway Spec) and fully integrated into the provisioning flow.

## Current State

### Implemented ✅
- Port allocation system (`dc-agent/src/gateway/port_allocator.rs`)
- Traefik config generation (`dc-agent/src/gateway/traefik.rs`)
- iptables DNAT rule management (`dc-agent/src/gateway/iptables.rs`)
- Gateway manager orchestration (`dc-agent/src/gateway/mod.rs`)
- Gateway setup CLI (`dc-agent setup gateway`)
- Configuration parsing for `[gateway]` section
- Gateway setup called during VM provisioning (`dc-agent/src/main.rs` lines 1511-1535)
- Gateway cleanup called during VM termination (`dc-agent/src/main.rs` lines 1681-1693)
- DNS record creation/deletion via central API (`POST /api/v1/agents/dns`)
- Database columns for gateway fields (`gateway_slug`, `gateway_ssh_port`, `gateway_port_range_start`, `gateway_port_range_end`)
- API response fields for gateway info (`Contract` struct includes all gateway fields)
- UI display of connection details (`website/src/routes/dashboard/rentals/[contract_id]/+page.svelte` lines 546-568)
- Bandwidth monitoring via heartbeat (`dc-agent/src/gateway/bandwidth.rs`)

### Automated via `dc-agent setup gateway`
- IP forwarding (`net.ipv4.ip_forward=1`)
- NAT masquerade for VM subnet (10.0.0.0/8)
- Firewall rules (80, 443, 20000-59999 for VMs)
- FORWARD chain rules for VM traffic
- iptables persistence via `iptables-persistent`
- Traefik installation and configuration

### Requires Manual Setup (Pre-requisites)
- Public IP must be assigned to host interface
- SSH access to host with root privileges
- Cloudflare API token with DNS edit permission

## Scope

This spec covers deploying **one Proxmox node** with public IP. Multi-node deployment follows the same pattern.

## Prerequisites

Before starting:
- [ ] Public IP assigned to Proxmox node (e.g., `203.0.113.1`)
- [ ] Proxmox host accessible via SSH
- [ ] `dc-agent` binary available
- [ ] Cloudflare API token with DNS edit permission for `decent-cloud.org`
- [ ] Central API server running and accessible

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

**NAT Masquerade:**
```bash
# Auto-detects public interface from provided IP
iptables -t nat -A POSTROUTING -s 10.0.0.0/8 -o <public_iface> -j MASQUERADE
```

**Firewall Rules:**
```bash
# HTTP/HTTPS for Traefik
iptables -A INPUT -p tcp --dport 80 -j ACCEPT
iptables -A INPUT -p tcp --dport 443 -j ACCEPT

# VM port range
iptables -A INPUT -p tcp --dport 20000:59999 -j ACCEPT
iptables -A INPUT -p udp --dport 20000:59999 -j ACCEPT

# FORWARD chain for VM traffic
iptables -A FORWARD -m state --state RELATED,ESTABLISHED -j ACCEPT
iptables -A FORWARD -s 10.0.0.0/8 -j ACCEPT
iptables -A FORWARD -d 10.0.0.0/8 -j ACCEPT
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

**Goal:** Install and configure Traefik reverse proxy.

#### 2.1 Run Gateway Setup

```bash
dc-agent setup gateway \
  --datacenter dc-lk \
  --domain decent-cloud.org \
  --public-ip 203.0.113.1 \
  --cloudflare-token <CF_API_TOKEN>
```

This command:
- Downloads Traefik binary
- Creates systemd service
- Configures wildcard TLS certificate via Let's Encrypt
- Creates required directories

#### 2.2 Configure dc-agent

Add to `dc-agent.toml`:

```toml
[gateway]
datacenter = "dc-lk"
domain = "decent-cloud.org"
public_ip = "203.0.113.1"
port_range_start = 20000
port_range_end = 59999
ports_per_vm = 10
traefik_dynamic_dir = "/etc/traefik/dynamic"
port_allocations_path = "/var/lib/dc-agent/port-allocations.json"
```

#### 2.3 Verify Traefik

```bash
# Check service status
sudo systemctl status traefik

# Check logs for certificate acquisition
sudo journalctl -u traefik -f

# Test HTTPS (should get certificate error until cert is issued)
curl -v https://test.dc-lk.decent-cloud.org
```

### Phase 3: Code Integration

**Goal:** Wire gateway setup into VM provisioning lifecycle.

#### 3.1 Integrate Gateway into Provisioning

**File:** `dc-agent/src/provisioner/proxmox.rs`

Modify `provision()` to call gateway setup after VM is created:

```rust
// After VM is provisioned and IP is obtained...

// Setup gateway (if gateway config exists)
if let Some(gateway_config) = &self.config.gateway {
    let gateway_manager = GatewayManager::new(gateway_config.clone());

    let gateway_info = gateway_manager
        .setup_vm_gateway(&contract_id, &vm_ip)
        .await?;

    // Create DNS record via central API
    api_client
        .create_dns_record(&gateway_info.slug, &gateway_config.datacenter, &gateway_config.public_ip)
        .await?;

    instance.gateway_slug = Some(gateway_info.slug);
    instance.gateway_subdomain = Some(gateway_info.subdomain);
    instance.gateway_ssh_port = Some(gateway_info.ssh_port);
    instance.gateway_port_range_start = Some(gateway_info.port_range_start);
    instance.gateway_port_range_end = Some(gateway_info.port_range_end);
}
```

#### 3.2 Integrate Gateway into Termination

**File:** `dc-agent/src/provisioner/proxmox.rs`

Modify `terminate()` to clean up gateway:

```rust
// Before VM is destroyed...

if let Some(gateway_config) = &self.config.gateway {
    if let Some(slug) = &instance.gateway_slug {
        let gateway_manager = GatewayManager::new(gateway_config.clone());

        // Remove gateway configuration
        gateway_manager.cleanup_vm_gateway(slug).await?;

        // Delete DNS record via central API
        api_client.delete_dns_record(slug).await?;
    }
}

// Then destroy VM...
```

#### 3.3 Add DNS API Endpoints

**File:** `api-server/src/routes/agents.rs`

Add endpoint for agents to manage DNS:

```rust
#[post("/agents/dns")]
async fn manage_dns(
    auth: AgentAuth,
    payload: Json<DnsRequest>,
) -> Result<Json<DnsResponse>> {
    // Verify agent has DnsManage permission
    auth.require_permission(Permission::DnsManage)?;

    match payload.action {
        DnsAction::Create => {
            cloudflare_client
                .create_a_record(&payload.slug, &payload.datacenter, &payload.public_ip)
                .await?
        }
        DnsAction::Delete => {
            cloudflare_client
                .delete_a_record(&payload.slug, &payload.datacenter)
                .await?
        }
    }

    Ok(Json(DnsResponse { success: true }))
}
```

#### 3.4 Database Migration

**File:** `migrations/YYYYMMDD_add_gateway_fields.sql`

```sql
ALTER TABLE contract_sign_requests
    ADD COLUMN gateway_slug TEXT,
    ADD COLUMN gateway_ssh_port INTEGER,
    ADD COLUMN gateway_port_range_start INTEGER,
    ADD COLUMN gateway_port_range_end INTEGER;

CREATE UNIQUE INDEX idx_gateway_slug
    ON contract_sign_requests(gateway_slug)
    WHERE gateway_slug IS NOT NULL;
```

#### 3.5 Update API Responses

Ensure `ContractDetail` response includes gateway fields:

```rust
#[derive(Serialize)]
pub struct ContractDetail {
    // ... existing fields ...

    pub gateway_slug: Option<String>,
    pub gateway_subdomain: Option<String>,
    pub gateway_ssh_port: Option<u16>,
    pub gateway_port_range_start: Option<u16>,
    pub gateway_port_range_end: Option<u16>,
}
```

#### 3.6 Update UI

**File:** `frontend/src/components/ContractDetails.tsx` (or equivalent)

Display connection information:

```
┌─────────────────────────────────────────────┐
│ Connection Details                          │
├─────────────────────────────────────────────┤
│ Web Access:  https://k7m2p4.dc-lk.decent-cloud.org
│                                             │
│ SSH Access:  ssh root@k7m2p4.dc-lk.decent-cloud.org -p 20000
│                                             │
│ Available Ports: 20001-20009                │
│   TCP: 20001-20004 → VM:10001-10004         │
│   UDP: 20005-20009 → VM:10005-10009         │
└─────────────────────────────────────────────┘
```

### Phase 4: Testing

#### 4.1 Unit Tests

- [ ] Port allocation edge cases (full range, fragmentation)
- [ ] Traefik config YAML generation
- [ ] iptables rule generation
- [ ] Slug generation uniqueness

#### 4.2 Integration Tests

- [ ] Provision VM → verify gateway setup
- [ ] Terminate VM → verify gateway cleanup
- [ ] DNS record creation via API
- [ ] DNS record deletion via API

#### 4.3 End-to-End Tests

```bash
# 1. Provision a test VM
dc-agent provision --contract-id test-123

# 2. Verify DNS record exists
dig k7m2p4.dc-lk.decent-cloud.org

# 3. Verify HTTPS works
curl https://k7m2p4.dc-lk.decent-cloud.org

# 4. Verify SSH works
ssh -p 20000 root@k7m2p4.dc-lk.decent-cloud.org

# 5. Verify port forwarding works
# Start listener on VM port 10001
nc -l 10001  # on VM

# Connect from external
nc k7m2p4.dc-lk.decent-cloud.org 20001

# 6. Terminate VM
dc-agent terminate --contract-id test-123

# 7. Verify cleanup
dig k7m2p4.dc-lk.decent-cloud.org  # Should return NXDOMAIN
```

## Task Checklist

### Pre-requisites (Manual)
- [ ] Assign public IP to Proxmox host interface
- [ ] Ensure SSH access to host with root privileges
- [ ] Create Cloudflare API token with DNS edit permission

### Infrastructure (Automated by `dc-agent setup gateway`)
- [x] Enable IP forwarding (`net.ipv4.ip_forward=1`)
- [x] Configure NAT masquerade for VM subnet
- [x] Open firewall ports (80, 443, 20000-59999)
- [x] Configure FORWARD rules for VM traffic
- [x] Persist iptables rules
- [x] Install Traefik binary
- [x] Configure Traefik with wildcard TLS via Cloudflare DNS challenge
- [x] Start Traefik systemd service

### To Deploy
```bash
dc-agent setup gateway \
  --host <PROXMOX_HOST> \
  --datacenter <DC_ID> \
  --domain decent-cloud.org \
  --public-ip <PUBLIC_IP> \
  --cloudflare-token <CF_API_TOKEN>
```

### Code Integration
- [x] Wire `GatewayManager::setup_gateway()` into provisioning (`dc-agent/src/main.rs:1511-1535`)
- [x] Wire `GatewayManager::cleanup_gateway()` into termination (`dc-agent/src/main.rs:1681-1693`)
- [x] Implement `POST /api/v1/agents/dns` endpoint in api-server (`api/src/openapi/agents.rs:577-679`)
- [x] Implement Cloudflare client for DNS record management (`api/src/cloudflare_dns.rs`)
- [x] Add database migration for gateway columns (`api/migrations_pg/001_schema.sql`)
- [x] Update `Contract` API response (`api/src/database/contracts.rs`)
- [x] Update frontend to display connection details (`website/src/routes/dashboard/rentals/[contract_id]/+page.svelte:546-568`)

### Testing
- [x] Add unit tests for gateway components (`dc-agent/src/gateway/**/tests`)
- [x] Add unit tests for DNS API validation (`api/src/openapi/agents.rs::tests`)
- [x] Add unit tests for gateway fields in contracts (`api/src/database/contracts/tests.rs`)
- [ ] Run end-to-end test on real infrastructure

## Security Considerations

1. **Firewall before gateway**: Ensure host firewall is configured before exposing gateway ports
2. **Cloudflare token scope**: Token should only have DNS edit permission, not full account
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

   # Remove Traefik configs
   sudo rm /etc/traefik/dynamic/vm-*.yaml

   # Stop Traefik
   sudo systemctl stop traefik
   ```

## Future Enhancements

- Custom domain support (user brings their own domain)
- Per-VM bandwidth monitoring and limits
- Premium tier with dedicated public IP
- Geographic DNS routing for multi-DC

## References

- [DC Gateway Spec](./2025-12-31-dc-gateway-spec.md) - Architecture details
- [Traefik Documentation](https://doc.traefik.io/traefik/)
- [Cloudflare API](https://developers.cloudflare.com/api/)
