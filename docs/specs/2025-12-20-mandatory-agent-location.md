# Spec: Mandatory Agent Location

**Status**: Draft
**Author**: Claude Code
**Date**: 2025-12-20
**Issue**: Agents can be registered without geographic location metadata, making them non-functional for contract routing

## Problem Statement

Currently, agents can be registered via the legacy registration path without any geographic location metadata:

```bash
dc-agent setup proxmox --host 192.168.122.92 --identity np14
```

This creates an agent that:
- Has no `pool_id` (NULL in `provider_agent_delegations.pool_id`)
- Has no location/region metadata
- **Cannot be matched to contracts** that have location requirements
- Silently degrades (registers successfully but never receives work)

**This violates the fail-fast principle**: agents should not be registered if they lack critical metadata.

## Why This Matters

Every agent MUST have a geographic location because:

1. **Contract Routing**: Offerings have `datacenter_country` and optional `agent_pool_id` fields. Contract matching requires location data.
2. **Data Residency**: Legal compliance (GDPR, etc.) demands knowing where VMs are provisioned.
3. **SLA Guarantees**: Latency requirements need geographic proximity matching.
4. **User Transparency**: Customers need to know where their infrastructure runs.

An agent without location is **non-functional** - it will never receive provisioning requests.

## Root Cause

The pool system was added later to enable multi-region support, but the legacy registration path (`register_agent_with_api()`) was not updated or deprecated. This creates two registration paths:

1. **Legacy path** (direct delegation): No pool, no location ❌
2. **Modern path** (token-based): Pool assignment, location metadata ✅

## Proposed Solution

**Force pool-based registration**

Remove legacy registration entirely, require all agents to be in pools:

```bash
# Provider creates pool first via UI or API:
# POST /providers/{pubkey}/agent-pools
# { "name": "eu-west-1", "location": "europe", "provisioner_type": "proxmox" }

# Agent setup requires pool token:
dc-agent setup token --token apt_europe_abc123...
```

**Changes required**:
1. Remove `register_agent_with_api()` function
2. Update `dc-agent setup proxmox` to guide users to create pool first
3. Update docs and error messages
4. Migrate existing agents to pools or mark as deprecated

**Pros**:
- Cleaner architecture (single registration path)
- Pool benefits (load balancing, capacity management)
- No schema changes needed

**Cons**:
- More complex UX (two-step process)
- Requires provider to manage pools
- Breaking change for existing workflows

---

**Extension: Auto-detect location**

Detect agent's location automatically via IP geolocation during setup.

## Recommendation

Implement Force pool-based registration with and warn if auto-detection identifies another location.
By default the "setup" would not work if auto-detection detects a completely other region. User can still proceed with the setup if they specify `--force`.

Cleanup and remove the legacy and now unused commands, cli, and code.
