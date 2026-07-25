# Code Robustness Audit — 2026-07-25

**Scope:** first-party Rust (`api/src`, `dc-agent/src`, `common/src`, `cli/src`) + TS
(`website/src`). Third-party (`third_party/`, `node_modules/`, `target/`, `.venv/`) excluded.

**Method:** `rg` sweep + targeted `read`. Read-only. No code changes.

**Pre-filter:** Read `docs/OPEN_ISSUES.md` fully. Already-fixed anti-patterns
from prior sessions are EXCLUDED: shared `http_client()` w/ 30s timeout, silent
hex-decode in receipts, typst 30s timeout, dispute-hex `match`, dead
`network_metrics` module, `decode_hex_path`/`decode_pubkey` helpers added,
`STRIPE_API_BASE` const introduced. This audit reports only **net-new** instances.

## Summary

| Severity | Count |
|----------|-------|
| 🔴 High | 3 |
| 🟠 Medium | 9 |
| 🟡 Low | 6 |
| **Total** | **18** |

---

## 1. Silent errors

[severity 🟠] `dc-agent/src/main.rs:2707` `ApiClient::new(&config.api).ok()`
problem: Doctor command silently swallows the API-client construction error
(missing key, bad hex, unreadable key file) and continues with `api_client=None`.
Project rule: "BE LOUD ABOUT MISCONFIGURATIONS — Use `tracing::warn!` with actionable messages… Never silently skip functionality." Doctor output then misleads the operator: it claims things are fine when the agent cannot authenticate.
fix sketch: `let api_client = match ApiClient::new(&config.api) { Ok(c) => Some(Arc::new(c)), Err(e) => { warn!("API client init failed — agent cannot reach API: {e:#}"); None } };`
confidence: 9
safe: 9

[severity 🟠] `dc-agent/src/provisioner/proxmox.rs:677` `if let Ok(status) = self.get_vm_status(vm.vmid).await { … }`
problem: While enumerating templates, the `Err` arm is dropped silently. A real Proxmox auth/network failure looks identical to "VMID is not a template" — operator gets an incomplete template list with no log. Same pattern at line 838 (idempotency check) where an error masquerades as "VM does not exist, proceed to clone" and may double-provision.
fix sketch: `match self.get_vm_status(vm.vmid).await { Ok(s) => { … }, Err(e) => warn!(vmid, error=%e, "get_vm_status failed; skipping template candidate") };` (and at 838, return the error rather than silently cloning).
confidence: 8
safe: 8

[severity 🟡] `dc-agent/src/main.rs:2893` `if let Ok(ss_output) = std::process::Command::new("ss").args(["-tlnp"]).output() { … }`
problem: Doctor's gateway-listen check silently drops the `ss` failure. If `ss` is missing or errors, doctor prints nothing instead of "[WARN] could not verify Caddy is listening (`ss` failed: …)".
fix sketch: `match Command::new("ss").args(["-tlnp"]).output() { Ok(o) => …, Err(e) => println!("  [WARN] cannot verify ports (ss failed: {e})") };`
confidence: 8
safe: 9

[severity 🟡] `api/src/main.rs:837`, `:863`, `:1207` (3 sites) `if let Ok(client) = chatwoot::ChatwootClient::from_env() { … }`
problem: Three identical silent swallows of `ChatwootClient::from_env()` errors (missing/malformed env). The doctor/setup paths then print "[OK]" or skip the bot-config step with zero explanation of *why* Chatwoot is disabled — directly contradicting the "BE LOUD ABOUT MISCONFIGURATIONS" rule.
fix sketch: Extract a `fn chatwoot_or_warn() -> Option<ChatwootClient> { match ChatwootClient::from_env() { Ok(c) => Some(c), Err(e) => { warn!("Chatwoot disabled: {e:#}"); None } } }` and call it 3× — also fixes the duplication.
confidence: 9
safe: 9

