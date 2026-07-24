# Code-Robustness Audit — Decent Cloud backend + frontend

**Date:** 2026-07-24
**Scope:** `repo/api/`, `repo/common/`, `repo/cli/`, `repo/dc-agent/` (Rust) + `repo/website/src/` (SvelteKit/TS)
**Mode:** READ-ONLY audit. No source modified, no commits. All searches wrapped with `timeout 60`.
**Baseline:** `docs/OPEN_ISSUES.md` (2026-07-23 snapshot) — only NET-NEW findings below.

## Method / coverage

Ripgrep sweeps for: `let _ =`, `if let Ok`, `if let Some`, `.ok()`, `Err(_) =>`, `.unwrap()`,
`.expect()`, indexing, integer division, `reqwest::Client::new()`, `reqwest::Client::builder()`,
`Command::new()`, `tokio::spawn`, `#[allow(dead_code)]`, file `wc -l`. Each candidate was opened
and read in context to filter false positives (test code, infallible parsing, properly-bounded
indexing, intentionally-best-effort cleanup). Test code (`tests.rs`, `*_test.rs`, `#[test]`,
`#[cfg(test)]`) was excluded throughout.

The prior 2026-07-23 session already shipped money-safety fixes (R1–R10 — refund bounds, payment
status allow-lists, Stripe secret required in prod, SSE auth) and sharded the migration array; this
audit does **not** re-report those.

## TL;DR — counts per category

| Category | Net-new findings |
|---|---:|
| 1. Silent error swallowing (prod) | **3** |
| 2. Missing timeouts on I/O (HTTP / subprocess) | **11** |
| 3. Panics in prod paths | **0** (false positives only) |
| 4. Files > 2000 lines | **11** (8 prod, 3 test-only) |
| 5. Duplication / DRY | **2** |
| 6. Dead / unwired code | **2** |

Highest-impact cluster: HTTP clients constructed via `reqwest::Client::new()` with no
`.timeout()` across the money and identity paths. The cloud-provider backends (Hetzner/Vultr/Proxmox)
correctly use `.timeout(REQUEST_TIMEOUT_SECS)` on the builder — the rest of the codebase does not
follow that pattern.

---

## 1. Silent error swallowing

### 1a. `api/src/receipts.rs:297` — `if let Ok(requester_pubkey) = hex::decode(&contract.requester_pubkey)`
- **Pattern:** `if let Ok(...) = ...` with no `else`/Err arm.
- **Why it's a problem:** Inside `send_contract_accepted_notification`. Wraps the in-app
  notification INSERT for the requester. When `requester_pubkey` fails to hex-decode, the function
  silently skips the in-app "Rental Request Accepted" notification and continues to the email step.
  The Err (corrupt stored hex on a contract row) is never logged, so ops has no signal that
  notifications are being dropped. Money-adjacent: this notification is the tenant's confirmation
  that their paid request was accepted.
- **Fix sketch:** Replace with `let requester_pubkey = hex::decode(&contract.requester_pubkey).with_context(...)?;`
  or at minimum a `match` whose Err arm does `tracing::warn!(...);` before continuing. The sibling
  module `rental_notifications.rs:83` already uses the correct pattern (`hex::decode(...).context("Invalid provider pubkey hex")?`).
- **Confidence:** 8 — **Safety:** 9 (notification only; no money moved here).

### 1b. `api/src/receipts.rs:418` — same pattern in `send_contract_rejected_notification`
- **Pattern:** identical to 1a, but on the **rejected-with-refund** path.
- **Why it's a problem:** When a contract is rejected and a refund is initiated, the tenant's
  in-app "Rental Request Rejected … A refund has been initiated" notification is silently dropped
  if `requester_pubkey` fails to hex-decode. The tenant may have been charged and gets no in-app
  signal of the refund. Email still goes out if `requester_contact` is an email, but the in-app
  channel is silently dead with no log.
