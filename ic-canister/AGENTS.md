# IC-CANISTER KNOWLEDGE BASE

## OVERVIEW
`repo/ic-canister/` is the Internet Computer canister crate: wasm-targeted endpoints plus backend logic shared with the rest of the Rust workspace.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Crate entry | `src/lib.rs` | wasm-only endpoint gating lives here |
| Backend logic | `src/canister_backend/` | Core token and business logic |
| IC endpoint surface | `src/canister_endpoints/` | wasm-targeted public interface |
| Integration tests | `tests/` | Pocket-IC / canister behavior |
| Build config | `Cargo.toml`, `dfx.json` | Crate + IC setup |

## CONVENTIONS
- Keep host-only logic behind `cfg(not(target_arch = "wasm32"))` in shared crates; this crate itself is wasm-first.
- Reuse shared token and identity primitives from `repo/common/` rather than redefining them locally.
- Distinguish backend logic (`canister_backend`) from public canister entrypoints (`canister_endpoints`).

## ANTI-PATTERNS
- Mixing non-wasm-only assumptions directly into canister entrypoints.
- Duplicating token constants already re-exported from shared crates.
- Treating `dfx` state or generated artifacts as source of truth.

## COMMANDS
```bash
cargo test -p ic-canister
cargo build -p ic-canister --target wasm32-unknown-unknown
```

## NOTES
- This crate is smaller than `api/` and `dc-agent/`, so keep local docs narrow and IC-specific.
