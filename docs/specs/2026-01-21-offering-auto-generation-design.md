# Offering Auto-Generation System Design

**Date:** 2026-01-21
**Prerequisite:** [Proxmox API Exploration](./2026-01-21-proxmox-api-exploration.md)
**Status:** Design (pending implementation)

## Problem Statement

Currently, providers must manually create offerings with 40+ fields. The dc-agent has access to hardware information via Proxmox APIs but doesn't report it. We want to:

1. Have agents report hardware capabilities
2. Auto-generate sensible offering tiers based on available resources
3. Give providers control over what gets generated

## Design Principles

1. **Provider-controlled** - Auto-generation is triggered explicitly, not automatically
2. **Preview before commit** - Provider sees what will be created before confirming
3. **Minimal schema changes** - Extend existing patterns, don't reinvent
4. **Additive, not destructive** - Never modify/delete existing offerings

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         DATA FLOW                                    │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────┐     heartbeat      ┌──────────┐                       │
│  │ dc-agent │ ─────────────────► │ API      │                       │
│  │          │  + resources {}    │ Server   │                       │
│  └──────────┘                    └────┬─────┘                       │
│       │                               │                              │
│       │ queries                       │ stores                       │
│       ▼                               ▼                              │
│  ┌──────────┐                   ┌──────────────────┐                │
│  │ Proxmox  │                   │ provider_agent_  │                │
│  │ API      │                   │ status.resources │                │
│  └──────────┘                   └────────┬─────────┘                │
│                                          │                           │
│                                          │ aggregated                │
│                                          ▼                           │
│  ┌──────────┐    POST /generate   ┌──────────────┐                  │
│  │ Provider │ ◄─────────────────► │ Pool         │                  │
│  │ UI/CLI   │    offerings        │ Capabilities │                  │
│  └────┬─────┘                     └──────┬───────┘                  │
│       │                                  │                           │
│       │ confirms                         │ tier logic                │
│       ▼                                  ▼                           │
│  ┌──────────────────────────────────────────────┐                   │
│  │              provider_offerings               │                   │
│  │         (offering_source = 'generated')       │                   │
│  └──────────────────────────────────────────────┘                   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## Component Design

### 1. Resource Inventory (Agent → API)

**New struct in heartbeat:**

```rust
// dc-agent/src/api_client.rs
#[derive(Serialize)]
pub struct ResourceInventory {
    // CPU
    pub cpu_model: Option<String>,        // "AMD EPYC 7763 64-Core Processor"
    pub cpu_cores: u32,                   // Physical cores
    pub cpu_threads: u32,                 // Logical threads
    pub cpu_mhz: Option<u32>,             // Clock speed

    // Memory
    pub memory_total_mb: u64,             // Total RAM in MB
    pub memory_available_mb: u64,         // Available (not committed to VMs)

    // Storage (per pool)
    pub storage_pools: Vec<StoragePoolInfo>,

    // GPU (if any)
    pub gpu_devices: Vec<GpuDeviceInfo>,

    // Templates available
    pub templates: Vec<TemplateInfo>,
}

#[derive(Serialize)]
pub struct StoragePoolInfo {
    pub name: String,                     // "local-lvm"
    pub total_gb: u64,
    pub available_gb: u64,
    pub storage_type: String,             // "lvmthin", "zfspool", "dir"
}

#[derive(Serialize)]
pub struct GpuDeviceInfo {
    pub pci_id: String,                   // "0000:01:00.0"
    pub name: String,                     // "NVIDIA GeForce RTX 4090"
    pub vendor: String,                   // "NVIDIA Corporation"
    pub memory_mb: Option<u32>,           // VRAM if detectable
}

#[derive(Serialize)]
pub struct TemplateInfo {
    pub vmid: u32,
    pub name: String,                     // "ubuntu-22.04"
}
```

**Extended heartbeat request:**

```rust
pub struct HeartbeatRequest {
    pub version: Option<String>,
    pub provisioner_type: Option<String>,
    pub capabilities: Option<Vec<String>>,
    pub active_contracts: i64,
    pub bandwidth_stats: Option<Vec<VmBandwidthReport>>,
    pub resources: Option<ResourceInventory>,  // NEW
}
```