- **Fix sketch:** Same as 1a.
- **Confidence:** 8 — **Safety:** 8 (refund notification; money-relevant signal).

### 1c. `api/src/openapi/webhooks.rs:779` — `if let Ok(bytes) = hex::decode(hex_id)` in `lookup_contract_for_charge`
- **Pattern:** `if let Ok(...) = ...` inside the Stripe-dispute → contract lookup chain.
- **Why it's a problem:** If a Stripe dispute's metadata `contract_id` is malformed hex, the
  function silently falls through to the next lookup strategy (payment_intent, then charge) with
  no log entry. The function's own doc comment claims "the caller logs and pages ops" — but the
  caller only learns the *final* outcome (`None`), not that the metadata path failed mid-chain.
  For a dispute (money + chargeback), losing the trace of which lookup strategy was tried makes
  incident response harder.
- **Fix sketch:** Add a `tracing::warn!(hex_id = %hex_id, "dispute metadata contract_id not valid hex; falling through to payment_intent lookup")` on the implicit Err branch.
- **Confidence:** 6 — **Safety:** 9 (diagnostic noise only; the fallback chain itself is correct).

---

## 2. Missing timeouts on I/O

Pattern across the codebase: `reqwest::Client::new()` (no builder, no `.timeout()`). The default
`reqwest::Client` has **no** connect/read/write timeout — a slow or stuck peer hangs the future
until the task is cancelled. The cloud-provider clients in `api/src/cloud/{hetzner,vultr,proxmox}.rs`
already follow the right pattern (`.timeout(REQUEST_TIMEOUT_SECS)` on the builder); the entries
below do not.

### Money / identity path (highest priority)

- **`api/src/stripe_client.rs:573`** — `let client = reqwest::Client::new();` inside
  `create_usage_record` (subscription usage metering). The rest of `StripeClient` uses the official
  `stripe::Client`; only this method bypasses it with a raw `reqwest::Client::new()` and a
  hardcoded `https://api.stripe.com/v1/subscription_items/{}/usage_records` URL. A hung Stripe API
  hangs usage metering indefinitely. — **Confidence:** 9 — **Safety:** 8.

- **`api/src/icpay_client.rs:60`** — `let client = reqwest::Client::new();` inside `IcpayClient::new`,
  used for **all** ICPay API calls including `create_refund` (refund path). Same client is shared
  across `get_payments_by_metadata`, `verify_payment_by_metadata`, payout creation. No timeout on
  any of them. — **Confidence:** 9 — **Safety:** 8.

- **`api/src/oauth_simple.rs:188`** — `reqwest::Client::new()` for Google OAuth user-info fetch on
  the login/callback path. If Google's endpoint is slow, the user's auth flow hangs forever (no
  timeout, no request-level cancellation). — **Confidence:** 8 — **Safety:** 7.

### Provisioning / DNS path

- **`api/src/cloudflare_dns.rs:65`** — `client: Client::new()` in `CloudflareDns::from_env`. Used
  for DNS record create/delete during cloud VM provisioning. A hung Cloudflare API wedges the
  provisioning loop's per-resource step. — **Confidence:** 8 — **Safety:** 8.

### Support / docs / less-critical paths

- **`api/src/invoices.rs:511`** — `reqwest::get(&pdf_url)` (convenience constructor; builds a new
  client per call AND has no timeout) for Stripe invoice PDF download.
- **`api/src/llm_client.rs:173`** — `let client = Client::new();` for Anthropic/OpenAI calls.
- **`api/src/chatwoot/client.rs:77, 95, 490, 512`** — `Client::new()` in four constructors
  (`ChatwootClient`, `ChatwootPlatformClient` for both prod + test variants).
- **`api/src/support_bot/embeddings.rs:93`** — `Client::new()` per call (no shared client, no
  timeout) for OpenAI embeddings; also a perf issue (new connection pool every request).