[severity 🟠] `api/src/openapi/accounts.rs:449` (also 68, 106, 526, 665, 1779, 1791, 1843 — ~9 sites in this file alone) hand-rolled `match hex::decode … => Some("Invalid X format".to_string())` blocks
problem: Many hex-decode error arms return **terse** messages ("Invalid new public key format", "Public key must be 32 bytes") that don't echo the bad value or the underlying `hex::FromHexError`. This contradicts both the AGENTS.md rule ("provide failure details (e.g. with `"{:#?}"`)") and the prior session's stated goal ("unified terse→detailed error msgs"). The detailed helpers `decode_pubkey` / `decode_hex_path` already exist in `openapi/common.rs` but ~30 sites across accounts.rs/users.rs/admin.rs/providers.rs/offerings.rs/invoices.rs/webhooks.rs were never migrated to them — see Duplication §2 below.
fix sketch: `let pk = match decode_pubkey(&req.new_public_key) { Ok(b) => b, Err(e) => return Json(ApiResponse { success:false, error:Some(e), data:None }) };` — eliminates the duplicated 8-line block and produces a detailed message.
confidence: 9
safe: 8

---

## 2. I/O without timeouts

[severity 🔴] `cli/src/commands/provider.rs:186` and `:257` `let client = reqwest::Client::new();`
problem: User-facing CLI's `pool_suggest_offerings` / `pool_generate_offerings` use a bare reqwest client (no timeout) for an authenticated `.send().await?` round-trip. The prior session replaced 12 sites in `api/src/bin/api-cli.rs` + `api_cli/client.rs` with `http_client()`, but the **separate** `cli/` crate (user-facing CLI, not admin CLI) was missed. A hung API server hangs the CLI forever.
fix sketch: Add a `http_client()` helper to the `cli` crate (or re-export one from `dcc-common`) and use it at both sites — minimal change, same shape as the prior api-cli fix.
confidence: 10
safe: 10