### 2. Schema Changes

**Option A: Extend provider_agent_status (Recommended)**

```sql
ALTER TABLE provider_agent_status
ADD COLUMN resources JSONB;

-- Index for querying GPU-capable agents
CREATE INDEX idx_agent_status_has_gpu
ON provider_agent_status ((resources->'gpu_devices') IS NOT NULL AND (resources->'gpu_devices')::jsonb != '[]'::jsonb);
```

**Why JSONB:**
- Flexible schema for different provisioner types
- Easy to extend without migrations
- Supports queries like "find agents with GPUs"

### 3. Pool Capability Aggregation

**New struct for aggregated pool capabilities:**

```rust
// api/src/database/agent_pools.rs
pub struct PoolCapabilities {
    pub pool_id: String,

    // Aggregated from online agents
    pub total_cpu_cores: u32,
    pub total_memory_mb: u64,
    pub total_storage_gb: u64,

    // Minimums (smallest agent determines max single-VM size)
    pub min_agent_cpu_cores: u32,
    pub min_agent_memory_mb: u64,
    pub min_agent_storage_gb: u64,

    // Common capabilities
    pub cpu_models: Vec<String>,          // Unique CPU models across agents
    pub has_gpu: bool,
    pub gpu_models: Vec<String>,          // Unique GPU models
    pub available_templates: Vec<String>, // Union of all agent templates

    // Agent count
    pub online_agents: u32,
}
```

**Query to compute:**

```sql
SELECT
    p.pool_id,
    COUNT(DISTINCT d.agent_pubkey) as online_agents,
    SUM((s.resources->>'cpu_cores')::int) as total_cpu_cores,
    MIN((s.resources->>'cpu_cores')::int) as min_agent_cpu_cores,
    SUM((s.resources->>'memory_total_mb')::bigint) as total_memory_mb,
    MIN((s.resources->>'memory_total_mb')::bigint) as min_agent_memory_mb,
    -- ... etc
FROM agent_pools p
JOIN provider_agent_delegations d ON d.pool_id = p.pool_id AND d.revoked_at_ns IS NULL
JOIN provider_agent_status s ON s.provider_pubkey = d.agent_pubkey
WHERE p.pool_id = $1
  AND s.online = TRUE
  AND s.last_heartbeat_ns > $2
  AND s.resources IS NOT NULL
GROUP BY p.pool_id;
```

### 4. Tier Logic

**Default offering tiers (provider can customize):**

```rust
pub struct OfferingTier {
    pub name: String,           // "small", "medium", "large", "gpu-small"
    pub display_name: String,   // "Basic VPS", "Performance VPS"
    pub cpu_cores: u32,
    pub memory_gb: u32,
    pub storage_gb: u32,
    pub gpu_count: Option<u32>,

    // Requirements to enable this tier
    pub min_pool_cpu: u32,      // Pool must have at least this many cores
    pub min_pool_memory_gb: u32,
    pub min_pool_storage_gb: u32,
}

// Default tiers
const DEFAULT_COMPUTE_TIERS: &[OfferingTier] = &[
    OfferingTier {
        name: "small",
        display_name: "Basic VPS",
        cpu_cores: 1,
        memory_gb: 2,
        storage_gb: 25,
        gpu_count: None,
        min_pool_cpu: 4,
        min_pool_memory_gb: 8,
        min_pool_storage_gb: 100,
    },
    OfferingTier {
        name: "medium",
        display_name: "Standard VPS",
        cpu_cores: 2,
        memory_gb: 4,
        storage_gb: 50,
        gpu_count: None,
        min_pool_cpu: 8,
        min_pool_memory_gb: 16,
        min_pool_storage_gb: 200,
    },
    OfferingTier {
        name: "large",
        display_name: "Performance VPS",
        cpu_cores: 4,
        memory_gb: 8,
        storage_gb: 100,
        gpu_count: None,
        min_pool_cpu: 16,
        min_pool_memory_gb: 32,
        min_pool_storage_gb: 400,
    },
    OfferingTier {
        name: "xlarge",
        display_name: "High Performance VPS",
        cpu_cores: 8,
        memory_gb: 16,
        storage_gb: 200,
        gpu_count: None,
        min_pool_cpu: 32,
        min_pool_memory_gb: 64,
        min_pool_storage_gb: 800,
    },
];

const DEFAULT_GPU_TIERS: &[OfferingTier] = &[
    OfferingTier {
        name: "gpu-small",
        display_name: "GPU Instance",
        cpu_cores: 4,
        memory_gb: 16,
        storage_gb: 100,
        gpu_count: Some(1),
        min_pool_cpu: 8,
        min_pool_memory_gb: 32,
        min_pool_storage_gb: 200,
    },
];
```