- **`api/src/price_cache.rs:91, 124`** — `Client::new()` in constructor and in a refresh path.

### dc-agent self-upgrade path

- **`dc-agent/src/upgrade.rs:45`** and **`:72`** — `reqwest::Client::builder().user_agent("dc-agent").build()?`
  in `fetch_latest_version` and `download_file`. The builder sets `user_agent` but **omits**
  `.timeout()`. The whole self-upgrade flow (latest-release lookup, binary download) can hang
  indefinitely on a slow GitHub API. Note: the SSH/subprocess layer in `common/src/ssh_exec.rs`
  correctly wraps every `Command::new("ssh")` in `tokio::time::timeout(SCRIPT_TIMEOUT, …)` — the
  upgrade HTTP layer should follow the same convention. — **Confidence:** 9 — **Safety:** 8.

### CLI (lower priority — interactive tool, but still hangs the user)

- **`api/src/bin/api-cli.rs`** — 12 occurrences of `reqwest::Client::new()` (lines 1052, 1233, 1303,
  1425, 1457, 1746, 1808/1826 inline `env::var(...).unwrap()` builds, etc.). The CLI uses no
  builder at all, so any stalled API server hangs the user's terminal indefinitely. Less critical
  than server-side (no affected tenants), but the inconsistency vs the cloud-provider clients is
  worth a single shared `CliHttp::new()` helper. — **Confidence:** 7 — **Safety:** 9.

### Missing subprocess timeout

- **`api/src/invoices.rs:398`** — `tokio::process::Command::new("typst")` followed by `cmd.output().await`
  with **no `tokio::time::timeout` wrapper**. Typst can hang on first-run package downloads or on
  malformed templates; a stuck compile wedges the invoice-generation request handler. Compare with
  the correct pattern at `dc-agent/src/provisioner/script.rs:61` and `common/src/ssh_exec.rs:321`.
  — **Confidence:** 8 — **Safety:** 9.

> **Note on db queries:** all SQLx calls go through the pool (which has its own statement/acquire
> timeouts configured elsewhere), so per-query timeouts are intentionally omitted — not reported.

---

## 3. Panics in prod paths

**Zero net-new findings.** All `.unwrap()` / `.expect()` candidates in non-test code were verified
safe-by-construction or startup-only:

- `api/src/main.rs:1611` `.expect("failed to install SIGTERM handler")` — startup-only, fails fast
  before serving.
- `api/src/validation.rs:27, 32` `.unwrap()` on `Regex::new` of constant patterns inside
  `OnceCell::get_or_init` — panics at first use if the literal is malformed, which is a programmer
  bug, not a runtime one. Acceptable.
- `dc-agent/src/api_client.rs:467` `.expect("system clock before epoch")` on
  `SystemTime::now().duration_since(UNIX_EPOCH)` — physically impossible in any deployed context.
- `dc-agent/src/main.rs:1329` `.expect("default provisioner must exist")` on `HashMap::get` for the
  default provisioner type that was just used to populate the map — would only fire on a programming
  bug at startup. Borderline; could be `?` for cleanliness but not a defect.
- The remaining ~1000 `.unwrap()` matches in `api/src` are inside `#[cfg(test)]` inline test mods
  and were excluded.

No `[i]` indexing panics found in prod code — every `parts[N]` access has a preceding
`parts.len() > N` guard (verified at `dc-agent/src/api_client.rs:122-127`,
`dc-agent/src/provisioner/proxmox.rs:368-372`, `dc-agent/src/upgrade.rs:19-22`,
`dc-agent/src/main.rs:842-846`).

---

## 4. Files > 2000 lines

### Production source

