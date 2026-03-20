# DC-AGENT KNOWLEDGE BASE

## OVERVIEW
`repo/dc-agent/` is the provider-side runtime that polls the API, provisions workloads, manages gateway routing, and performs host setup/diagnostics.

## STRUCTURE
```text
dc-agent/
|- src/main.rs        # clap CLI and main polling loop
|- src/provisioner/   # Proxmox/manual/script backends
|- src/gateway/       # public routing and port allocation
|- src/setup/         # bootstrap and token-driven host setup
`- src/config.rs      # persisted agent config model
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| CLI commands | `src/main.rs` | `run`, `doctor`, `setup`, `test-provision`, `upgrade` |
| Provider API sync | `src/api_client.rs` | Central API communication |
| Provisioning backend | `src/provisioner/` | `proxmox.rs` is the heavy path |
| Gateway behavior | `src/gateway/` | DNS, forwarding, external reachability |
| Registration/setup | `src/setup/`, `src/registration.rs` | Initial host bootstrap |
| Config schema | `src/config.rs` | What the runtime persists and reads |

## CONVENTIONS
- `dc-agent` is the real runtime crate; `repo/agent/` is only the sandbox/container helper area.
- `doctor` and `test-provision` are the fastest ways to verify setup-sensitive changes.
- Provisioners implement the shared trait in `src/provisioner/mod.rs`; extend the trait instead of branching ad hoc.
- API-facing structs must stay serializable and explicit; missing optional fields are represented as `Option`, not magical defaults.

## ANTI-PATTERNS
- Putting backend-specific provisioning logic in `main.rs`.
- Treating gateway state as separate from provisioning state when the command already reconciles both.
- Silent recovery from host misconfiguration; diagnostics should surface the exact failure.

## COMMANDS
```bash
cargo build -p dc-agent
cargo nextest run -p dc-agent
dc-agent doctor
dc-agent test-provision --help
```

## NOTES
- Changes here usually interact with API contracts in `repo/api/` and shared types in `repo/common/`.
