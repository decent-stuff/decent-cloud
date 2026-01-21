# Proxmox API Exploration for Auto-Generated Offerings

**Date:** 2026-01-21
**Purpose:** Document what Proxmox API data is available for auto-generating VPS offerings

## Executive Summary

The dc-agent currently uses ~15 Proxmox API endpoints focused on VM lifecycle (clone, configure, start, stop, delete). However, **none of the endpoints for querying node resources, storage capacity, or available templates are implemented**. This document catalogs available Proxmox APIs that could feed automatic offering generation.

## Current State

### What dc-agent Reports in Heartbeat

| Field | Value | Used for Offerings? |
|-------|-------|---------------------|
| `version` | Agent version string | No |
| `provisioner_type` | "proxmox" | No |
| `capabilities` | Always `None` (unused) | **No - empty!** |
| `active_contracts` | Count of running VMs | No |
| `bandwidth_stats` | Per-VM bandwidth | No |

### Current Proxmox API Endpoints (Implemented)

```
VM Lifecycle:
  POST /nodes/{node}/qemu/{vmid}/clone      - Clone template
  PUT  /nodes/{node}/qemu/{vmid}/config     - Configure VM
  PUT  /nodes/{node}/qemu/{vmid}/resize     - Resize disk
  POST /nodes/{node}/qemu/{vmid}/status/start
  POST /nodes/{node}/qemu/{vmid}/status/stop
  DELETE /nodes/{node}/qemu/{vmid}

Monitoring:
  GET /nodes/{node}/qemu/{vmid}/status/current
  GET /nodes/{node}/qemu                    - List all VMs
  GET /nodes/{node}/qemu/{vmid}/agent/network-get-interfaces

Task Management:
  GET /nodes/{node}/tasks/{upid}/status
  GET /nodes/{node}/tasks/{upid}/log

Verification:
  GET /version
  GET /nodes/{node}/storage/{storage}       - Verify storage exists (no details)
  GET /pools/{pool}                         - Verify pool exists
```

## Available But Unused Endpoints

### 1. Cluster Resources (Most Useful!)

**Endpoint:** `GET /cluster/resources`

**Why it matters:** Single call returns all nodes, VMs, and storage with resource metrics.

**Response fields for nodes:**
```json
{
  "node": "pve1",
  "type": "node",
  "status": "online",
  "cpu": 0.0061,           // Current CPU utilization (0-1)
  "maxcpu": 8,             // Total CPU cores
  "mem": 1321598976,       // Used memory (bytes)
  "maxmem": 16675291136,   // Total memory (bytes)
  "disk": 45155565568,     // Used disk (bytes)
  "maxdisk": 67799756800,  // Total disk (bytes)
  "uptime": 7223           // Seconds since boot
}
```

**Response fields for storage:**
```json
{
  "type": "storage",
  "storage": "local-lvm",
  "node": "pve1",
  "content": "images,rootdir",
  "disk": 10737418240,     // Used (bytes)
  "maxdisk": 107374182400, // Total (bytes)
  "status": "available"
}
```

**Auto-generation potential:**
- Calculate available CPU cores for new VMs
- Calculate available RAM for new VMs
- Calculate available storage for new VMs
- Generate offering tiers based on available capacity

---

### 2. Node Status

**Endpoint:** `GET /nodes/{node}/status`

**Why it matters:** Detailed node information including CPU model.

**Response fields:**
```json
{
  "cpuinfo": {
    "model": "AMD EPYC 7763 64-Core Processor",
    "cores": 64,
    "cpus": 128,           // Total threads
    "mhz": "2450.000",
    "sockets": 2
  },
  "memory": {
    "total": 270582939648,
    "used": 12884901888,
    "free": 257698037760
  },
  "rootfs": {
    "total": 107374182400,
    "used": 10737418240,
    "free": 96636764160
  },
  "uptime": 1234567,
  "kversion": "Linux 6.8.12-1-pve #1 SMP"
}
```

**Auto-generation potential:**
- `processor_brand`: Extract "AMD" or "Intel" from model
- `processor_name`: Full CPU model name
- `processor_cores`: From cpuinfo.cores
- `processor_speed`: From cpuinfo.mhz
- `memory_amount`: From memory.total (convert to GB)

---

### 3. Storage Details

**Endpoint:** `GET /nodes/{node}/storage`

**Why it matters:** Lists all storage pools with capacity info.

**Response fields:**
```json
[
  {
    "storage": "local-lvm",
    "type": "lvmthin",
    "content": "images,rootdir",
    "total": 107374182400,
    "used": 10737418240,
    "avail": 96636764160,
    "active": 1,
    "enabled": 1
  }
]
```