| File | Lines | Split suggestion |
|---|---:|---|
| `api/src/openapi/providers.rs` | **6643** | 88 handlers across ~15 domains. Split into `providers/profile.rs`, `contacts.rs`, `stats.rs`, `contracts_health.rs`, `offerings_crud.rs`, `offerings_import_export.rs`, `allowlist.rs`, `rental_requests.rs`, `notifications.rs`, `sla_sli.rs`, `onboarding.rs`, `agent_pools.rs`, `setup_tokens.rs`, `bandwidth.rs`, `offering_suggestions.rs`, `auto_accept_rules.rs`. Currently the single biggest source file in the repo. |
| `api/src/bin/api-cli.rs` | **3654** | Split per command group into `cli/contract.rs`, `cli/identity.rs`, `cli/dns.rs`, `cli/gateway.rs`, `cli/e2e.rs`, `cli/health.rs` (mirrors the layout `cli/src/commands/` already uses in the user-facing `cli/` crate). |
| `dc-agent/src/main.rs` | **3562** | Mixes CLI dispatch + setup flows + polling loop + doctor + reconcile. Pull `run_setup*`, `install_systemd_service`, `run_proxmox_setup_if_requested`, `run_gateway_setup_if_requested` into `dc-agent/src/setup/orchestrator.rs`; pull `run_doctor` into `dc-agent/src/doctor.rs`; pull `poll_and_provision` + `reconcile_instances` + `collect_running_by_contract` into `dc-agent/src/runtime.rs`. |
| `api/src/openapi/accounts.rs` | **2953** | Account CRUD, profile, email, keys, security, sessions — natural split per sub-resource. |
| `api/src/database/offerings.rs` | **2845** | Offering queries + the recommendation engine (`build_preference_profile`, `score_candidate`) + import/export helpers — pull the recommendation engine into `offerings/recommend.rs`. |
| `api/src/openapi/webhooks.rs` | **2683** | Stripe + Chatwoot + Telegram + ICPay webhook handlers in one file. Split per source into `webhooks/stripe.rs`, `webhooks/chatwoot.rs`, `webhooks/telegram.rs`, `webhooks/icpay.rs` (the dispute/charge helpers in `webhooks/stripe.rs` alone are ~700 lines). |
| `api/src/database/cloud_resources.rs` | **2444** | Cloud-resource lifecycle + marketplace listing + reconciliation. |
| `api/src/openapi/contracts.rs` | **2339** | Contract create/cancel/extend/refund/list — split refund+extend (money ops) from read-only list/get. |

### Frontend

| File | Lines | Split suggestion |
|---|---:|---|
| `website/src/lib/services/api.ts` | **4276** | 139 functions behind clear `// ============ … ============` section headers already in the file (Contract Extension, Rental Request, Reseller, Billing Settings, VAT, Invoice, Agent Delegation, Agent Pools, Pool Upgrade, Setup Tokens, Subscription, Provider Stats, Bandwidth, Offering Generation, Cloud Self-Provisioning, Spending Alerts). Split into `api/contracts.ts`, `api/rentals.ts`, `api/billing.ts`, `api/agents.ts`, `api/cloud.ts`, `api/spending.ts` with a barrel `api/index.ts`. Biggest frontend file in the repo. |
| `website/src/routes/dashboard/rentals/[contract_id]/+page.svelte` | **2058** | Single-component rental-detail page; extract sub-components for status banner, gateway info, action buttons, event timeline. |

### Test-only (lower priority, but still large)

| File | Lines |
|---|---:|
| `api/src/database/contracts/tests.rs` | 5696 |
| `api/src/database/offerings/tests.rs` | 5245 |
| `api/src/database/stats/tests.rs` | 3141 |
| `api/src/database/accounts/tests.rs` | 1891 (just under threshold) |

These test files are large but their growth is bounded by the modules they cover; splitting them
adds little value vs splitting the production files above.

---

## 5. Duplication / DRY violations

### 5a. `hex::decode(&pubkey.0)` boilerplate — repeated 16+ times with **two different error messages**