**Tier selection logic:**

```rust
fn select_applicable_tiers(
    capabilities: &PoolCapabilities,
    custom_tiers: Option<&[OfferingTier]>,
) -> Vec<OfferingTier> {
    let tiers = custom_tiers.unwrap_or(DEFAULT_COMPUTE_TIERS);

    let mut applicable = Vec::new();

    for tier in tiers {
        // Check pool has enough total resources for at least a few VMs
        if capabilities.total_cpu_cores >= tier.min_pool_cpu
            && capabilities.total_memory_mb >= tier.min_pool_memory_gb as u64 * 1024
            && capabilities.total_storage_gb >= tier.min_pool_storage_gb as u64
        {
            // Check smallest agent can host this tier
            if capabilities.min_agent_cpu_cores >= tier.cpu_cores
                && capabilities.min_agent_memory_mb >= tier.memory_gb as u64 * 1024
                && capabilities.min_agent_storage_gb >= tier.storage_gb as u64
            {
                applicable.push(tier.clone());
            }
        }
    }

    // Add GPU tiers if pool has GPUs
    if capabilities.has_gpu {
        for tier in DEFAULT_GPU_TIERS {
            // Similar checks...
            applicable.push(tier.clone());
        }
    }

    applicable
}
```

### 5. API Design

**New endpoints:**

```
# Preview what offerings would be generated
GET /api/v1/providers/{pubkey}/pools/{pool_id}/offering-suggestions
Response: {
    "pool_capabilities": PoolCapabilities,
    "suggested_offerings": [OfferingSuggestion],
    "unavailable_tiers": [{ "tier": "xlarge", "reason": "Insufficient CPU (need 32, have 16)" }]
}

# Generate offerings (requires confirmation)
POST /api/v1/providers/{pubkey}/pools/{pool_id}/generate-offerings
Body: {
    "tiers": ["small", "medium"],           // Optional: specific tiers to generate
    "pricing": {                            // Required: provider sets pricing
        "small": { "monthly_price": 5.0, "currency": "USD" },
        "medium": { "monthly_price": 15.0, "currency": "USD" }
    },
    "visibility": "public",                 // Optional: default "public"
    "operating_systems": ["ubuntu-22.04"],  // Optional: filter templates
    "dry_run": false                        // If true, return preview only
}
Response: {
    "created_offerings": [Offering],
    "skipped_tiers": [{ "tier": "gpu-small", "reason": "No pricing provided" }]
}
```

**Offering suggestion structure:**

```rust
pub struct OfferingSuggestion {
    pub tier_name: String,
    pub offering_id: String,           // Suggested: "{pool_id}-{tier_name}"
    pub offer_name: String,            // "Basic VPS (eu-proxmox)"

    // Pre-filled from capabilities
    pub processor_brand: Option<String>,
    pub processor_name: Option<String>,
    pub processor_cores: i64,
    pub memory_amount: String,         // "2 GB"
    pub total_ssd_capacity: String,    // "25 GB"
    pub gpu_name: Option<String>,
    pub gpu_count: Option<i64>,
    pub operating_systems: String,     // CSV of templates
    pub datacenter_country: String,    // From pool location
    pub datacenter_city: Option<String>,

    // Provider must set
    pub needs_pricing: bool,
}
```

