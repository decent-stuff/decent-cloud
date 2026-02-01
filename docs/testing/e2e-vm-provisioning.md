# End-to-End VM Provisioning Testing

This document describes how to test the full VM provisioning flow from contract creation to SSH access.

## Overview

The provisioning flow:
1. User creates a rental contract via website/API
2. Contract gets accepted (auto-accept or manual)
3. Payment succeeds (test with fiat/Stripe or crypto)
4. dc-agent polls API and picks up the contract
5. dc-agent provisions VM via Proxmox
6. dc-agent sets up gateway (DNS, iptables, Caddy)
7. User can SSH via gateway port

## Prerequisites

### 1. dc-agent Running on Proxmox Host

```bash
# SSH to Proxmox host
ssh root@<proxmox-ip>

# Verify dc-agent is configured and healthy
dc-agent doctor

# Run dc-agent (or ensure systemd service is running)
dc-agent run
```

### 2. Provider Offering Exists

The provider must have an offering in the API. Check via:
```bash
curl -s https://dev-api.decent-cloud.org/api/v1/offerings | jq '.data[] | {id, offer_name, provider_online}'
```

Look for offerings where `provider_online: true`.

## Testing Methods

### Method 1: Via Website (Recommended for Full E2E)

1. Go to https://dev.decent-cloud.org (dev) or https://decent-cloud.org (prod)
2. Log in with Internet Identity or create an account
3. Navigate to "Rent a Server"
4. Select an offering from an online provider
5. Enter your SSH public key
6. Complete payment
7. Monitor dc-agent logs on Proxmox host:
   ```bash
   ssh root@<proxmox-ip> 'journalctl -u dc-agent -f'
   ```
8. Once provisioned, SSH via gateway:
   ```bash
   ssh -p <gateway-port> ubuntu@<gateway-subdomain>.dc-<datacenter>.decent-cloud.org
   ```

### Method 2: Via dc-agent test-provision (Local Testing)

For testing the provisioning without going through payment flow:

```bash
# SSH to Proxmox host
ssh root@<proxmox-ip>

# Test provisioning with gateway (skips DNS for local testing)
dc-agent test-provision \
  --ssh-pubkey "ssh-ed25519 AAAA... your-key" \
  --test-gateway \
  --skip-dns \
  --keep \
  --contract-id test-vm-$(date +%s)

# This will:
# - Clone VM from template
# - Wait for IP address
# - Setup iptables port forwarding
# - Skip DNS (for local testing)
# - Keep VM running for inspection

# Test SSH access via gateway port shown in output
ssh -p 20000 ubuntu@<proxmox-ip>

# Cleanup when done (use Proxmox UI or API to delete the test VM)
```

### Method 3: Direct Database Injection (Dev Only)

For testing dc-agent without going through the full API flow:

```bash
# Connect to dev database
psql $DATABASE_URL_PG

# Insert a test contract with status='accepted' and payment_status='succeeded'
# The dc-agent will pick it up on next poll

-- See api/src/database/contracts.rs for schema details
```

## Verifying Gateway Functionality

### Check iptables Rules

```bash
ssh root@<proxmox-ip> 'iptables -t nat -L DC_GATEWAY -n --line-numbers'
```

Expected output:
```
Chain DC_GATEWAY (1 references)
num  target     prot opt source    destination
1    DNAT       tcp  --  0.0.0.0/0 0.0.0.0/0  tcp dpt:20000 to:172.16.0.x:22
2    DNAT       tcp  --  0.0.0.0/0 0.0.0.0/0  tcp dpt:20001 to:172.16.0.x:10001
...
```

### Check Port Allocations

```bash
ssh root@<proxmox-ip> 'cat /var/lib/dc-agent/port-allocations.json'
```

### Test SSH via Gateway

```bash
# Direct IP + port
ssh -p <gateway-port> ubuntu@<proxmox-ip>

# Via subdomain (requires DNS to be set up)
ssh -p <gateway-port> ubuntu@<slug>.dc-<datacenter>.decent-cloud.org
```

### Test TCP Port Forwarding

```bash
# On VM: start a listener on port 10001
ssh -p <gateway-port> ubuntu@<proxmox-ip> 'nc -l -p 10001'

# From outside: connect to external port (gateway-port + 1)
echo "test" | nc <proxmox-ip> <gateway-port+1>
```

## Simulating Host Reboot

Test that gateway rules survive a reboot:

```bash
# Clear iptables rules (simulates reboot)
ssh root@<proxmox-ip> 'iptables -t nat -F DC_GATEWAY'

# Verify SSH fails
ssh -p <gateway-port> ubuntu@<proxmox-ip>  # Should fail

# Restart dc-agent - rules should be restored
ssh root@<proxmox-ip> 'systemctl restart dc-agent'

# Verify SSH works again
ssh -p <gateway-port> ubuntu@<proxmox-ip>  # Should work
```

## Troubleshooting

### dc-agent Not Picking Up Contracts

1. Check dc-agent is running: `systemctl status dc-agent`
2. Check API connectivity: `dc-agent doctor --no-test-provision`
3. Check logs: `journalctl -u dc-agent -f`
4. Verify contract status in database is `accepted` and payment_status is `succeeded`

### VM Provisioned but SSH Fails

1. Check VM has IP: `qm guest cmd <vmid> network-get-interfaces`
2. Check iptables rules exist: `iptables -t nat -L DC_GATEWAY -n`
3. Check MASQUERADE rule: `iptables -t nat -L POSTROUTING -n`
4. Check bridge-nf-call-iptables: `sysctl net.bridge.bridge-nf-call-iptables` (should be 1)

### Gateway Rules Not Restored After Reboot

1. Check port-allocations.json has `internal_ip` field (required for restore)
2. Check dc-agent logs for "Restoring iptables rules" message
3. Verify dc-agent version is >= 0.4.9 (added restore functionality)

## Key Files

- **dc-agent config**: `/etc/dc-agent/dc-agent.toml`
- **Port allocations**: `/var/lib/dc-agent/port-allocations.json`
- **Caddy sites**: `/etc/caddy/sites/`
- **Orphan tracking**: `/var/lib/dc-agent/orphans.json`

## For AI Agents

When testing VM provisioning programmatically:

1. **Prefer `test-provision`** for quick local tests that don't need payment
2. **Use `--test-gateway --skip-dns --keep`** flags for gateway testing
3. **Check logs** via `journalctl -u dc-agent -f` to monitor progress
4. **Verify SSH** after provisioning to confirm full flow works
5. **Clean up** test VMs via Proxmox UI or `qm destroy <vmid> --purge`