- **Locations:**
  - `api/src/openapi/providers.rs` — 13 occurrences of the `match hex::decode(&pubkey.0) { Ok(pk) => pk, Err(_) => return Json(ApiResponse { … error: Some("Invalid pubkey format".to_string()) … }) }` block (lines 553, 596, 754, 790, 829, 1107, 1174, 1214, 1265, 1366, 1419, 1472, 1523 — and 4 more for contract_id_bytes/pool_owner).
  - `api/src/openapi/users.rs` — 6 occurrences.
  - `api/src/openapi/invoices.rs` — 4 occurrences.
  - `api/src/openapi/contracts.rs`, `agents.rs`, `admin.rs` — additional occurrences.
- **Inconsistency:** lines 553 and 596 of `providers.rs` emit `"Invalid pubkey hex: {e} (value: {val})"`
  (detailed, includes the parse error and the bad value), while line 754 (and ~10 others) emit the
  terse `"Invalid pubkey format"`. Same logical operation, different error fidelity depending on
  which handler you hit — a classic copy-paste evolution smell.
- **Fix sketch:** Add a shared helper in `api/src/openapi/common.rs`:
  ```rust
  pub fn decode_pubkey_path<T>(path: &Path<String>) -> Result<Vec<u8>, Json<ApiResponse<T>>> {
      hex::decode(&path.0).map_err(|e| Json(ApiResponse {
          success: false, data: None,
          error: Some(format!("Invalid pubkey hex: {} (value: {})", e, &path.0)),
      }))
  }
  ```
  and same for `contract_id_path`. Replaces ~25 match blocks with one line each.
- **Confidence:** 9 — **Safety:** 9.

### 5b. Stripe API URL hardcoded in `api/src/stripe_client.rs:585`

- **Pattern:** `"https://api.stripe.com/v1/subscription_items/{}/usage_records"` is constructed inline.
- **Why it's a problem:** Every other Stripe endpoint in the file goes through the official
  `stripe::Client` (which owns its base URL). This one raw-HTTP call hardcodes the base, so a
  future test/staging override (or a typo fix) has to find this one inline string. The ICPay client
  does it right: `const API_URL: &'static str = "https://api.icpay.org";` at `icpay_client.rs:53`.
- **Fix sketch:** `const STRIPE_API_BASE: &str = "https://api.stripe.com/v1";` and format from it.
- **Confidence:** 7 — **Safety:** 9.

---

## 6. Dead / unwired code

### 6a. `api/src/network_metrics.rs:22` — `load_ledger_metrics()` is dead
- The whole function is annotated `#[allow(dead_code)]` and is referenced **nowhere** outside its
  own module (verified: `rg load_ledger_metrics` returns only the definition and its tests don't
  call it — they call the private `load_metrics_from_file`). The module is wired into `main.rs`
  via `mod network_metrics;` but its only public surface is unused.
- **Fix sketch:** Either delete the function and the `#[allow(dead_code)]`, or wire it into a
  `/api/v1/admin/ledger-metrics` endpoint if the intent was to expose it.
- **Confidence:** 9 — **Safety:** 9.

### 6b. `api/src/icpay_client.rs:12, 71, 151` — "Prepared for payment verification feature"
- Three items marked `#[allow(dead_code)] // Prepared for payment verification feature`:
  - `IcpayPayment` struct (line 12)
  - `get_payments_by_metadata` method (line 71)
  - `verify_payment_by_metadata` method (line 151)
- None is referenced outside `icpay_client.rs` (verified: `rg IcpayPayment` and
  `rg verify_payment_by_metadata` outside the file return nothing). They have unit tests but no
  production caller.
- **Context:** Maps to GitHub issue **#420 — "ICPay: implement automated payouts when ICRC-1
  transfer API ships"**, which is in the **Deferred** list in `OPEN_ISSUES.md`. Per project rule
  "ALWAYS REMOVE ALL DUPLICATION AND COMPLEXITY … No backward-compatibility excuses", and YAGNI,
  this is technically dead code pending a deferred feature.
