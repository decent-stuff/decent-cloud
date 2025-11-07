# Uniqueness Constraints in Decent Cloud API

## Overview
A key is guaranteed to be unique within a single label + block combination.
The tuple (block offset, label, key) is always unique for ledger entries.

## Database Schema
- The database stores structured data from ledger entries
- Each entry type (label) has its own table
- Uniqueness is handled at the application level

## Implementation Details
- Use `INSERT OR IGNORE` to handle duplicate entries gracefully
- This allows sync to continue even when processing duplicate data
- Sync position always advances, preventing infinite loops

## Fixed Tables
- `contract_sign_requests`: Now uses `INSERT OR IGNORE` to handle duplicates
- `sync_state`: Reset when position gets stuck (at known problematic position)
- Removed incorrect `UNIQUE(contract_id)` constraint that was causing issues

## Sync Service Behavior
- Fetches data every 30 seconds
- Groups entries by label for batch processing
- Updates position only after successful processing
- Gracefully handles duplicate entries
