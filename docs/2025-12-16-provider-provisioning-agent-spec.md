# Provider Provisioning Agent Implementation
**Status:** In Progress
**Date:** 2025-12-16

## Overview

Implement a generic `dc-agent` binary that supports multiple provisioner backends. Starting with Proxmox as the first real provisioner, with a script-based provisioner for custom integrations.

## Requirements

### Must-have
- [ ] Generic agent crate with pluggable provisioner architecture
- [ ] Proxmox VE provisioner (clone from template, start/stop, health check)
- [ ] Script provisioner (bash/python/any language via stdin/stdout JSON)
- [ ] Manual provisioner (notifications only, dashboard input)
- [ ] Polling-based API client with Ed25519 authentication
- [ ] Configuration via TOML file
- [ ] Unit tests with mocked Proxmox API responses (real API format)
- [ ] API extension: `GET /api/v1/providers/{pubkey}/contracts/pending-provision`

### Nice-to-have
- [ ] Integration test harness for real Proxmox
- [ ] Health check reporting to API
- [ ] Credential encryption with requester's pubkey

## Architecture

```
dc-agent (single binary)
├── main.rs           - Entry point, config loading, polling loop
├── config.rs         - TOML configuration parsing
├── api_client.rs     - Decent Cloud API client with auth
├── provisioner/
│   ├── mod.rs        - Provisioner trait definition
│   ├── proxmox.rs    - Proxmox VE implementation
│   ├── script.rs     - External script provisioner
│   └── manual.rs     - Notification-only provisioner
└── tests/
    ├── proxmox_mock.rs - Mocked Proxmox API tests
    └── integration.rs  - Real Proxmox tests (requires env)
```

## Proxmox API Reference