- **Fix sketch:** Either (a) delete now and re-add when #420 ships, or (b) leave as-is with the
  existing `#[allow(dead_code)]` annotation since the deferral is documented. Borderline — left to
  maintainer judgment.
- **Confidence:** 7 — **Safety:** 9.

---

## Notable clean areas (positive signal)

For calibration, these areas were inspected and found clean:

- **All background services** (`cloud_provisioning_service.rs`, `cleanup_service.rs`,
  `payment_release_service.rs`, `auto_renewal_service.rs`, `timeout_cleanup_service.rs`,
  `sla_alert_service.rs`, `publish_scheduled_service.rs`) — every error path logs via
  `tracing::error!`/`warn!`; no `let _ =`, no silent `Err(_)`. Model code.
- **`api/src/main.rs` shutdown drain** (lines 1606-1673) — proper `tokio::select!` on
  SIGINT/SIGTERM, `shutdown_tx` broadcast, `join_all` with hard timeout, task-failure counting.
- **`common/src/ssh_exec.rs`** — every `Command::new("ssh")` is wrapped in
  `tokio::time::timeout(SCRIPT_TIMEOUT, …)` with descriptive error context.
- **`dc-agent/src/provisioner/script.rs:61`** — `tokio::time::timeout(timeout, child.wait_with_output())`
  is the correct pattern; the script provisioner cannot hang.
- **`api/src/cloud/{hetzner,vultr,proxmox_api}.rs`** — all use `.timeout(REQUEST_TIMEOUT_SECS)`
  on the client builder. The convention exists; it just hasn't been propagated to Stripe/ICPay/
  OAuth/Cloudflare/Chatwoot/LLM/embeddings/upgrade/invoices/api-cli.
- **Money-flow in `database/contracts/rental.rs`** — the R1–R10 fixes from 2026-07-23 are present
  and correct: `calculate_net_refund_e9s` is shared between Stripe and ICPay cancel paths;
  `refund_issued` gate prevents flipping `payment_status='refunded'` without a real refund id;
  the Stripe-required-in-prod boot gate (`require_stripe_in_prod`) is at `main.rs:1142-1148`.

## Asymmetry worth a follow-up issue (not strictly a defect)

`api/src/main.rs:1142` enforces `require_stripe_in_prod(&environment)` — server refuses to start
in prod without `STRIPE_SECRET_KEY`. There is **no equivalent `require_icpay_in_prod`** even though
ICPay IS a selectable tenant payment method (`RentalRequestDialog` defaults to ICPay for ICP, and
`api/src/openapi/contracts.rs:904` calls `IcpayClient::new().ok()` on the cancel path). If
`ICPAY_SECRET_KEY` is removed in prod while ICPay contracts are outstanding, cancels will compute
the refund amount, log a `tracing::warn!`, and leave `payment_status` untouched (the R5 protection
*does* prevent falsely claiming a refund) — but the customer's money is not returned and ops only
sees a warn. A symmetric boot-time gate would close this. Worth a GitHub issue, not an emergency.

---

## Verdict

No launch-blocking defects found. The 2026-07-23 money-safety work holds up. The findings above are
**hardening opportunities**, ranked:

1. **Add `.timeout()` to all `reqwest::Client` builders** (Section 2 — money/identity path first).
   Single highest-leverage change. The pattern already exists in `api/src/cloud/`.
2. **Fix the two `receipts.rs` silent `if let Ok` on the refund notification path** (Section 1a/1b).
3. **Extract the `decode_pubkey_path` helper** to kill the 16× boilerplate + error-message
   inconsistency (Section 5a).
4. **Split `api/src/openapi/providers.rs` (6643 LOC) and `website/src/lib/services/api.ts` (4276 LOC)**
   — both have already-known section boundaries that map 1:1 to submodules.