[severity 🔴] `dc-agent/src/provisioner/manual.rs:27` `client: Client::new()` (in `ManualProvisioner::new`)
problem: ManualProvisioner builds a bare `reqwest::Client::new()` with no timeout. Its `send_webhook` posts to a user-supplied `notification_webhook` URL — a slow/dead recipient hangs the provisioner (which runs inside the agent's polling loop) indefinitely. All sibling provisioners (proxmox, digitalocean) and `dc-agent/src/api_client.rs` use `.timeout(Duration::from_secs(30))`.
fix sketch: `client: reqwest::Client::builder().timeout(Duration::from_secs(30)).build().expect("reqwest default config always builds")` — match the existing dc-agent convention.
confidence: 10
safe: 10

[severity 🔴] `dc-agent/src/setup/proxmox.rs:431` `reqwest::Client::builder().danger_accept_invalid_certs(true).build()?`
problem: `verify_api_token` posts to `https://127.0.0.1:8006/...` with no `.timeout()`. If the local Proxmox API daemon is wedged, `dc-agent setup` hangs forever during initial configuration. Every other dc-agent client builder applies a 30s timeout; this one is the outlier.
fix sketch: Add `.timeout(std::time::Duration::from_secs(30))` to the builder chain.
confidence: 10
safe: 10

[severity 🟠] `dc-agent/src/setup/mod.rs:21` `execute_command` — `Command::new("sh").arg("-c").arg(cmd).output()`
problem: This is the **shared** helper used by `GatewaySetup::execute` and `ProxmoxSetup::execute` for every local-shell step of dc-agent setup (caddy install, sysctl, qm destroy, firewall rules, etc.). It has no timeout. One hung command (e.g. a `curl` without `--max-time`, an interactive prompt) blocks the entire setup indefinitely. High-leverage: fixing this one fn fixes every setup-site caller at once. Note `gateway::detect_public_ip` already uses `curl --max-time 5` defensively, suggesting the team knows the issue exists for ad-hoc commands.
fix sketch: Switch to `tokio::process::Command` and wrap in `tokio::time::timeout(Duration::from_secs(120), child.wait_with_output())` with a configurable per-call override; bail on elapsed.
confidence: 9
safe: 7

[severity 🟡] `dc-agent/src/upgrade.rs:119, 144, 153, 166` — four `Command::new(...).output()` / `.status()` calls with no timeout
problem: `verify_binary_version` (runs the downloaded binary with `--version`), `is_systemd_service`, and `restart_service` (two systemctl calls). If the new binary launches an interactive prompt or `systemctl restart` wedges, the self-upgrade flow hangs forever with no watchdog. Lower severity because these are short-lived setup-time calls and systemd has internal timeouts — but `verify_binary_version` running an untrusted binary with no timeout is the genuine risk.
fix sketch: Wrap each in `tokio::time::timeout` (or `std::process::Command::output` + a `wait_timeout` crate); 10s for `--version`, 30s for systemctl.
confidence: 8
safe: 7

[severity 🟡] `api/src/bin/api-cli.rs:1614` `tokio::process::Command::new("ssh").args([...]).output().await?`
problem: Gateway SSH-connectivity test relies only on the ssh client's `-o ConnectTimeout=10`. If the SSH negotiation succeeds but the remote `echo SSH_CONNECTION_OK` hangs (PTY trap, slow shell init), the await has no overall cap. CLI helper, lower severity.
fix sketch: Wrap the whole call in `tokio::time::timeout(Duration::from_secs(20), …).await`.
confidence: 7
safe: 9

---

## 3. Duplication

[severity 🟠] `dc-agent/src/api_client.rs:785–803` (in `register_gateway`) re-implements the request-signing primitive that already exists as `ApiClient::build_auth_headers` (line 288)
problem: The inline block builds the exact same `timestamp_str + nonce_str + method + path + body` sign-message and the same `X-Timestamp` / `X-Nonce` / `X-Signature` headers — the file even carries a comment `// Build auth headers (same signing as ApiClient::build_auth_headers)`. This is exactly the DRY violation AGENTS.md forbids; drift between the two sites would silently break auth on either path.
fix sketch: Promote `build_auth_headers` to a free function `build_signed_headers(identity, method, path, body_bytes) -> AuthHeaders` and call it from both `ApiClient::signed_request` and `register_gateway`. (The standalone `setup_agent` is correctly excluded — it's unauthenticated.)
confidence: 9
safe: 8

[severity 🟠] ~30 sites across `api/src/openapi/{accounts,users,admin,providers,offerings,invoices,webhooks}.rs` still hand-roll `match hex::decode(…) { Ok => …, Err => return ApiResponse{ error: Some("…") }) }` instead of the `decode_pubkey` / `decode_hex_path` helpers added in 2024-07-24 in `openapi/common.rs`
problem: The prior session added the helpers and migrated `agents.rs`/`stats.rs`/`providers.rs`(partial), but `accounts.rs` alone still has 9 raw sites, several returning terse messages that violate the "detailed errors" rule (see Silent errors §5). This is both DRY and a consistency bug — the same `pubkey` field produces "Invalid public key hex: <err> (value: <v>)" from one handler and "Invalid public key format" from the next.
fix sketch: Sweep `rg "match hex::decode"` in `api/src/openapi`; replace each `pubkey`-decode block with `decode_pubkey(...)` and each `contract_id`/`id`-decode with `decode_hex_path(..., "label")`. Mechanical, file-local changes; preserves the public API.
confidence: 9
safe: 8

---

## 4. Dead / unwired code

[severity 🟡] `dc-agent/src/post_provision.rs` (5-line file) is a pure re-export shim: `pub use dcc_common::ssh_exec::*;`
problem: File doc says "Re-exports from `dcc_common::ssh_exec` for backward compatibility within dc-agent." AGENTS.md: "No backward-compatibility excuses. This is a monorepo — change all that's needed to end up with clean code." Three call sites in `dc-agent/src/main.rs` (lines 8 import, 1999 `execute_post_provision_script`, 2334 `reset_password_via_ssh`) keep the shim alive.
fix sketch: Update the 3 call sites to `use dcc_common::ssh_exec::{execute_post_provision_script, reset_password_via_ssh};` and delete `dc-agent/src/post_provision.rs` plus its `pub mod post_provision;` line in `lib.rs`.
confidence: 8
safe: 8

[severity 🟡] `api/src/stripe_client.rs:15` `const STRIPE_API_BASE: &str = "https://api.stripe.com";` is **private** to its file
problem: The prior session added this const to DRY the Stripe base URL, but it is not `pub`, so callers outside `stripe_client.rs` cannot reference it. The result is that 5 sites still hardcode the literal (see Magic §1). Either the const should be `pub(crate)` and used by the rest of the crate, or it's theatre that masks the ongoing duplication.
fix sketch: `pub(crate) const STRIPE_API_BASE: &str = …;` and migrate `main.rs` and `api-cli.rs` callers to `crate::stripe_client::STRIPE_API_BASE`.
confidence: 9
safe: 9

---

## 5. Magic numbers / strings

[severity 🟠] `api/src/main.rs:487, 524, 557` literal `"https://api.stripe.com/v1/webhook_endpoints"` (×3 in one function)
problem: The setup-webhook helper hardcodes the Stripe base 3× in the same `fn`. `STRIPE_API_BASE` already exists at `api/src/stripe_client.rs:15` (private — see Dead §2) but is not used here.
fix sketch: `pub(crate) const STRIPE_API_BASE` + `format!("{STRIPE_API_BASE}/v1/webhook_endpoints")` and `format!("{STRIPE_API_BASE}/v1/webhook_endpoints/{webhook_id}")`.
confidence: 10
safe: 10

[severity 🟡] `api/src/bin/api-cli.rs:1823, 1883` literal `"https://api.stripe.com/v1/balance"` (×2)
problem: Two Stripe health-check sites in `api-cli` hardcode the same base URL. Same root cause as above.
fix sketch: Use the same `pub(crate)` const (note: `api-cli` is in the same crate as `stripe_client`).
confidence: 10
safe: 10

[severity 🟡] HTTP timeout `30` defined locally as `REQUEST_TIMEOUT_SECS: u64 = 30` in `api/src/cloud/hetzner.rs:21`, `api/src/cloud/vultr.rs:21`, `api/src/cloud/proxmox_api.rs:18`, plus `.timeout(Duration::from_secs(30))` repeated in `dc-agent/src/api_client.rs:247, 709, 777`, `dc-agent/src/provisioner/{proxmox,digitalocean}.rs`
problem: `api/src/http_util.rs` already has the canonical `HTTP_TIMEOUT_SECS: u64 = 30` constant but it is private to that file. Cloud providers and dc-agent each re-spell the same number; a future global change (e.g. to 60s) requires editing 8+ files.
fix sketch: `pub(crate) const HTTP_TIMEOUT_SECS: u64 = 30;` in `http_util.rs` and reference it from each builder. Or expose a `default_timeout()` fn.
confidence: 8
safe: 9

---

## 6. Threads / tasks

No net-new findings. Every `tokio::spawn` in `api/src/main.rs` (metadata_cache, cleanup, timeout_cleanup, email, auto_renewal, sla_alert, publish_scheduled, cloud_provisioning) uses `shutdown_tx.subscribe()` / `shutdown_rx` watch channels for graceful termination. `api/src/cloud_provisioning_service.rs:79, 110` use `tokio::select!` with a shutdown receiver. `api/src/receipts.rs:579` and `dc-agent/src/main.rs:1675` (remote-upgrade spawn) are test-only / fire-and-forget-with-error-logging — acceptable.

---

## Highest-leverage net-new fixes (recommended order)

1. **`dc-agent/src/setup/mod.rs:21` — add timeout to `execute_command`.** One change, fixes every setup-time hang site at once.
2. **`cli/src/commands/provider.rs:186, 257` — replace bare `reqwest::Client::new()` with a timed client.** Brings the user-facing CLI in line with the prior session's api-cli fix.
3. **`dc-agent/src/provisioner/manual.rs:27` — add 30s timeout to ManualProvisioner's client.** Matches every sibling provisioner.
4. **Migrate ~30 `match hex::decode` sites in `api/src/openapi/*` to the existing `decode_pubkey`/`decode_hex_path` helpers.** Mechanical sweep that simultaneously fixes the terse-error-message violation.
5. **Make `STRIPE_API_BASE` `pub(crate)` and use it in `main.rs:487/524/557` + `api-cli.rs:1823/1883`.** Finishes the Stripe-URL DRY the prior session started.
