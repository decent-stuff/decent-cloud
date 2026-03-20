# DATABASE KNOWLEDGE BASE

## OVERVIEW
`repo/api/src/database/` is the SQLx-backed persistence layer for accounts, contracts, offerings, providers, cloud resources, and reporting.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Module registry / exports | `mod.rs` | Public DB surface and test modules |
| Contracts lifecycle data | `contracts.rs` | One of the largest modules |
| Marketplace inventory | `offerings.rs`, `providers.rs` | Offerings + provider metadata |
| Account/billing data | `accounts.rs`, `subscriptions.rs` | User-facing account state |
| Infra/resource state | `cloud_resources.rs`, `agent_pools.rs` | Provisioning-facing records |
| Shared DB tests | `test_helpers.rs` | Ephemeral PostgreSQL helpers |

## CONVENTIONS
- One domain module per major table/aggregate; filenames mirror the data domain.
- Re-exports in `mod.rs` define the stable surface consumed by the rest of the API crate.
- Tests live inline or in adjacent `tests.rs`/`*_tests.rs`; use existing helpers before adding new setup code.
- Local default DB URL is `postgres://test:test@localhost:5432/test`; containerized agent docs use `postgres` hostname at higher levels.

## ANTI-PATTERNS
- Copy-pasting query or mapping logic across domain modules.
- Bypassing `test_helpers.rs` for integration-style DB tests.
- Mixing request/HTTP concerns into database modules.
- Hiding missing data or DB failures behind silent defaults.

## COMMANDS
```bash
cargo nextest run -p api database
cargo test -p api database::
```

## NOTES
- This directory is large because business domains are stored close to their query logic.
- For endpoint behavior, jump back up to `repo/api/src/openapi/`.
