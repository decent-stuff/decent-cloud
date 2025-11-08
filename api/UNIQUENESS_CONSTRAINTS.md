# Uniqueness Constraints in Decent Cloud API

## Overview
A key is guaranteed to be unique within a single label + block combination.
The tuple (block offset, label, key) is always unique for ledger entries.

## Database Schema
- The database stores structured data from ledger entries
- Each entry type (label) has its own table
- Uniqueness is handled at the database level with proper INSERT strategies

## Implementation Details

### Tables with UNIQUE constraints (use INSERT OR REPLACE):
- `provider_registrations.pubkey_hash`
- `provider_profiles.pubkey_hash`
- `user_registrations.pubkey_hash`

### Tables that should store ALL entries (use regular INSERT):
- `provider_check_ins` - store all check-ins for historical tracking and uptime analysis
- `contract_sign_requests` - store all requests for complete audit trail

### Transactional tables (use regular INSERT - each entry is distinct):
- `token_transfers`, `token_approvals`, `contract_payment_entries`
- `contract_sign_replies`, `reputation_changes`, `reputation_aging`
- `reward_distributions`, `linked_ic_ids`
- `provider_offerings` and related tables (id-based uniqueness)

## Sync Service Behavior
- Fetches data every 30 seconds
- Groups entries by label for batch processing
- Updates position only after successful processing
- Handles duplicate entries according to table constraints
- Supports both Prov* and NP* ledger labels for provider operations
