# API KNOWLEDGE BASE

## OVERVIEW
`repo/api/` is the central Rust backend crate: HTTP server, admin/test CLI binaries, DB access, service integrations, and background jobs.

## STRUCTURE
```text
api/
|- src/main.rs          # api-server binary and startup validation
|- src/bin/api-cli.rs   # admin/test CLI
|- src/openapi/         # HTTP endpoint surface
|- src/database/        # SQLx-backed persistence layer
|- migrations_pg/       # schema migrations
`- email-utils/         # api-only helper crate
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Server boot, env validation | `src/main.rs` | `Serve`, `Doctor`, `Sync`, `Setup` commands |
| Endpoint behavior | `src/openapi/` | Handlers grouped by domain |
| DB queries and models | `src/database/` | Large module set; has its own child AGENTS |
| Admin/test automation | `src/bin/api-cli.rs` | Contract, DNS, health, E2E helpers |
| Chatwoot support bot | `src/support_bot/` | Has existing local `AGENTS.md` |
| Help center/doc sync | `src/sync_docs.rs`, `src/helpcenter/` | Chatwoot docs flow |

## CONVENTIONS
- `anyhow` + context-rich errors; fail fast rather than fallback.
- `tracing` is the logging surface; startup logs explicitly call out missing optional config.
- `src/main.rs` is the deploy-time validation choke point; new env-dependent features belong there and in `Doctor`.
- `api-cli` lives in this crate; do not look for a separate crate for admin commands.
- Shared domain primitives come from `repo/common/`; third-party source references live in `repo/third_party/`.

## ANTI-PATTERNS
- Lazy config validation at request time.
- Duplicating endpoint logic already present in `src/openapi/` or shared DB helpers.
- Treating `agent/` (outer workspace) as the Rust agent; the actual runtime crate is `repo/dc-agent/`.
- Editing generated/offline artifacts (`.sqlx`, bindings, caches) without matching source changes.

## COMMANDS
```bash
cargo build -p api --bin api-server
cargo build -p api --bin api-cli
cargo make test-api
cargo make clippy-api
```

## NOTES
- Existing repo-level instructions in `repo/AGENTS.md` still apply here.
- `src/database/` is dense enough to warrant its own local guidance.