Based on official [Proxmox VE API documentation](https://pve.proxmox.com/wiki/Proxmox_VE_API).

### Authentication

**API Token (recommended):**
```
Authorization: PVEAPIToken=USER@REALM!TOKENID=UUID
```

No CSRF token required for API tokens.

### Key Endpoints

#### Clone VM from Template
```
POST /api2/json/nodes/{node}/qemu/{vmid}/clone

Parameters:
- newid: integer (100-999999999) - Target VM ID [required]
- name: string - VM display name
- full: boolean - Full clone (true) vs linked clone (false)
- target: string - Target node (shared storage only)
- storage: string - Target storage for full clone
- pool: string - Resource pool assignment

Response:
{
  "data": "UPID:node:pid:pstart:starttime:type:id:user:"
}
```

#### Configure VM (cloud-init)
```
PUT /api2/json/nodes/{node}/qemu/{vmid}/config

Parameters:
- ciuser: string - Cloud-init user
- cipassword: string - Cloud-init password
- sshkeys: string - URL-encoded SSH public keys
- ipconfig0: string - IP configuration (e.g., "ip=dhcp" or "ip=10.0.0.5/24,gw=10.0.0.1")
- nameserver: string - DNS server
- cores: integer - CPU cores
- memory: integer - RAM in MiB
```

#### Start VM
```
POST /api2/json/nodes/{node}/qemu/{vmid}/status/start

Response:
{
  "data": "UPID:..."
}
```

#### Stop VM
```
POST /api2/json/nodes/{node}/qemu/{vmid}/status/stop
```

#### Get VM Status
```
GET /api2/json/nodes/{node}/qemu/{vmid}/status/current

Response:
{
  "data": {
    "vmid": 100,
    "name": "vm-name",
    "status": "running",  // or "stopped"
    "qmpstatus": "running",
    "cpus": 2,
    "maxmem": 2147483648,
    "maxdisk": 10737418240,
    "uptime": 3600,
    "netin": 1234567,
    "netout": 7654321,
    "diskread": 123456,
    "diskwrite": 654321,
    "cpu": 0.05,
    "mem": 536870912,
    "pid": 12345,
    "ha": {"managed": 0}
  }
}
```

#### Delete VM
```
DELETE /api2/json/nodes/{node}/qemu/{vmid}

Parameters:
- purge: boolean - Remove from all related configurations
- destroy-unreferenced-disks: boolean - Delete unreferenced disks
```

#### Get Task Status (for async operations)
```
GET /api2/json/nodes/{node}/tasks/{upid}/status

Response:
{
  "data": {
    "status": "running",  // or "stopped"
    "exitstatus": "OK",   // when stopped
    "type": "qmclone",
    "user": "root@pam",
    "starttime": 1702742400,
    "node": "pve1"
  }
}
```

#### Get VM Network Interfaces (for IP discovery)
```
GET /api2/json/nodes/{node}/qemu/{vmid}/agent/network-get-interfaces

Response:
{
  "data": {
    "result": [
      {
        "name": "eth0",
        "ip-addresses": [
          {"ip-address": "10.0.0.100", "ip-address-type": "ipv4", "prefix": 24}
        ],
        "hardware-address": "aa:bb:cc:dd:ee:ff"
      }
    ]
  }
}
```

Note: Requires QEMU Guest Agent running in VM.

## Configuration Format

```toml
# dc-agent.toml

[api]
endpoint = "https://api.decent-cloud.org"
provider_pubkey = "ed25519_pubkey_hex"
provider_secret_key = "ed25519_secret_hex"  # Or path to key file

[polling]
interval_seconds = 30
health_check_interval_seconds = 300

[provisioner]
type = "proxmox"  # or "script", "manual"

# Proxmox-specific configuration
[provisioner.proxmox]
api_url = "https://proxmox.local:8006"
api_token_id = "root@pam!dc-agent"
api_token_secret = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
node = "pve1"
template_vmid = 9000  # Template to clone from
storage = "local-lvm"  # Target storage for clones
pool = "dc-vms"  # Optional: resource pool
verify_ssl = false  # For self-signed certs

# Script-based provisioner (alternative)
[provisioner.script]
provision = "/opt/dc-agent/provision.sh"
terminate = "/opt/dc-agent/terminate.sh"
health_check = "/opt/dc-agent/health.sh"
timeout_seconds = 300

# Manual provisioner (alternative)
[provisioner.manual]
notification_webhook = "https://slack.webhook/..."  # Optional
```

## Script Provisioner Protocol

Scripts receive JSON on stdin, output JSON on stdout.

### Provision Request (stdin)
```json
{
  "action": "provision",
  "contract_id": "abc123",
  "offering": {
    "id": "off-123",
    "cpu_cores": 2,
    "memory_gb": 4,
    "storage_gb": 50,
    "bandwidth_mbps": 100
  },
  "requester_ssh_pubkey": "ssh-ed25519 AAAA...",
  "instance_config": {}
}
```

### Provision Response (stdout)
```json
{
  "success": true,
  "instance": {
    "external_id": "vm-12345",
    "ip_address": "10.0.0.100",
    "ipv6_address": "2001:db8::100",
    "ssh_port": 22,
    "root_password": "generated_password"
  }
}
```

### Error Response
```json
{
  "success": false,
  "error": "Out of storage space",
  "retry_possible": true
}
```

## Steps

### Step 1: Create agent crate structure
**Success:** Cargo.toml exists, compiles, workspace member
**Status:** Pending

### Step 2: Implement configuration parsing
**Success:** TOML config loads, validates required fields
**Status:** Pending

### Step 3: Implement Provisioner trait
**Success:** Trait defined with provision/terminate/health_check methods
**Status:** Pending

### Step 4: Implement Proxmox provisioner
**Success:** Can clone VM, configure, start, get status, terminate
**Status:** Pending

### Step 5: Implement Script provisioner
**Success:** Executes external scripts, parses JSON responses
**Status:** Pending

### Step 6: Implement API client
**Success:** Authenticates, fetches pending contracts, reports status
**Status:** Pending

### Step 7: Implement polling loop
**Success:** Polls API, provisions contracts, handles errors
**Status:** Pending

### Step 8: Add API extension endpoint
**Success:** GET /providers/{pubkey}/contracts/pending-provision works
**Status:** Pending

### Step 9: Unit tests with mocked Proxmox API
**Success:** Tests cover clone, start, stop, status, error cases
**Status:** Pending

### Step 10: Integration test harness
**Success:** Tests can run against real Proxmox when PROXMOX_TEST_URL set
**Status:** Pending

## Execution Log

### Step 1: Create agent crate structure
- **Implementation:** Created dc-agent crate with minimal skeleton
  - Created `/code/dc-agent/Cargo.toml` with workspace dependencies
  - Added `dc-agent` to workspace members in root `Cargo.toml`
  - Created directory structure:
    - `src/main.rs` - Entry point with clap CLI (run, doctor subcommands)
    - `src/lib.rs` - Module declarations
    - `src/config.rs` - Placeholder for step 2
    - `src/api_client.rs` - Placeholder for step 6
    - `src/provisioner/mod.rs` - Placeholder for step 3
    - `src/provisioner/proxmox.rs` - Placeholder for step 4
    - `src/provisioner/script.rs` - Placeholder for step 5
    - `src/provisioner/manual.rs` - Placeholder for later
  - CLI features:
    - `--config` option for config file path (default: dc-agent.toml)
    - `run` subcommand - prints "Agent starting..."
    - `doctor` subcommand - prints "Checking configuration..."
- **Files created:**
  - `/code/dc-agent/Cargo.toml`
  - `/code/dc-agent/src/main.rs`
  - `/code/dc-agent/src/lib.rs`
  - `/code/dc-agent/src/config.rs`
  - `/code/dc-agent/src/api_client.rs`
  - `/code/dc-agent/src/provisioner/mod.rs`
  - `/code/dc-agent/src/provisioner/proxmox.rs`
  - `/code/dc-agent/src/provisioner/script.rs`
  - `/code/dc-agent/src/provisioner/manual.rs`
- **Verification:** `cargo build -p dc-agent` compiles successfully, binary runs with --help, run, and doctor subcommands
- **Outcome:** Success - skeleton compiles and workspace member added

## Completion Summary
(To be filled in Phase 4)