**Auto-generation potential:**
- Calculate max disk sizes for offerings
- Identify storage type (SSD vs HDD based on naming/type)
- Check available space before accepting contracts

---

### 4. Storage Content (Templates)

**Endpoint:** `GET /nodes/{node}/storage/{storage}/content`

**Why it matters:** Lists available VM templates.

**Response fields:**
```json
[
  {
    "volid": "local:iso/ubuntu-22.04.iso",
    "content": "iso",
    "format": "iso",
    "size": 1234567890
  },
  {
    "volid": "local-lvm:vm-9000-disk-0",
    "content": "images",
    "vmid": 9000,
    "format": "raw",
    "size": 10737418240
  }
]
```

**Auto-generation potential:**
- `operating_systems`: List available OS templates
- `template_name`: Map template VMIDs to OS names

---

### 5. PCI/GPU Devices

**Endpoint:** `GET /nodes/{node}/hardware/pci`

**Why it matters:** Lists GPUs available for passthrough.

**Query parameter:** `--pci-class-blacklist ""` to show all devices

**Response fields:**
```json
[
  {
    "id": "0000:01:00.0",
    "class": "0x030000",        // VGA controller
    "device_name": "NVIDIA GeForce RTX 4090",
    "vendor_name": "NVIDIA Corporation",
    "iommu_group": 15,
    "subsystem_device": "0x16c1",
    "subsystem_vendor": "0x10de"
  }
]
```

**Auto-generation potential:**
- `gpu_name`: From device_name
- `gpu_count`: Count devices with class 0x030000 (VGA)
- `product_type`: Set to "gpu" if GPUs available

---

### 6. Network/Bridge Information

**Endpoint:** `GET /nodes/{node}/network`

**Why it matters:** Lists available network bridges.

**Response fields:**
```json
[
  {
    "iface": "vmbr0",
    "type": "bridge",
    "cidr": "192.168.1.1/24",
    "gateway": "192.168.1.254",
    "bridge_ports": "enp1s0",
    "active": 1
  }
]
```

**Auto-generation potential:**
- Validate network configuration
- Determine available network interfaces for VMs

---

## Mapping Proxmox Data to Offering Fields

| Offering Field | Proxmox Source | Endpoint |
|----------------|----------------|----------|
| `processor_brand` | Parse from cpuinfo.model | `/nodes/{node}/status` |
| `processor_name` | cpuinfo.model | `/nodes/{node}/status` |
| `processor_cores` | cpuinfo.cores (per offering tier) | `/nodes/{node}/status` |
| `processor_speed` | cpuinfo.mhz | `/nodes/{node}/status` |
| `memory_amount` | memory.total (calculate tiers) | `/nodes/{node}/status` |
| `total_ssd_capacity` | Calculate from storage.avail | `/nodes/{node}/storage` |
| `gpu_name` | device_name | `/nodes/{node}/hardware/pci` |
| `gpu_count` | Count VGA class devices | `/nodes/{node}/hardware/pci` |
| `operating_systems` | List template names | `/nodes/{node}/storage/{s}/content` |
| `datacenter_*` | Already in agent config | N/A |

## Permissions Required

The API token needs **Sys.Audit** privilege to get full resource information. Without it, some fields return empty or partial data.

Current token setup in `dc-agent/src/setup/proxmox.rs` creates:
```
PVE.Pool = ["VM.Allocate", "VM.Clone", "VM.Config.Disk", ...]
PVE.Storage = ["Datastore.AllocateSpace", ...]
```

**Missing:** `Sys.Audit` for reading node resources. This should be added.

## Recommended Implementation Order

1. **Add Sys.Audit permission** to token setup
2. **Implement `/cluster/resources`** - single call, most data
3. **Implement `/nodes/{node}/status`** - CPU model details
4. **Implement `/nodes/{node}/hardware/pci`** - GPU discovery
5. **Implement storage content listing** - template discovery

## Next Steps

This document feeds into:
- **Session 2:** Design auto-generation system (how offerings derive from capabilities)
- **Session 3:** Implement capability reporting (heartbeat changes)

## Sources

- [Proxmox VE API Documentation](https://pve.proxmox.com/wiki/Proxmox_VE_API)
- [Proxmox API Resources Forum](https://forum.proxmox.com/threads/proxmox-api-resources.161087/)
- [Hypervisor Resource Overview](https://forum.proxmox.com/threads/hypervisor-resource-overview-using-api.109028/)
- [Proxmox PCI Passthrough](https://pve.proxmox.com/wiki/PCI(e)_Passthrough)
- [Storage Status API](https://deepwiki.com/proxmox/pve-storage/4.3-storage-status-and-import-api)