### 6. Generated Offering Identification

**New offering_source value:**

```sql
-- Extend check constraint
ALTER TABLE provider_offerings
DROP CONSTRAINT IF EXISTS provider_offerings_offering_source_check;

ALTER TABLE provider_offerings
ADD CONSTRAINT provider_offerings_offering_source_check
CHECK (offering_source IN ('provider', 'seeded', 'generated'));
```

**Benefits:**
- Can filter/display generated offerings differently in UI
- Can re-generate (delete old generated, create new) without affecting manual offerings
- Analytics on auto vs manual offerings

### 7. When Auto-Generation Triggers

| Trigger | Action |
|---------|--------|
| Agent first reports resources | Pool shows "Suggestions available" badge in UI |
| Provider clicks "Generate Offerings" | Shows preview with pricing inputs |
| Provider confirms with pricing | Creates offerings |
| Agent resources change significantly | Pool shows "New suggestions available" |

**Not automatic because:**
- Pricing must come from provider (business decision)
- Provider may want specific naming/descriptions
- Prevents offering churn on resource fluctuation

### 8. UI/CLI Integration

**CLI command:**

```bash
# Preview suggestions
dc-cli offerings suggest --pool eu-proxmox

# Generate with pricing file
dc-cli offerings generate --pool eu-proxmox --pricing pricing.json

# pricing.json format:
{
  "small": { "monthly_price": 5.0, "currency": "USD" },
  "medium": { "monthly_price": 15.0, "currency": "USD" }
}
```

**UI flow:**

```
Pool Details Page
├── Stats: 3 agents online, 48 cores, 192GB RAM, 2TB storage
├── [Generate Offerings] button
│
└── Generate Offerings Modal
    ├── Available Tiers (checkboxes)
    │   ├── [x] Small (1 core, 2GB, 25GB) - Price: [___] USD/mo
    │   ├── [x] Medium (2 cores, 4GB, 50GB) - Price: [___] USD/mo
    │   ├── [ ] Large (4 cores, 8GB, 100GB) - Price: [___] USD/mo
    │   └── [x] GPU (4 cores, 16GB, 100GB, 1x RTX 4090) - Price: [___] USD/mo
    │
    ├── Unavailable Tiers (greyed out)
    │   └── XLarge - Need 32 cores (have 16)
    │
    ├── Common Settings
    │   ├── Visibility: [Public ▼]
    │   └── Templates: [x] ubuntu-22.04  [ ] debian-12
    │
    └── [Preview] [Generate]
```

## Implementation Order

1. **Schema change:** Add `resources JSONB` to `provider_agent_status`
2. **Agent reporting:** Implement Proxmox resource queries, extend heartbeat
3. **API storage:** Update heartbeat handler to store resources
4. **Pool capabilities:** Implement aggregation query
5. **Tier logic:** Implement tier selection
6. **API endpoints:** Implement suggest/generate endpoints
7. **CLI command:** Add `offerings suggest/generate`
8. **UI:** Add generation modal to pool details

## Open Questions

1. **Re-generation policy:** When pool capabilities change, should we:
   - Notify provider only?
   - Auto-update generated offerings?
   - Require manual re-generation?

   **Recommendation:** Notify only, never auto-modify

2. **Multi-agent heterogeneity:** If pool has agents with different hardware:
   - Use minimum (safest)?
   - Use maximum (optimistic)?
   - Generate separate offerings per hardware class?

   **Recommendation:** Use minimum for max VM size, note heterogeneity in UI

3. **Pricing suggestions:** Should we suggest prices based on market data?

   **Recommendation:** Out of scope for v1, add later if needed

## Success Metrics

- Time to first offering reduced from ~30 min (manual) to ~5 min
- Providers with auto-generated offerings have higher contract rates (hypothesis)
- Reduction in support tickets about "how to create offerings"

## Next Steps

This design feeds into:
- **Session 3:** Implement capability reporting (agent-side changes)
- **Session 4:** Implement API changes (server-side)
- **Session 5:** Implement UI/CLI
