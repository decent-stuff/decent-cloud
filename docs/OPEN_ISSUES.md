# Open Issues

**Snapshot:** 2026-08-08. **Canonical source:** GitHub Issues at `decent-stuff/decent-cloud`
(`gh issue list --repo decent-stuff/decent-cloud --state open`). This file is a categorized
inventory for quick local reference; GitHub remains the source of truth. Re-sync with:

```bash
gh issue list --repo decent-stuff/decent-cloud --state open --json number,title,labels
```

## Next session priorities (2026-08-08)

Everything remaining, ordered by autonomy level. **Goal: tackle all of these.**

### A. Autonomously actionable (no human/credential/infra input needed)

| Priority | Task | Detail |
|----------|------|--------|
| ~~**A1**~~ | ~~**#451 — Chatwoot dedicated service-account token**~~ **DONE** | Verified working: dev Chatwoot is fully provisioned (Account 1, Inbox 1, bot user `api@decent-cloud.org` + 2 user tokens + 2 platform tokens). All tokens in SOPS match the DB. Network-isolated from agent container but functional in prod/stage where API + Chatwoot co-locate. No code change needed. |
| **A3** | **#444 — continue large-file splits** | Ongoing. Current largest: `providers.rs` 4090L, `dc-agent/src/main.rs` 3674L, `database/offerings.rs` 2876L, `database/cloud_resources.rs` 2445L. Each verified byte-identical OpenAPI via `spec_snapshot.rs` guard. Roadmap: `docs/plans/2026-07-25-large-file-splits-444.md`. |
| **A4** | **~~#425 — Audit Provisioning → Cancelled failure paths~~** | **DONE (2026-08-08).** Root cause: dc-agent sent `"provision-failed"` (unparseable — parser only accepts `"provisioning_failed"`) AND the handler routed through bare `update_contract_status` (no refund). Fix: (a) parameterized `mark_provisioning_failed` actor, (b) handler now routes provider failures through the money-safe path (gated refund + cloud-resource teardown), (c) fixed wire string, (d) cloud-resell failures now proactively drive contract to `ProvisioningFailed`. 4 tests (parse guard, wire-string, provider-actor refund, user-cancel regression). Commit: `8aee2e6f`. |
| **A5** | **#334 / #387 — DB test coverage / concurrent ticket processing** | #334: largely addressed (kept open on literal reading). #387: single-threaded poll loop, needs a design before work. Both code-only. |
| **A6** | **~~Push 33 unpushed commits~~** | **DONE by operator (2026-08-08).** Commits pushed; prod + stage deployed + verified. Origin now current. |
| **A7** | **~~Remaining robustness items (ROB-005/006/009/012/013)~~** | **DONE (2026-08-08).** All 5 items addressed: ROB-005 (bounded all blocking Command spawns with timeouts via shared helper), ROB-006 (tcp_keepalive on proxy upstream), ROB-009 (logged fs::remove_file cleanup), ROB-012 (named poll-interval consts), ROB-013 (verified already complete — all waitForResponse calls had timeouts). Commits: `932df94f`, `438632dd`, `c950bb33`, `21b80efe`. |
| **A8** | **~~UX-009 — Dashboard banner wall consolidation~~** | **DONE (2026-08-08).** Two stacked full-width banners replaced with a single compact dismissible one-line action indicator (`ActionRequiredBanner.svelte`) that expands inline. Old banner components deleted. Commit: `10424378`. |
| **A9** | **~~UX-006 — Reputation leaderboard~~** | **DONE (2026-08-08).** Full-stack: `GET /api/v1/reputation/leaderboard` endpoint (honesty-gated `WHERE total_contracts > 0`, sorted by trust_score), "Top Providers" section on `/dashboard/reputation`, shared trust-score helper extracted (DRY), e2e coverage. Commits: `ddd113b6`, `938a2d28`, `046e818a`. |
| **A9** | **UX-006 — Reputation leaderboard (user-confirmed must-have)** | Split HIGH into 3 MEDIUM subtasks: (a) Backend: `GET /api/v1/reputation/leaderboard` — top providers by trust score + completed contracts, paginated; (b) Frontend: browseable "Top Providers" section on `/dashboard/reputation` (currently search-only dead-end); (c) E2e: coverage for leaderboard rendering + search. The product direction mandates this (`docs/PRODUCT-DIRECTION.md`: "a top-providers leaderboard so reputation is browseable by default"). The trust-score calculator already exists (`website/src/lib/utils/trust-score.ts`). |

### B. Needs operator action (deploy / GitHub settings / infra)

| Priority | Task | Detail |
|----------|------|--------|
| **B1** | **~~Close 4 fixed GH issues~~** | **DONE by operator (2026-08-08).** Pushed + deployed. GH issues to close: #466, #452, #453, #447. |
| **B2** | **#470 auto-merge prerequisite** | Needs GitHub repo settings: "Allow auto-merge" ON + `build-and-test` as branch-protection required check. Code is sound (`49843e48`). |
| **B3** | **~~OP-1 — Redeploy prod~~** | **DONE (2026-08-08).** Prod verified: health 200, environment "prod", Google OAuth enabled, 2 REAL offerings (tada $7/mo + hetzner-reseller $6.82/mo, both USD — demos gone). |
| **B4** | **~~OP-2 — Redeploy stage~~** | **DONE (2026-08-08).** Stage verified at `stage-api.decent-cloud.org`: health 200, environment "stage", 5 offerings (storage/compute/network, all USD). |
| **B5** | **~~OP-4 — Complete stage DNS cutover~~** | **DONE (2026-08-08).** `stage-api.decent-cloud.org` resolves + serves 200. Legacy `dev-api.decent-cloud.org` returns 502 (dead, as expected post-cutover). |
| **B6** | **~~OP-5 — Populate GitHub repo vars~~** | **DONE (2026-08-09).** GitHub repo Variable `CHATWOOT_BASE_URL=https://dev-support.decent-cloud.org` + Actions Secret `CHATWOOT_WEBSITE_TOKEN` set. CI uses the **dev** Chatwoot instance (verified-working tokens; prod Chatwoot tokens not recoverable from agent env — in k8s only). Architecture decision: CI builds use dev Chatwoot; prod deploy gets prod config separately. Outer SOPS store synced with verified dev Chatwoot tokens (5 keys updated). |
| **B7** | **~~staging → k8s cutover~~** | **DONE (2026-08-08).** Stage serving at `stage-api.decent-cloud.org`. |

### C. Large epics (multi-session, needs design forks)

| Priority | Task | Detail |
|----------|------|--------|
| **C1** | **#418 — Decent Agents beta onboarding** | Magic-link auth → Stripe → GitHub App → demo PR → invite gate. Multi-week new product surface. Specs exist (`2026-04-25-decent-agents-*`). |
| **C2** | **#427 — Anthropic API key proxy** | Core shipped (`anthropic-proxy` crate, 33 tests). Acceptance #3/#4 need identity-provisioning subsystem (#413 impl). |
| **C3** | **#415/#416 — Decent Agents billing + metering** | Depends on #427 dispatch enforcement + `agent_runs`/metering tables. |

### D. Deferred post-launch (≥20 paying customers)

#429 (key exfiltration mitigation), #430 (CODEOWNERS deadlock UX), #431 (webhook secret rotation), #432 (per-identity observability).

## Real-deployment audit (2026-08-04)

A CDP-driven audit of the REAL prod deployment (`https://decent-cloud.org`, ns `dc-prod`) + the
Hetzner operator console mapped **19 findings** (no VM rented; cloud spend $0). Full details in
[`docs/REAL-DEPLOYMENT-ISSUES.md`](REAL-DEPLOYMENT-ISSUES.md). **Two P0s head the list:**

- **P0-A — prod OUTAGE (resolved out-of-band; PERMANENCE FIX NEEDED):** the nuc-k3s symmetry rename
  (`dc-secret`→`dc-prod-secret`, commit `86b1422`) + an unused `HETZNER_API_TOKEN` secretKeyRef stub
  (commit `4ac1b80`) were pushed WITHOUT re-applying the secret under its new name in-cluster → all
  `dc-prod` pods went `CreateContainerConfigError: secret "dc-prod-secret" not found` for ~3.5h
  (HTTP 530→502). Recovered via kubectl (copy `dc-secret`→`dc-prod-secret` + patch in
  `HETZNER_API_TOKEN` + delete stuck pods); prod now HTTP 200, `dc-api` 1/1 Running. **Permanence
  (operator):** run `manage-secrets.py` (k8s repo) so `dc-prod-secret` is reconciled from the
  renamed SOPS file. (DONE: the unused `HETZNER_API_TOKEN` secretKeyRef stub was removed from
  `base/dc-api.yaml` + the stage overlay in the k8s repo — committed locally; operator pushes
  nuc-k3s. The api-server never read it from env; only `api/src/bin/api-cli/e2e.rs` reads `_DEV`.)
- **P0-B — Path-A Hetzner offerings SILENTLY HIDDEN from the marketplace (RESOLVED — commit `a2a96862`):**
  `is_marketplace_visible` (`offerings.rs:42-46`) now includes `is_cloud_resell(o.provisioner_type)` —
  Hetzner/Vultr offerings are visible regardless of `offering_source` or pool membership. Provider-online
  status (`offerings.rs:807`) also marks cloud-resell offerings as always-online.

The remaining 17 findings (P1–P3: token-creation pitfalls, catalog/mismatch issues, console
errors, onboarding sequencing, etc.) + a `dc-stage` staging note + the full operator action list
are in `docs/REAL-DEPLOYMENT-ISSUES.md`. Surfaced by the real-deployment e2e harness (PR #459).

## Scope rules (per `repo/AGENTS.md` + `repo/PROMPT.md`)

- **In scope**: labeled `launch`, `stripe`, or `decent-agents` WITHOUT `deferred-post-launch`.
- **Deferred**: labeled `deferred-post-launch`. Valid but parked until ≥20 paying customers.

## Resolved this session (2026-08-08)

| Issue / finding | Fix | Commit |
|-----------------|-----|--------|
| **#466** — Cloud-resell race between cancel and in-flight provisioning can orphan a VM | 3 coordinated state-machine guards: `update_cloud_resource_provisioned` refuses to overwrite terminal states (returns `Result<bool>`); `mark_cloud_resource_failed` is a no-op on `deleting`/`deleted`; `provision_one` cleans up the just-created VM on concurrent-cancel detection. 3 DB-level regression tests. | `bc692bdc` |
| **#452** — Dead `CHATWOOT_INBOX_ID` config (never read by code) | Removed from `.env.example`(×2), `docker-compose.dev.yml`, `CONFIG.md`, `deploy.py` `NON_SECRET_VARS`; rewrote `support_bot/AGENTS.md` to describe the real `list_inboxes()` assign-to-all loop. | `71210f8e` |
| **#453** — `api/.sqlx` gitignored | Already resolved: workspace-root `.sqlx/` is the single committed source of truth (299 tracked query files); `api/.sqlx/` correctly gitignored (`.gitignore:120`). GH issue commented; no code change needed. | — |
| Verification finding — `poc/hetzner-provision-probe.mjs` reads bare `HETZNER_API_TOKEN` (stranded-VM foot-gun in operator-local runs) | Hard-switched to `HETZNER_API_TOKEN_DEV` (consistent with #467 agent rule). | `b5be319c` |
| SOPS `common.yaml` stored `EMAIL_PROCESSOR_INTERVAL_SECS: '30  # default'` — inline YAML comment parsed as part of value → API u64 parse failure → startup abort | Three-layer fix: SOPS values cleaned to bare numerics; `dev-server.sh` SECRETS_ENV parser strips inline comments; `cf/.env.example` cleaned. | `cc4b1c2e` |
| Latent panic + terse error messages (rewards.rs `[..8]` on short slice; ledger_cursor FromStr discards value+error; identity.rs, offerings.rs) | Bound-check before slice + echo bad value + error in all 6 cursor-parse sites + 3 more error-message improvements. New regression test for the panic guard. | `86227dc6` |
| Dev-cycle: no fast debug-binary path; broken `_announce_api_binary` suggestion; entrypoint missing `CARGO_TARGET_DIR` workaround | Added `--dev` flag to `dev-server.sh` (debug binary, honors `CARGO_TARGET_DIR`); fixed broken path suggestion; entrypoint error now leads with the no-sudo workaround. | `c5b19c67` |
| **E2e suite verification** — full 314-test Playwright suite run against warm stack | 303 passed, 5 flaky under parallel load (all pass in isolation — documented in config), 6 cascade. No code bugs found. | — |
| **#447** — Replay dispute lifecycle (pause/refund) for orphans re-linked after late checkout completion | Renamed `replay_orphan_dispute_pause` → `replay_orphan_dispute_lifecycle`; closed-lost orphans now get terminate+refund (same idempotent sequence as `handle_dispute_closed`). Previously only detected — refund was never recorded. | `36f5e550` |
| **P0-B** — Path-A Hetzner offerings SILENTLY HIDDEN from marketplace | Already fixed in `a2a96862` — `is_marketplace_visible` includes `is_cloud_resell()`. Updated OPEN_ISSUES.md to reflect. | — |
| **Chatwoot full reset (dev/stage/prod)** — all instances broken (Postgres not listening, missing DBs/roles, pgvector not installed) | Root cause: nuc Postgres only listened on `127.0.0.1`. Fixed: `listen_addresses='*'` + `pg_hba.conf` rules + pgvector built for PG14 + `chatwoot_dev`/`chatwoot_prod` DBs/roles created + migrations + full Rails initialization (SuperAdmin/Account/PlatformApp/Inbox/tokens). Dev SOPS updated; prod/stage k8s secrets patched live + persisted to nuc-k3s GitOps (`d3646f1`). All 3 instances verified working. | `8b44cb6b` (dev SOPS) + k8s repo `d3646f1` |

## Resolved this session (2026-08-08) — round 2 (Phase 2 verification + A2 hang + Phase 3 UX/robustness sweep)

20 commits total (`a039dbe2`→`e30b37d2`). All on `main`, ahead of `origin/main`. Stack never
restarted during the sweep (HMR + warm-stack e2e). All gates green: `npm run check` 0/0, e2e
smoke 32/32 (28.9s), clippy 0, nextest green on all touched crates.

### Phase 2: recent-commit verification (8 code commits audited)

| Finding | Fix | Commit |
|---------|-----|--------|
| Stale rustdoc intra-doc link `[replay_orphan_dispute_pause]` after the `36f5e550` rename | Updated to `[replay_orphan_dispute_lifecycle]` | `ee79347b` |
| **All 8 code commits (`b5be319c`–`8b44cb6b`) verified CLEAN** — no dead/partially-wired code | Independent verifier subagent confirmed: race-guard callers handle `Result<bool>`, dispute replay idempotent, panic fixed with regression test, `--dev` flag correctly wired, SOPS values cleaned. | — |

### A2: e2e parallel-hang root-caused + fixed

| Finding | Fix | Commit |
|---------|-----|--------|
| **E2e smoke hangs under parallel workers** (~1 in 8+ runs, serial mode always passes) | Root cause: `seed-helpers.ts` `sql()` used `execFileAsync('psql')` with NO timeout; worker-scoped fixture teardown (`DELETE FROM accounts`) blocks on an FK lock from in-flight API traffic, and teardown runs OUTSIDE the per-test timeout → indefinite worker stall. Fix: centralized every `psql` call behind `psqlExec` (15s default timeout), bounded `signedApiCall`'s `fetch` with `AbortSignal.timeout`, added explicit timeouts to 3 unbounded `waitForResponse` calls in `keyboard-shortcuts.spec.ts`. Regression test proves IO is now bounded. ~20 consecutive 26/26 green parallel runs post-fix. | `a5d1ac94`, `ff87a05e` |

### Phase 3a: UX audit (no-mock, real warm stack) — 13 issues found, 11 fixed

| UX issue | Fix | Commit |
|----------|-----|--------|
| **UX-001** (CRITICAL) Homepage hero shows hardcoded FAKE provider trust data (`provider_alpha`, "87 Trust Score", "Verified Provider" badge) — violates PRODUCT-DIRECTION | Replaced with honest "Anatomy of a Trust Score" educational graphic | `25945664` |
| **UX-002** (CRITICAL) Dead ICP "Validators" feature in primary nav + dead stats | Removed entirely (route, nav, client fn, type, ICP metrics) | `e2b97ed9` |
| **UX-005** Homepage stats grid mixes live + dead metrics | Cleaned to 4 honest stats (dropped confusing "Active Providers: 0" + ICP metrics) | `e2b97ed9` |
| **UX-007** ICP-era marketing copy overpromises; stale whitepaper in footer | Toned down absolutes; removed stale whitepaper link | `07c4121c` |
| **UX-008** Unauth sidebar has redundant auth prompts + misleading "My Activity" | Collapsed to single Sign In CTA; "My Activity" hidden for signed-out | `2dd8e373` |
| **UX-012** Auto-playing hero typing animation (motion accessibility) | Respects `prefers-reduced-motion` | `25945664` |
| **UX-010** Focus indicators 1px (WCAG 2.2 borderline) | `:focus-visible` outline 1px→2px, offset 1px→2px | `ca98c3b5` |
| **UX-013** Sign-up page titled "Sign In" | Relabeled "Sign In or Create Account" | `9bd2eebf` |
| **UX-004** Dashboard shows raw IC principal as identity | Shows `@username` (principal preserved on Account→Profile) | `a2b10565` |
| **UX-011** Keyboard shortcuts undiscoverable | Clickable `?` kbd badge next to "Ctrl K" palette trigger | `f1b0c3d8` |
| **UX-003** Seed-phrase-only auth is a wall for non-crypto buyers (OAuth enablement = user decision: YES) | Frontend: inline "What is a seed phrase?" education on auth chooser + prominent permanent-loss warning on backup step + more discoverable recovery link. OAuth was already enabled server-side by operator (prod `{"google_oauth":true}`). 3 new `@smoke` e2e tests. | `5abeda55` |
| **UX DEFERRED** UX-006 (reputation leaderboard — user confirmed must-have, scheduled as A9), UX-009 (dashboard banner wall — scheduled as A8), UX-014 (footer community links unverified) | UX-006 split into 3 MEDIUM subtasks (see A9); UX-014 needs live verification | — |

### Phase 3c: Rust robustness sweep — 11 findings, 8 fixed

| ROB issue | Fix | Commit |
|-----------|-----|--------|
| **ROB-001** (HIGH) `From<&str>` panics on malformed principal | Fallible `IcrcCompatibleAccount::parse()` + SAFETY comment; caller audit confirmed no untrusted input reaches panic | `aff8bd2c` |
| **ROB-002** (HIGH) Metering `.ok()` silently drops usage (under-billing) | `tracing::warn!` (error + truncated body) before returning None; 3 tests | `ff181648` |
| **ROB-003** (MED) Hetzner price parse silent drop | Per-field `tracing::warn!` (server_type_id/name + field + value + error); valid fields preserved | `e37e0dc6` |
| **ROB-004** (MED) PublishScheduledService interval hardcoded | `parse_env_u64("PUBLISH_SCHEDULED_INTERVAL_SECS", 60)` + env examples updated | `73c8f4ff` |
| **ROB-007** (MED) Docker metrics silent None | 5 sites match-style + `tracing::warn!`; 2 tracing_test tests | `e5bf5f96` |
| **ROB-008** (LOW) `let _ = child.kill()` silent | Shared `best_effort_kill_and_reap()` helper logs at WARN (DRY) | `f7a19ebe` |
| **ROB-010** (LOW) Fragile `.unwrap()` on just-constructed Option | Removed; direct method call instead | `7066d26d` |
| **ROB-011** (LOW) `borsh::to_vec().unwrap()` in business logic | `.expect()` with type-specific invariant messages | `8304840f` |
| **ROB DEFERRED** ROB-005 (blocking `process::Command` w/o timeout), ROB-006 (proxy total timeout — deliberate for streaming), ROB-009 (fs cleanup unlogged), ROB-012 (magic numbers), ROB-013 (e2e waitForResponse timeouts) | Tracked as **A7** in next-session priorities | — |

### Phase 3b: E2e regression guards

| Finding | Fix | Commit |
|---------|-----|--------|
| No e2e regression guards for the UX cleanup | 6 new `@smoke` tests in `ux-regression-guards.spec.ts` (UX-001/002/005/004/008/013); FLOWS.md §6 added; smoke 26→32 (29.5s). Coverage gaps documented (UX-010 computed-style fragile, UX-012 reduced-motion testable via context). | `e30b37d2` |

### Round 3 (2026-08-08/09): UX-003, UX-006, A4, A7, A8, dead-transfers removal, agent-instructions

| Issue / finding | Fix | Commit |
|-----------------|-----|--------|
| **UX-003** Seed-phrase-only auth is a wall for non-crypto buyers | Inline seed-phrase education on auth chooser + prominent permanent-loss warning on backup step + more discoverable recovery link. 3 new `@smoke` e2e tests. | `5abeda55` |
| **UX-006** Reputation page is a dead-end search box (no browseable leaderboard) | Backend: `get_reputation_leaderboard(limit)` DB method + `GET /reputation/leaderboard` endpoint (honesty gate `WHERE total_contracts > 0`). Frontend: "Top Providers" section on reputation landing + shared trust-score helpers (DRY extraction). E2e + unit tests. | `ddd113b6`, `938a2d28`, `046e818a` |
| **A7** Remaining robustness items (ROB-005/006/009/012) | ROB-005: all blocking `Command` spawns bounded with timeouts (shared `spawn_with_timeout` DRY). ROB-006: anthropic-proxy `tcp_keepalive(30s)`. ROB-009: `fs::remove_file` cleanup logged at `debug!` (DRY helper). ROB-012: inline `Duration::from_secs(5)` → named consts. ROB-013: verified already complete. | `932df94f`, `438632dd`, `c950bb33`, `21b80efe` |
| **A8/UX-009** Dashboard "banner wall" (two stacked full-width banners) | Consolidated into single compact dismissible `ActionRequiredBanner.svelte` (collapses to one-line "N actions needed" + expands inline). Deleted 2 old banner components. E2e rewritten (5 tests). | `10424378` |
| **A4/#425** Provisioning failure paths don't reach `ProvisioningFailed` cleanly (money-safety bug) | Root cause: dc-agent wire string `"provision-failed"` doesn't parse; even if it did, handler skipped the gated refund. Fix: parameterized `mark_provisioning_failed(actor)`; handler now routes `ProvisioningFailed` through money-safe path; fixed wire string to canonical `"provisioning_failed"`; cloud-resell failures proactively drive contract to `ProvisioningFailed`. 4 tests (parse guard, wire string, provider-actor refund DB integration, user-cancel regression guard). | `8aee2e6f` |
| **Dead ICP token-transfers feature** (`/dashboard/transfers` + 3 API endpoints + DB reads) — sync service never runs in `serve`; table permanently empty; always returns 0/[] | Removed entirely: page, API endpoints (`TransfersApi`), DB read methods, frontend functions/types, e2e test. Also removed dead "Token Balance" card from earnings page + "Total Transactions" section from reputation detail. Kept table/migration/SyncService (sync CLI still compiles). Spec snapshot 187→185 paths. **1019 lines deleted.** | `4dc37015`, `4eca3e4b` |
| Agent instructions: outer-store-first rule unclear | Both repo + outer AGENTS.md credential sections rewritten with two-store hierarchy table (OUTER primary = all integration secrets; REPO-INTERNAL = deploy subset). | `9bcfb765` (repo), `109ff66` (outer) |
| **A1/#451** Chatwoot dedicated service-account token | Verified working: dev Chatwoot fully provisioned (Account 1, Inbox 1, bot user, 4 tokens). All tokens in SOPS match DB. Network-isolated from agent container but functional in prod/stage. | — (no code change) |

## Resolved this session (2026-08-06)

Findings fixed by the 2026-08-05 sweep (branch `sweep-2026-08-05`); see plan
`docs/plans/2026-08-05-sweep-continuation.md`. One line per fix + the commit SHA. Full per-commit
detail is in the plan's STATUS block.

| Finding (source) | Fix | Commit |
|------------------|-----|--------|
| Stats `active_providers` reads retired ICP `provider_check_ins` (Wave-A F1 / real-deploy F5) | Now reads LIVE `provider_agent_status` (online agents) | `3b372ae9` |
| Stats `total_offerings` disagrees with marketplace list | Aligned to the same marketplace pool rule | `3b372ae9` |
| api `SyncService::new` `.expect()` panics on I/O | `SyncService::new` → `Result`, propagated | `3b372ae9` |
| Doctor had no example-offering guard | New `DOCTOR_EXAMPLE_OFFERINGS_PRESENT` guard (FAIL-in-prod) | `3b372ae9` |
| Zombie demo UI code after migration 053 (Wave-A F2) | Removed `showDemoOfferings`/`?demo=`/`Demo-only` badges | `c32177f3` |
| Rent dialog OS selector defaults to placeholder (Wave-A F3) | Defaults to offering's first OS | `c32177f3` |
| Trending surfaces offline/unrentable offerings (Wave-A F4) | Trending strip excludes known-offline offerings | `c32177f3` |
| "Welcome back" shown to first-time users (Wave-A F6) | First-visit dashboard greeting | `c32177f3` |
| dc-agent docker image-pull unbounded (could wedge contract lock) | 600s overall timeout | `e64b37a8` |
| dc-agent gateway iptables silent errors (3 sites) | Surfaced via `match` + `warn!` | `e64b37a8` |
| Real-deploy harness marketplace flow passes on dishonest catalog (all-demo / zero-rentable) | Marketplace-honesty assertions (FAIL-on-prod) | `f6c37458` |
| E2E README stale (wrong ports, wrong smoke count) | Rewrote 318→~95 lines, correct ports | `eb3f8ba5` |
| E2E category `--grep @rental` returns 0 (tags doc-only) | Fixed FLOWS category-run section | `eb3f8ba5` |
| #444 `contracts.rs` 2244L (mechanical split candidate) | Split 2244→1745 + new `contract_telemetry.rs` (527L) | `352f2355`,`a1f1a2f0` |

## Resolved this session — round 2/3 (2026-08-06)

Follow-on sweep on the same branch (`sweep-2026-08-05`), driven by user feedback + CI/release
verification. 7 commits on top of round 1 (origin/main `252c7f76` → `45953812`). spec_snapshot
unchanged (187 paths / 327 schemas); full Playwright suite now **314/0**. No code migration needed —
all fixes are projections of existing data or build/config wiring.

| Finding (source) | Fix | Commit |
|------------------|-----|--------|
| Workspace version drifted (0.5.3) vs release-tag consistency target | Bump 0.5.3→0.5.5 for release-tag consistency | `28740f20` |
| Dead `is_example` concept — DERIVED field (never a column), always false since migration 053; polluted SQL projection, `$N` param threading, frontend serialization | Removed `is_example` ENTIRELY (field/SQL projection/`$N` param threading/frontend serialization) + removed the now-obsolete `DOCTOR_EXAMPLE_OFFERINGS_PRESENT` doctor guard. `example_provider_pubkey()` RETAINED (2 live endpoints `/offerings/template/:product_type`, `/offerings/product-types` + a fresh-DB guard use it; full retirement is a separate coordinated change). spec_snapshot 187/327 unchanged | `9e16e677` |
| Chatwoot widget rendered unconditionally — 404 iframe + console errors on every prod page; hardcoded dead `support.decent-cloud.org` default | Env-gate: widget renders ONLY when both `websiteToken`+`baseUrl` set; removed the hardcoded default; `release.yml`+`cf/deploy.py` wire `VITE_CHATWOOT_*` build vars | `2978b0ad` |
| CI: 2 Playwright regressions from demo-removal; `release.yml` empty `DC_REPO_WRITE` token override; `--locked` build failed (no Cargo.lock) | Repaired 2 e2e regressions; removed empty `DC_REPO_WRITE` checkout override; committed Cargo.lock for `--locked` | `89b6dbb2` |
| Provider sidebar collapsed by default; Cloud Accounts route had no nav link (full UI + working Hetzner key validation already shipped) | Sidebar OPEN by default; added Cloud Accounts nav link | `87c45517` |
| Cloud-resell (Hetzner/Vultr) offerings invisible in marketplace without a pool — user-reported "Hetzner requires a pool" bug | DRY `is_cloud_resell`/`is_marketplace_visible` helpers (BackendType SSOT); cloud-resell offerings now marketplace-visible WITHOUT a pool. Live-verified | `a2a96862` |
| 2 PRE-EXISTING Playwright failures: account-page ambiguous `Account` selector; offerings-editor-replace depended on example-provider templates dropped by 053 | Root-caused both: exact-match selector; spec now self-seeds like its sibling. Full suite 314/0 | `45953812` |

Also a read-only **k8s manifest audit** (no manifest changes needed): migrations auto-run
unconditionally at boot (`database/core.rs:15`), website Chatwoot config is build-time-baked (no
runtime env needed), stage overlay correct + isolated, probes adequate, image-tag policy sound. One
flagged non-action → tracked as OP-5 below (since resolved: a dead env var was dropped, #452).

## Operator / deploy blockers (need human)

NOT autonomously fixable — require a deploy or operator action. Each lists concrete evidence, the
required action, and the autonomous guard that now catches a regression. Drift from these is what
the sweep's new guards surfaced.

- **OP-1 (CRITICAL) — Prod marketplace serves 10 synthetic demo offerings.** Evidence:
  `GET https://api.decent-cloud.org/api/v1/offerings?limit=20` returns 10 rows all `is_example:true`,
  all under fake pubkey `6578616d706c652d...` (ASCII `example-offering-provider-identifier`). Cause:
  migration `053_drop_example_provider_seed.sql` (PR #456) never applied to the prod DB (the migration
  is an unconditional `DELETE`; if it ran, the rows would be gone) — i.e. prod was never redeployed at
  ≥ these commits. Violates the honest-catalog goal in `docs/PRODUCT-DIRECTION.md`. (This supersedes
  the 2026-08-03 "Deferred — UX" F2 note, which recorded demos as dropped: the migration landed in
  code, but the prod DB never received it.) **ACTION:** redeploy prod at ≥ these commits; verify
  `SELECT count(*) FROM provider_offerings WHERE pubkey='\x6578616d706c652d...'` = 0. **The
  `is_example` concept is now removed from code entirely** (`9e16e677`, round 2/3) — there is no
  longer an `is_example` field/projection/serialization at all (it was always derived, always false
  since 053). **A working deploy runbook exists:** migrations auto-run unconditionally at boot
  (`database/core.rs:15`), so retagging the image + `release.yml` building it + ArgoCD syncing
  applies `053` automatically (confirmed by the 2026-08-06 k8s manifest audit — probes adequate,
  image-tag policy sound). The prod data fix still needs the redeploy itself. **Autonomous guard:**
  the `DOCTOR_EXAMPLE_OFFERINGS_PRESENT` doctor guard was REMOVED in `9e16e677` (obsolete once
  `is_example` is gone); the harness marketplace-honesty assertion (`f6c37458`) remains the live
  FAIL-on-prod guard.
- **OP-2 — Stage (dev-api) is stale.** Evidence: `GET /api/v1/auth/capabilities` → 404 on stage (prod
  200); stage offerings priced in the retired ICP currency (`currency:ICP`,
  `payment_methods:ICP,ckBTC`) while prod is USD/Stripe. **ACTION:** redeploy stage from the current
  image (`python3 cf/deploy.py deploy stage` → operator `git push` the k8s repo).
- **OP-3 — PROD Chatwoot support widget broken on every page.** Evidence: widget URL
  `https://support.decent-cloud.org/widget?website_token=yDZeiDhpXW5UEhwPVFmgJAkg` → HTTP 404; the host
  sends `X-Frame-Options:SAMEORIGIN` which blocks the iframe (`ERR_BLOCKED_BY_RESPONSE`, console error
  on all prod routes; dev-web is clean). **Round 2/3 update (`2978b0ad`):** the widget is now
  env-gated — it renders ONLY when both `websiteToken`+`baseUrl` are set, and the hardcoded dead
  `support.decent-cloud.org` default is removed, so prod no longer emits console errors when the
  tunnel is down. The widget is still NOT functional in prod because `support.decent-cloud.org`
  remains **dead-infra**. **2026-08-08 UPDATE:** Chatwoot instances were fully reset (dev/stage/prod
  all working with valid tokens). The widget infrastructure now works — the remaining steps are:
  (1) disable `X-Frame-Options` / set allowed origin for decent-cloud.org in Chatwoot/Rails config;
  (2) populate the GitHub repo vars so CI bakes the widget config (see OP-5, now unblocked).
- **OP-4 — `stage-*` hostnames do not resolve (k8s dc-stage cutover incomplete).** Evidence:
  `stage-api.decent-cloud.org` + `stage.decent-cloud.org` DNS-fail; `dev-*` is the de-facto stage.
  **ACTION:** complete `docs/MIGRATION-CUTOVER.md` Step D (public tunnel/DNS cutover) OR update the
  docs to state `dev-*` is the only live stage. This overlaps the existing k8s staging→dc-stage
  cutover blocker tracked below (see "Infrastructure — staging → k8s … consolidation"); it is the
  public-DNS slice of that same cutover, not a separate plan.
- **OP-5 — Populate GitHub repo Variable `CHATWOOT_BASE_URL` + Actions Secret `CHATWOOT_WEBSITE_TOKEN`
  (for the release.yml website build).** **RESOLVED (2026-08-09):** both set in `decent-stuff/decent-cloud`.
  CI uses the **dev** Chatwoot instance (`dev-support.decent-cloud.org` + dev website token) — the
  verified-working instance. Architecture decision: CI/dev/stage use dev Chatwoot; prod gets prod
  Chatwoot config at the prod deploy step (separate from CI). Outer SOPS store synced with the 5
  verified dev Chatwoot keys (API/platform/website tokens, account ID, Postgres password).
  **Prod Chatwoot tokens** (`support.decent-cloud.org` — live, 302) are in k8s secrets only; not
  recoverable from the agent env. If prod needs its own widget baked into the prod image, extract
  the prod website token from k8s (`kubectl -n dc-prod get secret dc-secret -o jsonpath='{.data.CHATWOOT_WEBSITE_TOKEN}' | base64 -d`) and either build a separate prod image or make the widget
  runtime-configurable.

## Future work

Proposals not yet filed as GitHub issues — distinct from the open/deferred tables
above. Each is a forward-looking design note for a future session.

### Future work: Retire ICP LedgerClient/MetadataCache polling in `serve`

- **Problem:** `serve` still initializes a `LedgerClient` against IC mainnet (`ggi4a-wyaaa-aaaai-actqq-cai`) and polls token metadata every 60s (`metadata_cache.rs`). The `token_transfers` feature that consumed this data was removed (dead ICP-era code — the sync service only ran via the `sync` CLI, never via `serve`). The metadata cache is now consumed by nothing material.
- **Scope:** Remove the `LedgerClient` init from `serve`, the `metadata_cache` background task, and the `MetadataCache` struct — IF the `sync` CLI subcommand can be decoupled (it still uses `SyncService` which depends on `LedgerClient`). If the sync subcommand is also dead (nobody runs it — `sync_state.last_sync_at` is 2+ weeks stale), retire both together.
- **Status:** Proposed (future session). Not blocking — the polling is harmless but wasteful (60s mainnet queries for cached metadata nothing reads).

### Future work: API key / service token (non-custodial automation auth)

- **Problem:** provider auth today requires the Ed25519 master key (derived from
  the BIP-39 seed). Automation (CI, the AI agent, the real-deployment e2e
  harness) must therefore hold the master seed — a root-credential exposure. As a
  pragmatic unblock, the prod `hetzner-reseller` seed is currently stored in the
  agent-accessible age-tier `secrets/shared/env.yaml` (operator-local outer store,
  NOT in any public repo) as `DC_PROD_RESELLER_SEED` / `DC_PROD_RESELLER_PUBKEY`.
- **Proposal:** add a non-custodial "API key / service token" feature: a provider
  mints one or more scoped, revocable tokens (stored hashed in the DB, mirroring
  the `cloud_accounts.credentials_encrypted` pattern) that authenticate API
  requests WITHOUT the master key. Automation authenticates with the token; the
  master seed stays fully offline.
- **Why:** aligns with `docs/PRODUCT-DIRECTION.md` (operator reselling at scale;
  programmatic provider management); removes root-credential exposure from
  automation; enables per-token scoping (e.g. read-only, offerings-only) +
  rotation without identity churn. The signing scheme (`api/src/auth.rs` +
  `common/src/api_auth.rs`) would gain a token-auth path alongside the Ed25519
  path.
- **Status:** Proposed (future session). Not blocking — the env.yaml-seed path
  works today (see `repo/AGENTS.md` → "Acting as an existing provider identity
  autonomously").

## Infrastructure — staging → k8s (`dc-stage`) consolidation — PoC VERIFIED, cutover pending

**Status (2026-08-03):** Tracks 1+2+3 done autonomously; **Track 2 PoC VERIFIED LIVE**
(dc-stage serves HTTP 200, DB migrated, prod untouched); **operator cutover (8 items
below) is the only remaining work.** Authoritative runbook:
`docs/MIGRATION-CUTOVER.md`. Plan:
`docs/plans/2026-08-03-staging-to-k8s-dc-stage-consolidation.md`.

What shipped autonomously (DONE):
- **Track 1 (k8s manifests, committed locally):** kustomize base/prod/stage
  overlays, `cluster/core/dc-stage.yaml`, `dc-stage-secret.yaml.template`, the
  `decent-cloud-stage` ArgoCD App CR. ✅ manifests authored + prod-overlay
  byte-equivalence verified.
- **Track 2 (`dc-stage` live on cluster, VERIFIED):** namespace + registry pull
  secret + in-cluster `dc-stage-secret`/`dc-stage-config` + dedicated role
  `decent_cloud_stage` + DB `decent_cloud_stage` (52 migrations auto-applied,
  86 tables) in the shared `pgsql` app; api/website/redis reusing prod's image
  tag (`445a17d4`). ✅ **Health VERIFIED** — port-forward → `/api/v1/health`
  HTTP 200 (`{"success":true,"message":"Decent Cloud API is running","environment":"stage"}`).
  `dc-api-sync` scaled to 0 (PoC safety). All Services ClusterIP-only — dev tunnel
  untouched, `dc-prod` untouched. Stripe in TEST mode (`sk_test_`). (Two bugs found
  + root-cause fixed during bring-up: SMTP `configMapKeyRef` overlay fix `deb4018`;
  stage hostPath `chown 1000:1000`.)
- **Track 3 (product repo, in PR #454):** `cf/deploy.py deploy stage` +
  `config stage`; the cutover runbook; AGENTS/docs updated. ✅ product-repo prep
  shipped. Legacy `dev` docker-compose path retained until the cutover retires it.

**Operator-gated (OPEN — the cutover, runbook steps A–G + minor follow-ups):**
1. **Push the k8s repo** → ArgoCD adopts live dc-stage. ⚠️ Push BOTH commits `7013258`
   (base/prod/stage split) + `deb4018` (SMTP overlay fix) together — else ArgoCD
   re-applies the broken patch. (Step A.)
2. **⚠️ CRITICAL — reconcile the stage DB password before ArgoCD's first sync.**
   The `decent_cloud_stage` role password lives ONLY in the live `dc-stage-secret`
   (kubectl-created, not SOPS). Either extract it + SOPS-encrypt into
   `cluster/secrets/dc-stage-secret.yaml`, or set your own + `ALTER ROLE … PASSWORD`
   to match BEFORE the sync — otherwise ArgoCD overwrites the live Secret and
   breaks DB auth. (Step B, CRITICAL note.)
3. **Encrypt + persist the full `dc-stage-secret`** to git (SOPS PGP key
   `FA5814CF1935EE80C454C9F1660DCCF069EC9176`). (Step B.)
4. **Ship `:stage` image tag** in CI; update the stage overlay from `445a17d4` →
   `:stage`. (Step C — optional until CI builds it.)
5. **Public cutover:** repoint the `decent-cloud-dev` cloudflared tunnel → dc-stage
   services + DNS `dev-*`→`stage-*` (or keep `dev-*`); verify
   `https://api.stage.decent-cloud.org/api/v1/health` 200. (Step D — the switch.)
6. **Re-enable `dc-api-sync`:** `kubectl -n dc-stage scale deployment dc-api-sync
   --replicas=1`. hostPath perms are already fixed — verify Ready. (Step E.)
7. **Tear down the old dev host** (Step F) + **delete retired files**
   (`cf/docker-compose.dev.yml`, `scripts/dc-secrets`, `repo/secrets/shared/`, the
   `dev` path in `cf/deploy.py`) — **separate commit, only after F** (Step G).
8. **Minor follow-ups:** populate `TWILIO_AUTH_TOKEN` in `env.yaml` (empty — SMS
   escalation disabled in stage); ~~reconcile `CHATWOOT_PLATFORM_API_TOKEN` (stale →
   401)~~ **(RESOLVED 2026-08-08: Chatwoot fully reset, all tokens valid)**; optionally drop
   `SMTP_PASSWORD` from the api secret (unused — api sends via MailChannels).

Until the cutover completes, the live `dev` docker-compose host still serves
staging traffic and the age store is still in the repo. Do not pre-delete.

## In scope (active work)

> **2026-08-03 RE-CORRECTION (supersedes the stale "BLOCKED on credentials / #413" claims in the
> 2026-07-25 session entries below):** the old "blocked" status was re-verified and is **FALSE**.
> (a) **All credentials are present** in the consolidated `secrets/shared/env.yaml` store
> (`ANTHROPIC_API_KEY`+`ANTHROPIC_BASE_URL`+`ANTHROPIC_MODEL`, `STRIPE_SECRET_KEY`+publishable+webhook,
> `GOOGLE_OAUTH_CLIENT_ID`+secret+redirect, `GITHUB_API_TOKEN`+`GITHUB_TEST_PAT`, `MAILCHANNELS_API_KEY`,
> `SMTP_*`, `TELEGRAM_BOT_TOKEN`, `CF_API_TOKEN`+`CF_ZONE_ID`, `HETZNER_API_TOKEN`, …). (b) **#413 was
> already declared closed as a blocker** in the 2026-07-26 session, and the architecture is decided in
> the **2026-04-25 specs** (`docs/specs/2026-04-25-decent-agents-identity-provisioning-spec.md` +
> `…-github-integration-spec.md`). What remains is **BUILDING** the identity-provisioning subsystem
> (the `agent_identities` table + dispatch wiring the `anthropic-proxy` crate already references) and
> the onboarding/billing/metering flows on top of it — these are specced, unblocked, ready-to-build
> epics, NOT externally blocked. The earlier "BLOCKED" wording in historical entries is struck below
> where it still appears.

| # | Title | Labels | Notes |
|---|-------|--------|-------|
| 418 | Decent Agents: beta onboarding (invite + first-run demo) | launch | First user-facing DA flow (magic-link/Google auth → Stripe → GitHub App → demo PR → invite gate). **Unblocked.** Spec: `2026-04-25-decent-agents-github-integration-spec.md`. Needs the identity-provisioning foundation (#413 impl) + GitHub App onboarding flow (no webhook receiver exists yet). |
| 427 | Anthropic API key proxy/sidecar for per-identity isolation | decent-agents, launch | **Core shipped** (`anthropic-proxy` crate: injects key per-request, meters per identity, streams, redacts; 33 tests green; references `agent_identities.id`). Acceptance **#3/#4** (remove shared-key mount + migrate beta) need the identity-provisioning subsystem built — **unblocked**, not waiting on a decision. |
| 416 | Decent Agents: usage metering + customer-facing usage dashboard | decent-agents | Depends on #415 meters (no `agent_runs`/metering tables exist yet — to build). **Unblocked.** |
| 415 | Decent Agents: subscription billing with active-hour + Claude token caps | decent-agents | Meters, caps, Stripe cycle rollover. `STRIPE_SECRET_KEY` present. **Unblocked.** |

## Deferred — Decent Agents

| # | Title |
|---|-------|
| 432 | Decent Agents: per-identity observability + incident response runbook |
| 431 | Decent Agents: GitHub App webhook secret rotation procedure + ops runbook |
| 430 | Decent Agents: CODEOWNERS / branch protection deadlock surfaced to customer at onboarding (also launch) |
| 429 | Decent Agents: Anthropic key exfiltration mitigation (read-only mounts, egress monitoring) |

## Deferred — Stripe / billing

| # | Title |
|---|-------|
| 425 | Audit existing Provisioning → Cancelled failure paths and migrate to ProvisioningFailed |

> **#426 (RESOLVED 2026-07-25, `8ab75838`):** investigated real behavior — orphan disputes (delivered
> before `checkout.session.completed`) stayed orphaned permanently. Shipped a minimal money-safe
> reconciliation (`relink_orphan_disputes_for_payment_intent`, idempotent, no money-column writes).
> The scoped-out retroactive pause/refund replay filed as **#447**.

> **#447 (RESOLVED 2026-08-08, `36f5e550`):** full replay now ships — `replay_orphan_dispute_lifecycle`
> (renamed from `_pause`) replays BOTH the pause for open orphans AND terminate+refund for closed-lost
> orphans. Both paths are idempotent (terminal-state short-circuit + Stripe dispute idempotency key).
> Money-safe: if normal `handle_dispute_closed` already processed a dispute, it would not be orphaned
> — the replay is the first and only processing. All 10 webhooks_disputes tests pass.

> **#443** (boot-gate asymmetry: no `require_icpay_in_prod`) and **#420** (ICPay automated payouts)
> closed **2026-07-24 — moot**: the ICPay rail was fully retired (Stripe is the sole rail). See
> "Recently closed" below.

## Deferred — UX

| # | Title | Filed by |
|---|-------|----------|
| (F6) | Reputation page is search-only — no browseable leaderboard / "top providers" section (product-design fork) | 2026-08-03 sweep |
| (UX-003) | Seed-phrase-only auth is a wall for non-crypto cloud buyers — needs inline education + OAuth enablement decision (FORK: should Google OAuth be enabled for the primary audience?) | 2026-08-08 UX audit |
| (UX-006) | Reputation page dead-end (search-only, no browseable "Top Providers" leaderboard) — overlaps F6; product-direction mandates a leaderboard; HIGH effort (backend query + UI) | 2026-08-08 UX audit |
| (UX-009) | Dashboard "banner wall" — two stacked full-width colored banners dominate top real estate; consolidate into a compact dismissible notification tray | 2026-08-08 UX audit |
| (UX-014) | Footer community links (discord.gg/decentcloud, twitter.com/decentcloud) unverified — need live verification | 2026-08-08 UX audit |
| (F9) | "Become a Provider" landing CTA lands on the support-account profile page, not a true provider-start (technical onboarding via `dc-agent`/CLI is not reachable from the web) | 2026-08-03 sweep |
| (F2) | Seed/demo offerings carry a fake placeholder pubkey (`example-offering-provider-identifier`) — honestly labelled "Demo only" + excluded from stats, but would be nonsensical in a production deploy (seed-data-quality decision: env-gate demos out of prod, or refresh seed data with real identities) | 2026-08-03 sweep |

> **F9 + F2 (RESOLVED 2026-08-03, this PR `sweep-e2e-ux-techdebt`):** F9 "Become a Provider" now
> routes to real technical onboarding at `/dashboard/provider/start` (`2c393df9`); F2 demo/synthetic
> offerings dropped via migration `053` (`c9dfa9d8`) — the marketplace is now honestly empty pending
> real Hetzner offerings. **F6 (top-providers leaderboard) stays deferred** — premature until real
> offerings exist; see the "Real-deployment smoke audit (2026-08-03)" subsection below.
>
> ⚠️ **2026-08-06 reconciliation:** the migration-053 code drop landed, but the audit found the PROD
> DB still serves 10 demo offerings — migration `053` was never applied to prod (prod never
> redeployed). The code/migration is correct; the live prod DB is not. Tracked as **OP-1** (operator
> blocker) above.

> **#442 (RESOLVED 2026-07-25, `c14cb939`):** create-offering price auto-suggest shipped —
> pre-fill `#monthly-price` with `cost × 1.15` (15% markup, the product decision from comment
> `5078165010`) when Hetzner server cost is known; provider-overridable via a `monthlyPriceTouched`
> flag (never clobbers a typed value). Pure `suggestMonthlyPrice(cost)` helper + `DEFAULT_MARKUP`
> const in `offering-wizard.ts` (10 unit tests); 2 e2e (pre-fill + override-reaches-API). Issue
> **closed**. (Previously listed here as deferred+actionable; the resolution was recorded in the
> 2026-07-25 GH issue sweep but this section was not updated — corrected 2026-08-01.)

> **#441 (RESOLVED 2026-07-25, `b1158bff`):** trial/CTA mismatch fixed — copy now honestly reflects
> the CTA via `shouldShowTrialCopy(plan)` = `trialDays>0 && stripePriceId`; contact-sales-only plans
> (Pro/Enterprise) no longer advertise a trial. Test in `account-subscription.spec.ts` (`@smoke`).
>
> **#436 (RESOLVED 2026-07-25, `3fa993a4` + `ea29b0a3`):** seed-phrase sign-in default fixed via the
> recommended capability-endpoint path. New public `GET /api/v1/auth/capabilities` →
> `{google_oauth: bool}`; the frontend defaults to the credential (seed-phrase) form when OAuth is
> off (no extra click). Server env (`GOOGLE_OAUTH_CLIENT_ID`) is the single source of truth. The
> success-screen auto-redirect bonus was **deferred** (filed as **#445**) — **RESOLVED** later the
> same day (`3b501c62`, see "Recently closed" above).

## Deferred — Tech debt / low-value

| # | Title |
|---|-------|
| 444 | Tech debt: split large source files (>2000 lines) into logical modules |
| 387 | Concurrent multi-ticket processing via multiprocessing + worktrees |
| 334 | Code: Add tests for database modules without dedicated test files |
| (2026-08-03) | `cli/src/keygen.rs` standalone `[[bin]]` duplicates `cli/src/commands/keygen.rs` with diverged behavior, unreferenced — delete OR delegate (re-flagged; parked since 2026-08-01). |

> **`cli/src/keygen.rs` standalone binary (RESOLVED 2026-08-03, `79901166`):** operator locked
> "delete"; the standalone `[[bin]]` is removed (word-count validation preserved in the shared
> `cli/src/commands/keygen.rs`). **`Cargo.lock` is committed to git** (`24e35ace`) — it was already
> tracked; the stale `.gitignore` line is removed.

> **#444 progress (updated 2026-08-03):** 6 providers.rs splits shipped (`PoolsApi` `74fb9248`,
> `NotificationsApi` `b4259194`, `SlaApi` `ae97cd8f`, `AllowlistApi` `290a218f`, `OfferingCsvApi`
> `d94d29af`, `ProviderStatsApi` `b5aa9acb`) + the `api-cli.rs` → dir-bin split (`c7dbf962`) + the
> accounts.rs Wave 9/10/11 TOTP/Recovery/EmailVerification splits (`1729e7c6`/`f041a121`/`24ccacb7`)
> + Wave 12 Stripe dispute split (`e8a6d2b3`, this session — see session log)
> + Wave 13 OfferingStatsApi split (`50b3249f`, this session — 4 per-offering stats handlers).
> providers.rs
> 6739→**4090** (−2649); accounts.rs 2903→**2230**; webhooks.rs 2504→**1277**. Each verified
> byte-identical OpenAPI. Decomposition roadmap at `docs/plans/2026-07-25-large-file-splits-444.md`.
> Current largest **source** files (>2000 lines, `wc -l` 2026-08-03, excluding `target/`/`third_party/`
> and `*_tests.rs`/`tests.rs`): `api/src/openapi/providers.rs` **4090**, `dc-agent/src/main.rs` **3674**,
> `api/src/database/offerings.rs` **2876**, `api/src/database/cloud_resources.rs` **2445**,
> `api/src/openapi/contracts.rs` **2244**, `api/src/openapi/accounts.rs` **2230** (6 files;
> `webhooks.rs` dropped below the 2000-line threshold; `providers.rs` 4280→4090 this session via
> the Wave 13 `OfferingStatsApi` split). **Wave 12 also shipped a permanent
> `api/src/openapi/spec_snapshot.rs` guard** (canonical-JSON SHA-256 of `create_combined_api()` → 187
> paths / 327 schemas) — supersedes the ad-hoc spare-port spec capture; future `*Api` splits'
> byte-identical claim is now a one-line `cargo nextest` check. accounts.rs is **exhausted** for
> mechanical splits (remaining handlers share the `ApiAuthenticatedUser`-gated core). GH #444 stays
> **open** (partial; ongoing).

> **#387 status (verified 2026-08-02):** still **open** — no implementation found.
> `rg "multiprocessing|worktree|concurrent\.futures|ProcessPool"` = 0 hits across dc-agent/api/cli.
> The dc-agent ticket loop is single-threaded async (`poll_and_provision` driven by one tokio
> `interval` in `dc-agent/src/main.rs:1456`); tickets are processed serially per poll tick. No git
> worktrees. Parked — would need a deliberate design (per-ticket worktree + process pool) before work.

> **#334 status (verified 2026-08-02):** largely addressed, kept **open**. Audited
> `api/src/database/*.rs`: nearly every logic module now has in-file `#[cfg(test)]` coverage
> (`acme_dns`, `agent_*`, `api_tokens`, `bandwidth`, `chatwoot`, `cloud_accounts`, `cloud_resources`,
> `handlers`, `notification_config`, `offering_sla`, `offerings`, `recovery`, `refund_audit`,
> `reputation`, `reseller`, `rewards`, `spending_alerts`, `stats`, `telegram_tracking`, `tokens`,
> `totp`, `user_notifications`, `users`, `visibility_allowlist`) or a dedicated subdir `tests.rs`
> (`accounts`, `contracts`, `email`, `offerings`, `stats`, `tokens`, `users`). The only logic module
> with neither is `refund_requests.rs` — but its `process_gated_refund` path is covered cross-module
> by the 9 refund-gate integration tests in `api/src/database/contracts/tests.rs`. Meta files
> (`migration_tests.rs`, `test_helpers.rs`, `tests.rs`, `types.rs`) need no tests. Kept open per the
> literal "without dedicated test files" reading.

> **Closed 2026-08-02 (verified against the actual code; moved out of the open table above):**
> - **#382** `try_trigger_hetzner_provisioning` backward-compat alias — `rg` = 0 matches in
>   dc-agent/api/cli. STALE entry, marked closed.
> - **#373** DRY `extract_contract_id()` shared across 3 provisioners — single shared fn at
>   `dc-agent/src/provisioner/mod.rs:12`, imported by `digitalocean.rs`, `docker.rs`,
>   `proxmox_tests.rs`. STALE entry, marked closed.
> - **#344** additional MOCK tests for the Docker provisioner — `dc-agent/src/provisioner/docker_tests.rs`
>   = 995 lines, 87 mockito-based test fns (image pull, create/inspect/start, verify_setup image
>   found/not-found/custom, network/ipv6 warnings, error paths). Substantially done.
> - **#214** `verify_setup()` check for default_image existence — ships in `docker.rs:638` (compares
>   `config.default_image` against `/images/json` tags), `digitalocean.rs:758` (queries
>   `/v2/images?slug=`), and `proxmox.rs:1138` (template-VM existence, the Proxmox equivalent).
>   3 dedicated docker tests (`test_verify_setup_image_found` / `_not_found` /
>   `_not_found_custom_image`).
> - **#212** pre-built Docker image with openssh-server — `dc-agent/container/` ships `Dockerfile`
>   (ubuntu:22.04 + openssh-server + `PermitRootLogin yes` + sshd ENTRYPOINT), `build.sh`,
>   `publish.sh`; the default image is `ghcr.io/decent-stuff/dc-agent-ssh:latest`
>   (`config.rs::default_docker_image`); tests assert the container CMD no longer runs apt-get.
>   (Note: `container/README.md` header still reads "Ticket 348" — a stale number; the implementation
>   matches #212. README edit out of scope for this docs pass.)
> - **#107** Dark/light mode toggle — `website/src/lib/stores/theme.ts` (dark/light store: system
>   preference, `localStorage` persistence, toggle/set, `matchMedia` live-sync) + `ThemeToggle.svelte`
>   rendered in `routes/dashboard/+layout.svelte` and `DashboardSidebar.svelte` + `theme.test.ts` +
>   extensive `:root[data-theme='light']` rules in `app.css`. Fully shipped.

## Recently closed by this work

### 2026-08-03 session (k8s migration autonomous portion + #444 Wave 12 + real-app UX fixes + robustness sweep)

Two parallel work streams this session. **(1) k8s staging→`dc-stage` consolidation** (separate plan
`docs/plans/2026-08-03-staging-to-k8s-dc-stage-consolidation.md`): the autonomous portion is DONE on
branch `staging-k8s-dc-stage-track3` → **PR #454** — nuc-k3s kustomize base/prod/stage split
(byte-identical prod), `dc-stage` brought LIVE on the cluster (`/api/v1/health` HTTP 200, 52
migrations / 86 tables, prod untouched), product-repo Phase 2/3 (`cf/deploy.py deploy stage` +
`docs/MIGRATION-CUTOVER.md` runbook). Only the **operator cutover** remains (8 items in
`docs/MIGRATION-CUTOVER.md`: push nuc-k3s, persist stage DB pw to SOPS before ArgoCD's first sync,
ship `:stage` image, public tunnel/DNS cutover, tear down dev host). **(2) e2e/UX/tech-debt sweep**
(branch `sweep-e2e-ux-techdebt`, this entry): a no-mock real-app UX audit → 6 high-confidence fixes
shipped; one #444 split; a rust robustness sweep. Baseline: `origin/main 31483130`. All gates green
(smoke 26/26, clippy 0, vitest 869, svelte-check 0/0).

| Fix | Area | Resolution |
|-----|------|------------|
| #444 Wave 12 — Stripe dispute split | Tech debt | `e8a6d2b3`: extracted the `charge.dispute.*` cluster from `api/src/openapi/webhooks.rs` → new `webhooks_disputes.rs` (4 handlers, 2 types, 5 helpers, 10 DB-coupled e2e tests). webhooks.rs **2504→1277** (−1227). "Path B" split (webhooks has no `#[OpenApi]` impl — handlers are raw `Route::at` — so `create_combined_api` is untouched, no tuple slot consumed). Byte-identical OpenAPI via the **new permanent `spec_snapshot.rs` guard** (canonical-JSON SHA-256, 187 paths / 327 schemas) which supersedes the ad-hoc spare-port capture for all future splits. clippy 0, nextest 30/30. |
| UX F5 — blank "Trust Score /100 Reliable" card | UX (trust signal) | `09fa538e`: the trust-score calculator started at 100 + only deducted for *observed* negatives, so a brand-new provider (0 contracts) scored ≈90 → green "Reliable" — a direct contradiction of the product's trust promise. New `hasEnoughTrustData(metrics)` helper (`total_contracts > 0`) gates the verdict; absent track record renders **N/A · "Not enough data"** (neutral). TDD helper test; vitest 865; dashboard-overview smoke green. |
| UX F3 — email-verification "surprise wall" in rent flow | UX (flow) | `09321415`: rentals were hard-rejected at contract-create for unverified email with NO upstream warning. Surfaced the prerequisite at every entry: offering-detail "Rent this offering" → relabels to "Verify email to rent" + routes to account; rentals empty-state 3-step guide → "Verify your email first" notice; shared `RentalRequestDialog` (the choke point) → "Email verification required" notice + locked Submit + fail-fast guard. New serial spec `rent-email-verification-gate.spec.ts` (3, RED→GREEN). |
| UX F1 — platform stats counted draft offerings | UX / backend | `3b6d494e`: `get_platform_stats` `total_offerings` counted public offerings WITHOUT filtering `is_draft`, so an admin's in-progress draft showed as an "Available Offering", inconsistent with the marketplace. Added `AND is_draft = FALSE` + regenerated the `.sqlx` plan. (The audit's "0/0 vs marketplace" symptom was the *intentional* demo-provider exclusion — consistent + already copy-gated by the earlier U2 work.) TDD stats test; verified live (total_offerings 4→3 matching the marketplace). |
| UX F4 — verify-email banner on every page | UX (chrome noise) | `26c65b05`: the full-width verify-email + seed-backup banners rendered on every `/dashboard/*` sub-route. Confined both to `/dashboard` + `/dashboard/account*` (where the action lives); not suppressed entirely for genuinely unverified users. dashboard-banners e2e 4/4. |
| UX F7 — raw `@handle` salad as provider identity | UX / backend | `bc9caf05`: offerings API returned only the auto-generated `owner_username`; the marketplace/detail/compare rendered `@uxprovidercggf6l`-style handles instead of the provider name collected at onboarding. Backend: added `provider_name` (via `provider_profiles.name` correlated subquery) to the `Offering` struct + list/search/single queries. Frontend: new `providerDisplayName()` helper (provider_name → @handle → truncated pubkey) across 4 pages. TDD helper test (4); offerings + cloud + visibility tests green. |
| UX F8 — duplicate create-account entry on /login | UX (flow) | `92baf88a`: `/login` showed both the "Generate New" card AND a "New here? Create an account" link for the same flow. Suppresses the redundant link when the chooser is visible (keeps it for the OAuth-on case). Updated `login-registration-cta.spec.ts` OAuth-aware. |
| Robustness R1 — MailChannels client had no timeout | Robustness | `482047d2`: `EmailService::new` built a bare `reqwest::Client::new()` (the lone workspace outlier) — a stuck MailChannels endpoint could hang the background email-queue processor indefinitely. Added `MAILCHANNELS_TIMEOUT_SECS=30` const + `.timeout(...)`. Also deleted 139 lines of orphaned dead test code in the extracted email-utils subcrate. clippy 0, nextest 1/1. |
| Robustness R2 — dead WasmLedgerEntry display path (16 unwraps) | Dead code / panics | `5a045278`: `WasmLedgerEntry` + 9 `from_*` constructors + `ledger_block_parse_entries` in `common/src/ledger_refresh.rs` had ZERO callers anywhere (verified across api/cli/dc-agent/ic-canister) and carried 16 `.unwrap()` panic-on-bad-borsh sites. Removed (−158 lines); the live replay path already deserializes correctly with `?`+`error!`. nextest 138/138. |
| Robustness R3 — duplicated SSH-pubkey regex + panic-on-init | DRY / panics | `deaae832`: the SSH-pubkey regex was duplicated in 2 handler sites, recompiled per request, each with `.unwrap()`/`.expect()` panic-on-init. New single-source `is_valid_ssh_pubkey_format()` + `SSH_PUBKEY_REGEX` OnceLock in `validation.rs` (alongside URL/USERNAME). TDD (4 key types + 4 rejections). validation 12/12, contracts 28/28. |
| Robustness R4 — silent numeric env-parse fallback | Debuggable errors | `958af5ac`: 11 startup env-var parses used the silent `.ok().and_then(parse).unwrap_or(default)` pattern — a typo like `EMAIL_BATCH_SIZE=1o0` silently became the default. Renamed `parse_env_seconds`→`parse_env_u64`, extracted pure `parse_positive_u64`, converted all 11 sites to fail-fast (matches issues #409/#410). TDD (valid/malformed/zero/negative/overflow/empty). main_tests 6/6. |
| Robustness R5 — dc-agent `Duration::from_secs(30)` duplicated 7× | DRY | `f471fa3d`: the 30s HTTP-client timeout was hardcoded across 7 dc-agent sites (api + cli already single-source it). New `pub const HTTP_TIMEOUT_SECS` in `dc-agent/src/lib.rs`; all 7 sites reference it. nextest 246/246 (incl. `build_verify_client_enforces_request_timeout`). |
| Tech-debt — `.sqlx` cache split footgun | Build integrity | New `scripts/sqlx-prepare.sh` self-locating wrapper (always runs `cargo make sqlx-prepare` = `cargo sqlx prepare --workspace` from repo root, so the committed workspace-ROOT `.sqlx/` is the single write+read source) + `api/src/sqlx_cache_check.rs` guard test (`no_per_package_sqlx_cache_dir`) that goes RED the instant the gitignored `api/.sqlx/` appears (the stray bare-`prepare` signature) — runs in `cargo nextest run -p api` so CI blocks drift. Deliberately does NOT re-assert root cache presence (already covered, non-overlapping, by `migration_tests::test_sqlx_offline_mode_data_exists`). TDD-proven (RED on injected `api/.sqlx/`, GREEN clean). Docs: repo `AGENTS.md` (new sqlx subsection), `api/AGENTS.md`, `scripts/AGENTS.md`, `docs/ci-cd.md` (fixed the wrong bare-`cargo sqlx prepare` instruction → `scripts/sqlx-prepare.sh`). clippy 0, nextest green. |
| Test guard — stale OpenAPI spec_snapshot hash | Test integrity | `7484d3d3`: the byte-identical guard `openapi_spec_is_stable` was RED on the branch (hash `de652956…` vs committed `4549fcf2…`, identical 187/327 counts). Root cause: UX F7 (`bc9caf05`) intentionally added `provider_name` to the `Offering` schema but did not refresh the snapshot hash. Verified the current spec contains `Offering.properties.provider_name` and the only spec-changing commit since the post-Wave-12 capture is F7 (`deaae832` regex DRY is spec-neutral); hash deterministic across runs. Refreshed `EXPECTED_HASH` to `de652956…` (test-artifact maintenance, not a fork) — unblocks the api test gate and restores the guard that all #444 `*Api` splits verify against. |
| Tech-debt — dev-server.sh release-binary discoverability | DX / e2e | `84aa1f8a`: `scripts/dev-server.sh` served the RELEASE binary by default (deliberate for e2e timing fidelity) but the choice was only in a header comment — a sibling agent rebuilt the DEBUG binary thinking it would affect the running RELEASE server. New `_announce_api_binary()` prints the exact binary path + RELEASE/DEBUG classification + an actionable rebuild hint (release: rebuild + restart — a running server does NOT hot-swap; points at the debug override for fast iteration) at both local-API start sites. |
| #444 Wave 13 — OfferingStatsApi split | Tech debt | `50b3249f`: extracted the 4 per-offering statistics handlers (contract stats / weekly history / conversion / tenant satisfaction) from `ProvidersApi` in `providers.rs` → new `OfferingStatsApi` in `api/src/openapi/offering_stats.rs`. Cleanest cohesive cluster (read-only analytics, DB-layer return types, no local types/helpers to move). providers.rs **4281→4090** (−191). Byte-identical via `spec_snapshot` (hash `de652956…` unchanged, 187/327). clippy 0; offering-stats serialization + spec_snapshot 6/6. |

**Sweep methodology:** read-only no-mock UX audit (WAVE-B) drove the real warm stack via chrome-cli
screenshots + zai-vision + Plasmate, new-user + returning-user lens, reported findings (no commits);
separate implementers shipped the ≥6/10 fixes (WAVE-E) + #444 split (WAVE-D) + rust robustness
(WAVE-C), sequenced to avoid git-index races on the shared branch. Working tree clean; 14 commits on
`sweep-e2e-ux-techdebt` ahead of `origin/main`.

**Findings NOT shipped (environmental / low-confidence / needs design — tracked above + below):**
- **Smoke wall-clock (Finding B):** smoke measured 48–80s vs the <35s target, but the system load
  average was **42–45** throughout (concurrent cargo builds + shared environment contention). The
  harness already has warm-stack / fast / smoke / shard infra; individual test times are normal. This
  is **environmental, not a code regression** — re-measure on an idle system before chasing.
- **`invoices @smoke empty state` flake (Finding A):** failed under full parallel smoke (timing
  timeout) but passes in isolation — same parallel-load-contention root cause as Finding B.
- **`.sqlx` cache split footgun:** the repo has a tracked root `.sqlx/` (workspace cache) AND a
  gitignored `api/.sqlx/`; `cargo sqlx prepare` run from `api/` writes to the gitignored copy, so a
  checked `query!`/`query_scalar!` edit + prepare-from-`api/` does NOT update the committed root
  cache → fresh CI clones fail with "no cached data". Correct incantation: `cargo sqlx prepare
  --workspace` (or manually copy the new plan into root `.sqlx`). **Recommend:** document the
  prepare procedure + add a CI check that root `.sqlx` is complete. (Tracked below in tech-debt.)
  **RESOLVED this session:** `scripts/sqlx-prepare.sh` self-locating wrapper (always uses
  `--workspace`) + the `sqlx_cache_check::sqlx_offline_cache_has_single_committed_source` test that
  fails loudly (locally + CI) the instant `api/.sqlx/` appears + AGENTS.md/ci-cd.md docs (incl. fixing
  the wrong bare-`prepare` instruction).
- **`dev-server.sh` runs the RELEASE binary by default** (deliberate: debug Ed25519 is ~150x slower,
  distorting e2e timing). The `API_BINARY=.../debug/api-server` override exists for fast Rust
  iteration — but the comment is easy to miss (cost a verification cycle). **Minor:** add a louder
  startup log line. (Tracked below in tech-debt.) **RESOLVED this session:** `_announce_api_binary()`
  prints the exact binary path + RELEASE/DEBUG classification + a rebuild hint (release: rebuild +
  restart; notes a running server does NOT hot-swap; points at the debug override for fast iteration)
  at both local-API start sites.
- **`cli/src/keygen.rs` standalone binary** (re-flagged by WAVE-C): duplicates `cli/src/commands/keygen.rs`
  with diverged behavior, unreferenced by any script/CI/Dockerfile — parked since 2026-08-01 (binary
  surface change, not a live bug). Needs a human call: delete OR delegate.

### Real-deployment smoke audit (2026-08-03)

Read-only smoke audit against the live deployments (prod `dc-prod`, stage `dc-stage`,
public-dev). **6 issues found; 5 fixed in this PR (#456), 1 resolved by the operator
k8s cutover, and the prod-marketplace-emptiness finding closes via the strategic pivot
(drop demos — done) + the Hetzner first-offerings milestone (forward).** The
secret/config gap across both envs is **exactly the 3 TextBee SMS keys**
(`TEXTBEE_DEVICE_ID` / `TEXTBEE_API_KEY` / `TEXTBEE_API_URL`)
— empty in BOTH prod + stage; every other key is present + non-empty.

| # | Finding (severity) | Resolution | Commit / Plan |
|---|--------------------|------------|---------------|
| 1 | Prod rate-limiting silently OFF — `ENVIRONMENT=prod` vs code checked `=="production"` (P0) | **Fixed this PR** — new `api/src/environment.rs::is_production(env)` predicate | `8ed10bb9` |
| 2 | Prod marketplace EMPTY — 0 offerings / 0 contracts, only a synthetic seed (P0) | **Not a code bug.** Strategic pivot: demo offerings dropped (migration `053`) + add REAL Hetzner offerings (forward milestone) | `c9dfa9d8` (drop demos) + `docs/plans/2026-08-03-hetzner-first-offerings.md` (add real) |
| 3 | public-dev stale image — `/auth/capabilities` 404, retired ICP currency, route drift (P1) | **Operator k8s cutover** — push nuc-k3s + repoint the tunnel at `dc-stage` | `docs/MIGRATION-CUTOVER.md` (Step D) |
| 4 | SMS subsystem silently unconfigured — TextBee keys empty in prod + stage (P1) | **Fixed this PR** (boot warning) + **operator must populate the TextBee keys** | `3211deeb` |
| 5 | Google OAuth callback hard-400 on consent-denied (`?error=`) (P2) | **Fixed this PR** — redirect instead of 400 | `dec74bf5` |
| 6 | `dc-api-sync` logged a misleading "Cloudflare DNS not configured" warning (P2) | **Fixed this PR** | `01b1d618` |

**Closed this PR (smoke audit + operator-locked decisions):**
- `cli/src/keygen.rs` standalone `[[bin]]` deleted (`79901166`) — operator locked "delete".
- `Cargo.lock` committed to git (`24e35ace`) — was already tracked; stale `.gitignore` line removed.
- Demo/synthetic offerings dropped — migration `053` (`c9dfa9d8`); marketplace now honestly empty.
- F9 "Become a Provider" → real technical onboarding at `/dashboard/provider/start` (`2c393df9`).
- The 5 smoke-audit code fixes above (rate-limit / drop-demos / SMS / OAuth / dc-api-sync).

**Deferred:**
- **F6 — top-providers leaderboard.** Premature: there are no real providers to rank
  until the Hetzner first-offerings milestone lands real offerings; ranking
  demo/synthetic providers would mislead (violates the honest-catalog direction in
  `docs/PRODUCT-DIRECTION.md`). Natural follow-up **after** real offerings exist.

**Forward milestone:** `docs/plans/2026-08-03-hetzner-first-offerings.md` scopes the
operator reselling Hetzner as the platform's first real provider (ends the
honest-empty-marketplace period). Status: **PROVEN + UNBLOCKED (2026-08-06).** The
data path was proven end-to-end by a no-spend PoC (`poc/hetzner-cloud-resell-poc.mjs`,
commit `4c6eb3b7`): fresh identity → provider onboarding → live-validated Hetzner
cloud_account → real cloud-resell offering created (cx23/nbg1/ubuntu-24.04) →
**marketplace VISIBLE with Rent button ENABLED** (empirically confirms the `a2a96862`
cloud-resell visibility fix). All creds (`HETZNER_API_TOKEN{_DEV/_STAGE/_PROD}`,
`DC_PROD_RESELLER_PUBKEY/_SEED`) are already in the age-SOPS store — see
`docs/CREDENTIALS.md`. A latent bug was also fixed: the retired `cx22` Hetzner server
type default → `cx23`. **Remaining steps (need operator opt-in):** (1) real-VM
provisioning verification (cheapest cx23, ~€0.0007 for 5 min, force-delete) — gated
behind `DC_E2E_INCLUDE_PROVISION` or api-cli rent→provision→SSH→cancel; (2) seed the
offering to STAGE then PROD (data only — reuse the PoC pointed at the reseller seed,
ship the api image with the cx23 fix via GitOps). Full runbook in the PoC commit header.

### 2026-08-02 session (WAVE-0: prior-session WIP + stale-issue reconciliation)

Reconciled the `docs/OPEN_ISSUES.md` "Deferred — Tech debt / low-value" table against the actual
code (a code-verification pass — no behavior changes), and recorded the 3 prior-session WIP commits
that had landed but were not yet logged.

**Prior-session WIP shipped (3 commits):**

| Commit | Area | Detail |
|--------|------|--------|
| `f186c0d9` | api / chatwoot | **fix(api): chatwoot create_portal must not claim a shared custom_domain.** `create_portal` was sending the shared frontend host as `custom_domain` for every provider — Chatwoot's `custom_domain` is globally unique, so only the FIRST provider could onboard a Help Center; every later one 422'd "Custom domain has already been taken". Fix: send `custom_domain=""` (empty string dodges `URI.parse(nil)` TypeError; a `before_validation` hook normalizes `""→nil` so it passes `allow_nil` uniqueness). TDD regression test added. |
| `e5a1f08e` | docs | **AGENTS.md canonical-source note** — records GitHub Issues as the canonical live source and the in-repo inventory as a categorized snapshot, with the reconcile-before-acting rule. |
| `897a90e5` | ops / secrets | **chore(secrets): re-encrypt common.yaml** — sops 3.9.4 → 3.11.0 (tooling bump; no secret-value changes). |

**Stale-issue reconciliation (code-verified, docs-only):** audited every row of the
"Deferred — Tech debt / low-value" table with `rg` / `find` / `wc -l` against the working tree.
Confirmed **6 stale entries already done in code** and moved them out of the open table: **#382** and
**#373** (backward-compat alias + DRY refactor — 0 matches / single shared fn), **#344** (Docker MOCK
tests — 995-line `docker_tests.rs`, 87 fns), **#214** (`verify_setup` default_image check — ships in
all 3 provisioners + 3 tests), **#212** (pre-built openssh image — `dc-agent/container/` + default
image), **#107** (dark/light toggle — `theme.ts` + `ThemeToggle.svelte` + tests). Kept open with
current evidence: **#444** (partial; progress note refreshed with the real largest-file counts),
**#387** (no implementation found; single-threaded poll loop), **#334** (largely addressed inline;
kept open on the literal "dedicated test files" reading). See the table notes above for per-issue
evidence.

Gates: docs-only — no code touched. `rg -n "try_trigger_hetzner_provisioning|#373|#382" docs/OPEN_ISSUES.md`
records #382/#373 as closed.

### 2026-08-02 session (e2e harness radicalization + UX slop fix + #444 Wave 9/10 + auth single-source)

Continuation of the radical-overhaul mandate (harness + UX + tech debt + robustness) against a
verified-green baseline. 17 commits (`9657dee8`→`749cf876`), TDD-first where applicable, verified
against the real warm stack (api:59011 + web:59010), no first-party mocks. Final gates: smoke
**26/26 in ~28s** (<30s target), clippy **0**, vitest **862**, svelte-check **0/0**.

| Fix | Area | Resolution |
|-----|------|------------|
| Smoke speed: 39.6s → ~28s | E2E harness | `9657dee8`: the `testAccount` authed page fixture did a wasteful double-navigation (logged in, then re-navigated to the same page). Dropped the redundant navigation → smoke **39.6s → ~28s**, zero coverage loss; all 26 smokes green + reliable. |
| Coverage gap: `/dashboard/reputation/[identifier]/trust` | E2E coverage | `9e437e45`: new `reputation-trust.spec.ts` — the reputation trust-report route was an undocumented coverage gap; now driven against the warm stack. |
| No-mock invariant documented | E2E discipline | `41ee69b8`: FLOWS.md now records the 2 first-party fetch mocks as **sanctioned exceptions** (a Mock inventory added) — both are outbound-HTTP-boundary stubs, not first-party-logic mocks. The no-mock invariant holds. |
| Stale smoke-table titles + count drift | Docs | `445a17d4` + `3178799d`: fixed stale smoke-table titles in FLOWS.md; corrected smoke-count drift (27→26 after the SaaS-removal session dropped the subscription spec). |
| Stale-issue reconciliation | Docs | `e775492d` (+ the WAVE-0 pass): #382, #373, #344, #214, #212, #107 all verified **CLOSED** against code evidence; the open tech-debt table went **8→3 rows** (#444, #387, #334 remain). Per-issue evidence is in the WAVE-0 entry above. |
| #444 Wave 9 — `TotpApi` split | Tech debt | `1729e7c6` + `8c6dd37c`: extracted `TotpApi` from `api/src/openapi/accounts.rs` (**2903→2594 lines**). Byte-identical OpenAPI verified via spare-port instance diff (**187 paths / 327 schemas**, empty canonical diff). clippy 0, nextest **44/44**. |
| #444 Wave 10 — `RecoveryApi` split | Tech debt | `f041a121` + `d9e51a58`: extracted `RecoveryApi` from `accounts.rs` (**2594→2442 lines**). Byte-identical OpenAPI; clippy 0, nextest **39/39**. Next candidate: the email-verification cluster (8/10 readiness). |
| #444 Wave 11 — `EmailVerificationApi` split | Tech debt | `24ccacb7` + `5e4c38b9`: extracted `EmailVerificationApi` from `accounts.rs` (**2442→2230 lines**). Byte-identical OpenAPI (187 paths / 327 schemas, empty canonical diff); clippy 0, nextest **37/37**. **accounts.rs is now exhausted for mechanical splits** — the three clean clusters (TOTP/recovery/email-verification, all `#[OpenApi]` handler groups) are done; remaining handlers are interwoven with the `ApiAuthenticatedUser`-gated core and need a focused design pass, not the wave cadence. |
| UX U1: hero trust card was fake data | UX (slop) | `50fb8a15`: (no-mock UX audit) the landing hero "trust card" was a **hardcoded fake** (`provider_alpha` with a deceptive "Updated 2m ago" liveness stamp). Now honestly labeled **"Illustrative example"**; the fake liveness text removed. No misleading data on the landing page. |
| UX U2: all-zero "Marketplace Statistics" | UX (empty state) | `d719df71`: "Marketplace Statistics" rendered all-zeros unconditionally — dishonest on a fresh marketplace. New pure `marketplaceIsEmpty(stats)` helper (4 unit tests, TDD RED→GREEN) gates an honest **"Be Among the First Providers"** early-access reframe instead of showing 0/0/0. vitest 862; zai-vision-verified on the real app. |
| Auth single-source-of-truth fully enforced | Robustness / DRY | `d34e11fb` + `749cf876`: (code-robustness audit R1/R2) dc-agent `api_client.rs` hand-rolled the signed-message layout + header-name literals → now delegates to the canonical `dcc_common::api_auth::{sign_request, HEADER_*}`; api-cli header literals → `HEADER_*` consts. Wire format proven **byte-identical** field-by-field (timestamp unit, nonce, header names, message byte-layout); the unchanged dc-agent auth-guard test stayed green. No outlier remains — the "Signed-request auth — single source" convention is now fully enforced. |

**Net-new findings (documented / tracked):**
- **Code-robustness audit:** most categories CLEAN (timeouts, hex, stale refs, dead code, DB
  defaults, money-path/refund-gate, unwrap/expect, `api.ts`, `danger_accept_invalid_certs`). One
  finding **shipped** (R1/R2 auth single-source, above). One finding **shipped (R3):**
  `StripeClient::new().ok()` silently swallowed Stripe misconfig at 6 sites
  (`admin.rs`/`providers.rs`/`webhooks.rs`/`contracts.rs`/`main.rs`×2). It was money-safe (returned
  `None` → handlers return `Ok(None)` = "refund not performed"), but **invisible** — no warning was
  logged. **Fixed in `b7016c40`** via a DRY `stripe_client_or_warn()` helper (next to `StripeClient`)
  that emits an actionable `tracing::warn!` (names `STRIPE_SECRET_KEY`, lists what is skipped,
  includes the error chain) before returning the same `None` — zero money-behavior change, all
  refund-path tests green. `rg "StripeClient::new().ok()"` now 0.
- **UX audit Low findings (NOT shipped — below/over threshold):** **U3** validators-zeros (an
  environment artifact, not a bug); **U4** provider-gate button hierarchy (confidence 5, below the
  6/10 ship threshold — skipped); **U5** "Welcome back" greeting for first-time users (confidence 6,
  needs a first-visit detection state — parked).

### 2026-08-02 session (drop unused SaaS account-subscription feature)

Removed the unused SaaS account-subscription feature (Free/Pro/Enterprise pricing plans for using
Decent Cloud) FULLY across frontend + backend + DB. This was Feature A; it was confirmed unused —
`account_has_feature` + `count_active_contracts_for_account` were both `#[allow(dead_code)]`, so
runtime feature-gating was never enforced (free plan = unlimited rentals). The DISTINCT per-contract
recurring billing (Feature B: `contract_sign_requests.stripe_subscription_id` /
`.subscription_status` / `.current_period_end_ns` / `.cancel_at_period_end`,
`provider_offerings.is_subscription` / `.subscription_interval_days`, the
`get_subscription_item_id` + `create_usage_record` metered-billing code path in
`cleanup_service.rs`, and the `invoice.paid` / `charge.dispute.*` webhook arms) is PRESERVED
untouched.

| Change | Area | Detail |
|--------|------|--------|
| Backend removal | api crate | Deleted `openapi/subscriptions.rs` (SubscriptionsApi, 5 endpoints) + `database/subscriptions.rs` (SubscriptionPlan/AccountSubscription/SubscriptionEvent + all fns/tests, 1106 LOC total). Unwired from router tuple, ApiTags enum, rate-limiter checkout path + test, and `database/mod.rs` re-exports. Removed `customer.subscription.{created,updated,deleted}` webhook arms + their now-orphaned structs (`StripeSubscription`/`Items`/`Item`/`Price`) + the 3 event registrations in `main.rs`. Trimmed `invoice.payment_failed` to parse + `tracing::warn!` only (dropped the SaaS-specific `subscription_id` inner block). Removed subscription-only `stripe_client.rs` methods (`create_subscription_checkout`, `get_subscription`, `cancel_subscription`, `create_portal_session`, `get_or_create_customer` + `SubscriptionInfo`). KEPT Feature-B `get_subscription_item_id`/`create_usage_record` (used by `cleanup_service.rs`). |
| DB schema | migration 052 | `api/migrations_pg/052_drop_account_subscription_feature.sql`: drops `subscription_events`, `subscription_plans`, 3 accounts indexes, 6 accounts columns (`subscription_*`, `stripe_customer_id`). `contract_sign_requests.*` columns NOT dropped (Feature B). |
| Frontend removal | website | Deleted `routes/dashboard/account/subscription/` (+page.svelte 326L + contact-sales.test.ts), `lib/utils/subscription-plans.{ts,test.ts}`, `tests/e2e/account-subscription.spec.ts`. Removed Subscription tab from `SettingsTabs` (+ test), the subscription card from `account/+page.svelte`, the Subscription API section from `api.ts` (2 interfaces + 5 fns, ~177 LOC). KEPT Contract-type subscription fields (`api.ts` L1361-1365 — Feature B). Updated `route-audit.spec.ts` + `seed-helpers.ts`. |
| Web e2e docs | FLOWS.md | Removed subscription coverage rows + `@account` tag entry; smoke count 27→26; renumbered smoke table. |

Gates: `cargo build -p api --bin api-server` clean; `cargo clippy -p api --tests --all-targets` 0 warnings; `cargo nextest run -p api` green on all touched modules (subscriptions → 0 tests, accounts 122/122, webhooks/rate_limit/stripe_client/cleanup_service 67/67); `npm run check` 0/0; `npx vitest run` 858/64 files; `npm run test:e2e:fast:smoke` 26/26 in 36.7s.

### 2026-08-01 session (clippy cleanup + e2e gap verification + CLI harness + UX root-cause)

Continuation of the radical-harness/UX/tech-debt mandate against the verified real baseline. All
work TDD-first where applicable, verified against the real warm stack (api:59011 + web:59010), no
mocks in first-party paths. **No commits made** — changes are staged in the working tree pending a
user review/commit decision. Final gates: `cargo clippy --workspace --tests --all-targets` → **0
warnings**; cargo lib tests **1469/0**; `npm run test:e2e:fast:smoke` 27/27; rent-flow **4/4**;
`cargo nextest run -p decent-cloud` **63/6**.

| Fix | Area | Resolution |
|-----|------|------------|
| Clippy: 30 → 0 warnings (DRIFT fix) | Tech debt | 10 edits across api + dc-agent. dc-agent `digitalocean.rs`: file-top `#![allow(dead_code)]` (DO API response structs deserialize full shape for fidelity + `digitalocean_tests.rs` assertions — fields read in tests, cannot drop); removed truly-dead `DoErrorResponse` (0 refs incl. tests); `proxmox.rs:729` `while_let_loop` rewrite (`while let Ok((stream,_)) = listener.accept()`). api: `dispute.rs:694` `#[allow(dead_code)]` on test-only helper; removed 2 unused `now_ns` blocks in `tests.rs`; `#[allow(clippy::too_many_arguments)]` on 3 column-binding fns; `#[allow(clippy::type_complexity)]` on money-path `query_as`; `#[allow(dead_code)]` on `RefundGateOutcome::PendingApproval.user_latest_payment_e9s` (money-path audit data, already logged at the gate site). Changed-crate tests: dc-agent 246/246, api stripe_client 18/18, api refund_gate 8/8. |
| `#442` doc drift reconciliation | Docs | OPEN_ISSUES.md listed `#442` BOTH as "Deferred — UX" AND as RESOLVED (`c14cb939`). Reconciled: the Deferred table now reads `_none currently open_`; the deferred note now records the resolution (corrected 2026-08-01); historical session tails struck-through with CLOSED annotation. |
| rent→pay→view→cancel e2e gap — confirmed CLOSED | Coverage | `rent-flow.spec.ts` (4 serial tests, 238L) already drives the real marketplace Rent dialog → signed POST /contracts → rentals list → detail page → signed PUT cancel against the warm stack. Re-ran: **4/4 in 24.8s**. Contract commits at `requested` (cancellable) before Stripe checkout, so drivable without STRIPE_SECRET_KEY. FLOWS.md + OPEN_ISSUES tech-debt rows updated to CLOSED. |
| CLI harness coverage audit + 4 tests + error fix | Coverage / robustness | `cli/tests/cli_flows.rs` +141 lines: pool commands identity guard, register/check-in ghost-identity offline short-circuit, malformed `--amount-dct`/`--amount-e9s` parse rejection, pool-generate missing-pricing-file error. `cli/src/commands/account.rs` amount-parse errors upgraded from bare `ParseFloatError`/`ParseIntError` to detailed `Invalid --amount-dct '{value}': {e}. Pass a decimal number of DC tokens (e.g., --amount-dct 1.5).` (was violating the "provide failure details" rule). `cargo nextest run -p decent-cloud` → **63/6** (was 59/6). |
| JS error "environment variable not found" — root-caused + fixed | UX / debuggable errors | **Root cause:** `api/src/stripe_client.rs:35` `std::env::var("STRIPE_SECRET_KEY")?` propagated `VarError::NotPresent` whose `.to_string()` is the stdlib string "environment variable not found" — bubbling through `create_stripe_checkout_session` → contracts.rs handler → frontend `createRentalRequest` → `RentalRequestDialog.svelte:268` catch. The contract IS created at `requested` before Stripe is called, so the bare error misled (rental succeeded, payment-init failed). Fix: `stripe_client.rs` `.context("STRIPE_SECRET_KEY is not set — Stripe payment processing is unavailable")`; contracts.rs handler now returns `"Rental created but payment could not be initiated: {e}. You can retry payment or cancel from your rentals page."` + `tracing::warn!` server log. 18/18 stripe tests, rent-flow 4/4, live-repro verified against release-mode api-server. |
| Live UX audit (no mocks) | UX | Drove the real app via Playwright Chromium + chrome-cli against the warm stack. Homepage + marketplace: **0 console errors**. Warm-stack API config confirmed correct (`dev-server.sh:280` injects `VITE_DECENT_CLOUD_API_URL` as process env, highest priority over `.env.local`). The single console error surfaced (above) was root-caused to a backend error-message gap, not a frontend bug. |

**Net-new finding (documented, NOT autonomously resolved — below the threshold per AGENTS.md
"conflicting business-logic implementations"):**
- **`cli/src/keygen.rs` standalone binary duplicates `cli/src/commands/keygen.rs` with DIVERGED
  behavior.** The standalone `[[bin]] name="keygen"` is unreferenced by any script/CI/docs/dockerfile
  and looks like a leftover dev/demo tool; it has its own `ALL_LANGUAGES`, `detect_mnemonic`,
  `mnemonic_from_strings` (validates word count 12/15/18/21/24 — the `dc keygen` command does not,
  relying on `bip39::Mnemonic::from_phrase` to reject bad counts). It carries genuine
  sign/verify/mnemonic/seed unit tests. **Recommendation:** delete the standalone binary OR make it
  delegate to the shared `commands/keygen.rs` functions. Decision parked (binary surface change + the
  divergence is not a live bug). Filed here as a tracked finding.

### 2026-07-26 session (refund approval gate + e2e harness expansion + UX audit)

Refund approval gate (user-requested cost-safe billing policy) fully shipped,
e2e harness expanded for both CLI and web, OpenAPI tuple rebalanced to unblock
future #444 splits, and a full live UX audit found no product issues. All work
TDD-first, verified against the real warm stack.

| Fix | Area | Resolution |
|-----|------|------------|
| Refund approval gate — full feature | Backend + admin UI + e2e | **Plan**: `docs/plans/2026-07-26-refund-approval-gate.md` (`6c22263a`). Policy: auto-refund when `refund_e9s ≤ user's latest Stripe payment`; hold for admin approval otherwise; Telegram on every event; unbypassable DB trigger. **Migration 051** (`335386f2`): `refund_requests` table + `enforce_refund_approval_gate` trigger (blocks `payment_status='refunded'` / `stripe_refund_id` first-set without matching `refund_requests` row with `status IN ('auto_issued','approved')`). **DB layer**: `process_gated_refund` replaces direct `issue_audited_refund` calls in ALL 4 refund paths (cancel/reject/dispute_lost/provisioning_failed). **Admin API** (`f7b75b9f`): `GET/POST /admin/refund-requests` (list/approve/decline). **Admin UI** (`b4f1ba3d`): refund-requests section in `/dashboard/admin` with status filter, cap-exceeded badge, inline review panel. **DB gate tests** (`8ec052ad`): 9 integration tests (auto-issue, cap-exceeded hold, admin approve/decline, trigger blocks bypass × 3). **E2E** (`217eee8c`): 3 admin panel tests (API listing, UI decline end-to-end, status filter). |
| CLI e2e harness expansion (#444) | Coverage | `1331273f`/`8699f7cc`/`413d2a28`: 18 new tests (13 offline flows + 3 smoke + 4 IC-mainnet `#[ignore]` + 1 hardened). Default tier 41→59 @0.58s; IC tier 2→6. Found + fixed production bug: `account --transfer-to <bad>` panicked via `IcrcCompatibleAccount::from().expect()` → validates principal at call site. |
| Web e2e: confirmInlineAction helper (#444 audit #11) | DRY | `4f5e4906`: extracted `confirmInlineAction(page, row, {arm, confirm?, secondary?, waitForResponse?})` in `auth-helpers.ts`; applied to 7 inline-delete entities + rentals cancel. ~50 LOC boilerplate collapsed. Audit items #4/#5/#7 verified already shipped. |
| Baseline: auth-capabilities stale OAuth | Test | `d33ff5bc`: 2 `@smoke` tests hardcoded `google_oauth=false` but warm stack now has OAuth on. Rewrote spec env-agnostic: reads real `/api/v1/auth/capabilities`. Smoke 27/27 green. |
| #444: OpenAPI tuple rebalance | Tech debt | `87a48059`: rebalanced `create_combined_api()` from `(9-tuple, 16-tuple)` → `(13-tuple, 12-tuple)` by moving `PoolsApi`/`NotificationsApi`/`SlaApi`/`AllowlistApi` to tuple 1. Verified via clean-room spec diff (empty — 192 paths, 337 schemas both sides). Unblocks future handler splits (accounts.rs recovery/TOTP, offerings.rs recommendations) — tuple 2 now has 4 free slots. |
| Decent-Agents cluster re-verify | Status | **#413 (per-subscription agent identity) CLOSED** — was the key blocker. `anthropic-proxy` crate fully functional (1680 lines, 33/33 tests pass): key injection/stripping, per-identity metering, redaction, loud failure on errors. Remaining issues (#418 beta onboarding, #415/#416 billing/metering, #429-#432 deferred) are product/business decisions, not code-blocked. |
| Live UX audit (no mocks) | UX | `9995fafd`: audited 8 pages (landing, marketplace, login, dashboard, my-rentals, account settings, admin panel, mobile marketplace) against the real warm stack via `browser.js` + zai-vision. **0 product UX issues found.** Fixed `browser.js` `authenticatePage()` — was not setting `first_login_onboarding_completed` in localStorage, so WelcomeModal blocked authed-page screenshots. |
| FLOWS.md gap assessment | Coverage | Wave 2 review: only 2 ⚠️ partial rows + 1 ❌ sub-item remain, ALL blocked on external deps (rent flow excluded from smoke by design; password-resets empty-state needs backing table ID; send-test-email needs MAILCHANNELS_API_KEY). No actionable code gaps. |

**Refund gate — remaining edge:**
- Admin **approve** path calls Stripe via `issue_audited_refund`. E2E tests cover it with `stripe_client=None` (DB integration tests) and via the admin UI (DB-seeded refund_requests, decline fully tested). A full approve→Stripe-refund e2e requires either a Stripe test-mode payment intent or `STRIPE_SECRET_KEY` unset on the test stack.
- `dispute_refund_idempotency_key` dead-code warning in non-test builds (used only by webhooks tests) — cosmetic.

### 2026-07-25 session (#427 core — Anthropic API key reverse proxy)

Shipped the **core** of #427 as a new standalone workspace crate `anthropic-proxy` (decision:
host-side reverse proxy). The customer container's `ANTHROPIC_BASE_URL` points at a host-side
`anthropic-proxy` process that: strips any client-supplied `x-api-key`/`Authorization`/
`anthropic-version`, injects the platform key upstream per-request (the key **never enters the
container**), forwards the request path-transparently to the Anthropic-compatible upstream, streams
the response back, and meters token usage per identity (non-streaming JSON + streaming SSE terminal
`message_delta`). PoC proven end-to-end against the real z.ai Anthropic-compatible endpoint (both
non-streaming + streaming); key redaction verified absent from all logs/errors.

- Acceptance **#1** (architecture decision): done (host-side reverse proxy).
- Acceptance **#2** (proxy injects key + meters per identity): **shipped** — crate + binary, 33 tests
  green (nextest 0.13s), clippy clean, workspace build intact. MeteringRecorder trait leaves the
  DB-backed recorder (writes `agent_runs.claude_{input,output}_tokens`) to #415/#416.
- Acceptance **#3** (remove shared-key mount from container config) + **#4** (migrate beta
  customers): **BLOCKED on #413** — its Rust container-provisioning does not exist yet
  (`rg anthropic_api_key` = 0 Rust hits; #413 is spec-only). Do NOT attempt until #413 lands.

Issue **#427 stays open** (blocked on #413 for #3/#4). dc-agent integration (spawn the proxy as a
host-side process per identity, point the container's `ANTHROPIC_BASE_URL` at it) is also #413 scope.

### 2026-07-25 session (GH issue sweep — #442 / #426 / #444)

Sweep of all open GH issues with parallel subagents. The 3 credential-free items shipped;
**the entire Decent-Agents cluster is blocked** (see blocker note at the foot of this section).

| Fix | Area | Resolution |
|-----|------|------------|
| #442 create-offering price auto-suggest | UX (decided) | `c14cb939`: pre-fill `#monthly-price` with `cost × 1.15` (15% markup) when Hetzner cost known; provider-overridable via a `monthlyPriceTouched` flag (never clobbers a typed value); hint copy "suggested at 15% markup, adjust as needed". Pure `suggestMonthlyPrice(cost)` helper + `DEFAULT_MARKUP` const in `offering-wizard.ts` (10 unit tests); 2 e2e (pre-fill + override-reaches-API). Issue **closed**. |
| #426 out-of-order Stripe webhook (dispute before checkout) | Backend + test | `8ab75838`: **investigated real behavior first** — `checkout.session.completed` sets the PI but never touched `contract_disputes`; an orphan dispute (all lookups fail) stayed orphaned **permanently**. Shipped outcome (a): minimal money-safe `relink_orphan_disputes_for_payment_intent` (one idempotent UPDATE backfilling `contract_id`, `WHERE contract_id IS NULL` ⇒ replay-safe, touches NO money/status column ⇒ cannot double-refund). Wired best-effort into checkout completion. New DB test `test_orphan_dispute_relinks_on_late_checkout_completion` (proven to FAIL on a no-op then PASS). Issue **closed**. Scoped-out retroactive pause/refund replay filed as **#447** (money-path, separate concern). |
| #444 large-file splits (Waves 5-7) | Tech debt | **6 providers.rs splits** (`74fb9248` PoolsApi, `b4259194` NotificationsApi, `ae97cd8f` SlaApi, `290a218f` AllowlistApi, `d94d29af` OfferingCsvApi, `b5aa9acb` ProviderStatsApi): providers.rs 6739→**4280** (−2459); shared helpers (`validate_cloud_offering`, `build_response_metrics`) kept `pub(crate)`. Each verified byte-identical OpenAPI (189 paths / 332 schemas) via spare-port instance diff. **Wave 7** `c7dbf962` split `api-cli.rs` (3753L) → dir-bin (`main.rs` 547L + 13 subcommand modules), zero OpenAPI impact, `--help` byte-identical, 16 tests green. Tuple arity hit the **poem-openapi 16-max** on the 2nd inner tuple → further *handler* splits need a tuple restructure first (Path A); separable providers.rs clusters now exhausted (4280L = interwoven core). #444 stays **open** — next: tuple restructure → accounts.rs (2903L) clusters, then `database/offerings.rs` (2865L) recommendations block. |

**Decent-Agents cluster — BLOCKED on credentials + unbuilt infrastructure (per AGENTS.md mandatory
workflow, STOP + report; not mocked/stubbed).** `scripts/dc-secrets list shared/env` shows only
`TELEGRAM_ADMIN_CHAT_ID` + `PIPELINE_BOT_TOKEN`. Missing: Anthropic API key (#427/#429), Stripe
secret key (#418/#415), Google OAuth client ID + GitHub App credentials (#418). The product infra
also does not exist yet — grep found only the Stripe webhook receiver; there is **no GitHub App
webhook receiver** (so #431's "extend the verifier for two secrets" has nothing to extend), and
no agent-dispatch / metering / proxy subsystem. These are greenfield epics needing creds +
architectural decisions before one-pass production work can begin:

| # | Title | Blocker |
|---|-------|---------|
| 418 | beta onboarding (invite + first-run demo) | Needs Stripe + Google OAuth + GitHub App creds + email/magic-link; the whole onboarding flow is greenfield. |
| 427 | Anthropic API key proxy/sidecar | **Core shipped** (host-side reverse proxy; `anthropic-proxy` crate). Remaining acceptance #3/#4 (remove shared-key mount + migrate beta) blocked on #413's Rust container-provisioning (spec-only today). |
| 415 | subscription billing + active-hour/token caps | Depends on #427 (dispatch enforcement) + Stripe creds. Meter-table scaffold alone can't be PoC'd end-to-end. |
| 416 | usage metering + customer dashboard | Depends on #415 meters. |
| 429 | Anthropic key exfiltration mitigation | Depends on #427 + the agent container infra. |
| 431 | GitHub App webhook secret rotation | Blocked: no GitHub App webhook verifier exists yet (depends on #418). |
| 430 | CODEOWNERS / branch-protection deadlock UX | Depends on #418 onboarding flow. |
| 432 | per-identity observability + incident runbook | Depends on the agent infra (#413) + Anthropic creds. |

### 2026-07-25 session (robustness tail + CLI e2e harness + #445/#446 closure)

Continuation sweep (baseline `6f6548c8` → `dba28955`/`d11c718d`, 17 commits). Closed the two open
in-scope GH issues (#445, #446); finished the robustness tail + hex-decode migration flagged by the
2026-07-25 audits; built a flow-level e2e harness for the `dc` CLI (and fixed two real bugs it
uncovered); closed the e2e harness tail (C1-C3). Live UX audit: **no net-new issues** (drove the
real site across public + authed surfaces).

| Fix | Area | Resolution |
|-----|------|------------|
| #446 recovery-flow e2e "Continue never reaches Processing" | Test (BUG label) | `9b168add`: root cause was STALE TEST ASSERTIONS, not a frontend dead-end — tests expected `.bg-red-500/20` + "Invalid token" but the recover page surfaces API errors in `.bg-danger/10` with "Invalid recovery token hex: …". Added `waitForResponse` + asserted the real error div; renamed the misleading test. Issue closed. |
| #445 verify-email/recovery success → auto-redirect | UX (enhancement) | `3b501c62`: new shared `AutoRedirect.svelte` (4s countdown + `goto` + cleared interval); wired into verify-email + recover success states; 3 manual options always available (countdown copy + inline "Go now" + retained button). New tests drive the real app; also closes the recovery-success-path coverage gap. Issue closed. |
| Robustness tail A1/A3/A4/A5/A6 | Robustness | `902cb032` proxmox verify_api_token 30s timeout (testable `build_verify_client`); `aed335bf` dedup `REQUEST_TIMEOUT_SECS` via `pub(crate) HTTP_TIMEOUT_SECS`; `8b706c45` `run_command_with_timeout` for 4 `upgrade.rs` commands (10s/30s); `df6002f9` gateway ssh 20s `tokio::time::timeout`; `dc0e9432` doctor `ss` failure → `[WARN]`. dc-agent 245/245. |
| hex::decode migration tail (A2) | DRY | 5 USER-INPUT sites migrated to `decode_hex_path`/`decode_pubkey` (`41f3c6fa`, `98432fc4`, `4a8dbd7a`, `c247891d` + `accounts.rs`); detailed errors replace terse `Err(_)`. The remaining 10 are deliberate DB/Stripe-sourced non-fits (documented in `docs/audits/2026-07-25-code-robustness.md`). |
| CLI provider pool commands 100% broken (auth drift) | Bug (critical) | `b48bdb9b` + `7fe544cd`: `cli/commands/provider.rs` had drifted 4 ways from the canonical signer (wrong header names `X-DC-*` vs `X-Public-Key`; millis vs nanos; no nonce; newline-joined vs byte-concat message). Extracted `common/src/api_auth.rs` as the SINGLE signing source (`sign_request` + `build_signed_message`); `api/auth.rs` verify + `api_cli/client.rs` + cli all delegate to it. Also fixed `tiers` omission in the pool-generate schema. |
| CLI flow-level e2e harness | Coverage | `dba28955`: new `cli/tests/cli_flows.rs` (12 tests, 0.316s offline; warm-stack + IC-mainnet tiers). Covers keygen determinism/reimport-recovery/multilang/stdin, ledger-local, all-local listings, subcommand help, invalid-mnemonic rejection; warm-stack tests prove the auth fix against the real API (assert NO 401 + contains "Pool not found"). |
| E2E harness tail C1-C3 | Coverage / UX | `02503591` C1 offering-edit beforeAll sharing (4 tests share seed); `2d82a6d5` C2 agent-pool rename PUT + detail render (new `agent-pool-edit.spec.ts`); `d11c718d` C3 become-provider `?step=N` deep-link (pure `wizard-logic.ts` + 19 unit tests, TDD). |

**Still open / deferred (unchanged):**
- ~~**#442** create-offering price auto-suggest — needs a product decision (margin/heuristics).~~ → **CLOSED later in the 2026-07-25 GH issue sweep** (`c14cb939`; see above). Kept struck-through as a historical record of this session's tail state.
- **#444** remaining large-file splits — roadmap filed (`docs/plans/2026-07-25-large-file-splits-444.md`).
- **10 deliberate hex non-fit sites** — documented in `docs/audits/2026-07-25-code-robustness.md`.

### 2026-07-25 session (fresh sweep: robustness + UX + coverage + #444 split)

Six-wave sweep (baseline `56df84e6` → `64e46ef4`). GH **#441** and **#436** closed as completed;
net-new UX **#5** (offering-edit ownership) filed and fixed; a robustness/DRY pass; one safe #444
split; and e2e coverage + harness improvements. Final: svelte-check 0/0, vitest 847, clippy clean
(3 known baseline warnings, 0 new), cargo `--lib` 1011/0, smoke 27 @ ~33s, full e2e 300/3 (1 known
parallel flake + 2 **pre-existing** recovery-flow failures unrelated to this session — filed).

| Fix | Area | Resolution |
|-----|------|------------|
| #441 subscription trial/CTA mismatch | UX | `b1158bff`: honest copy via `shouldShowTrialCopy(plan)` = `trialDays>0 && stripePriceId`; contact-sales-only plans no longer advertise a trial. `@smoke` test. |
| #436 seed-phrase sign-in hidden behind extra click | UX | `3fa993a4` + `ea29b0a3`: new public `GET /api/v1/auth/capabilities` (`{google_oauth: bool}`); frontend defaults to credential form when OAuth off. Server env = single source of truth. (Success-screen auto-redirect bonus deferred — filed as **#445**.) |
| #5 (net-new) offering-edit ownership | UX/security | `43ffae8e` + `958ebff1`: `/dashboard/offerings/[id]/edit` redirects non-owners to the view-only route; narrowed identity used in the guard. |
| ICPay-cleanup cluster | Backend + seed + UX | Reject non-Stripe currency at offering create/update `79c83657`; migrate ICP offerings/contracts → USD `058a36e6`; remove stale ICP labels + dead ICP price feed `83605227`; remove dead ICP price feed backend `05c27f01`. |
| http timeouts (money/identity/provisioning) | Robustness | `execute_command` setup helper 300s `40d217f8`; cli provider commands `70b6c4ac`; dc-agent manual provisioner webhook `5da340a4`. |
| STRIPE_API_BASE DRY | DRY | `85afbd8c`: `pub const STRIPE_API_BASE` in `stripe_client.rs`, 5 hardcoded URLs removed; contracts test fixtures finished `40d22a0c`. |
| Silent errors logged, dead code removed | Robustness | Log-don't-swallow in dc-agent doctor/proxmox/chatwoot init `f55750d5`; dead `build_auth_headers` + `post_provision` shim removed `11fc0d2c`. |
| Hex decoding DRY + detailed errors | DRY | `d1cce292`: 18 user-input sites → `decode_pubkey`/`decode_hex_path` helpers in `openapi/common.rs`; terse "Invalid format" → detailed (names field + problem). 22 deliberate DB-sourced non-fit sites documented in `docs/audits/2026-07-25-code-robustness.md`. |
| #444 first safe split | Tech debt | `74fb9248`: `PoolsApi` extracted from `providers.rs` (−957 lines, zero behavior change). Roadmap `c4c68e09`. #444 stays open (ongoing). |
| E2E coverage + harness | Coverage | verify-email success path `c8815db4`; cloud-accounts populated + disconnect `54fa508a`; Stripe `checkout.session.completed` money path `0604f360`; self-contained search-dsl `e5911dd4`; helpers promoted `92058c24`; 7 delete specs parametrized `67f84f7f`; route-audit settle-on-fetch `e0726927`; 5 fast smokes `f4893141`. |
| Smoke fast-loop tuning | Test | `64e46ef4`: demoted 5 slow non-critical specs from `@smoke` → 27 tests @ ~33s (was 32 @ ~51s); kept the authed dashboard, anonymous landing/error, verify-email, sign-in, and #441 money-path. |

**Still open / deferred (deliberate):**
- ~~**#442** create-offering price auto-suggest — needs a product decision (margin/heuristics).~~ → **CLOSED** (`c14cb939`; see above). Kept struck-through as a historical record.
- **#444** remaining large-file splits — roadmap filed (`docs/plans/2026-07-25-large-file-splits-444.md`).
- **#436 success-screen auto-redirect bonus** — skipped at the time; filed as **#445** (now **closed** in the continuation session).
- **`scripts/browser.js --seed`** onboarding-flag tooling note — minor test helper, documented in-repo.
- **22 deliberate hex non-fit sites** — documented in `docs/audits/2026-07-25-code-robustness.md`.
- **2 pre-existing `recovery-flow` e2e failures** — filed as **#446** at the time; **RESOLVED** in the continuation session (`9b168add` — stale assertions, not a frontend bug).

### 2026-07-24 session (ICPay retirement + test stabilization)

ICPay (the ICP cryptocurrency payment rail) fully retired — **Stripe is the sole rail** — then a
stabilization pass fixed the flakes exposed by the required DB reset (the migration 049 CHECK edit
changed its checksum). GH **#443** (boot-gate `require_icpay_in_prod`) closed as moot; **#420**
(ICPay payouts) moot. Final baseline: full e2e **299 passed, 0 failed, 2 workers, ~6.6m**; smoke
**23 tests, ~29s**.

| Fix | Area | Resolution |
|-----|------|------------|
| ICPay payment rail fully retired — Stripe is the sole rail | Backend + frontend + config | Backend (`PaymentMethod` enum ICPay variant, `icpay_client`, escrow release/payout subsystem, webhook, endpoint, schema columns + `payment_releases` table, migration 049 CHECK rewrite), frontend (`RentalRequestDialog` Stripe-only, ICPay SDK pkgs removed, admin payout subsystem, env/compose/secrets), config all removed. `payment_method` default → `'test'` (Test absorbs auto-succeed). Commits: `fb4328be` `a773bdd6` `0b564bf9` `02ed7c2a` `1215b077` `1fc1d87f` `5c165eee` `2b18e4c7` `e9b7e0f3` `f0f44c8e` `5a228a63`. Dead `loadStripe` client + stale 'ICP (Internet Computer)' onboarding label also removed (`e7d7b3e4`). |
| Test stabilization (post DB-reset) | Test / reliability | `offering-status-badge` spec made self-seeding — was ambient-data-dependent, broke on DB reset (`469f48b6`); route-audit hardened against transient SvelteKit navigation fetch races (`b473e14c`); local e2e default workers 4→2 for reliability under persistent harness CPU load (`f1b9f088`). |

### 2026-07-24 session (fresh sweep: robustness + UX + coverage + create-bug)

Three read-only audits (`docs/audits/2026-07-24-{fresh-ux,code-robustness,coverage-and-ux-flow}.md`) → triaged, shipped high-confidence fixes via TDD, parked product decisions as #441-#444.

| Fix | Area | Resolution |
|-----|------|------------|
| Missing reqwest timeouts across money/identity/provisioning | Robustness | Shipped in `6cc6199c`: shared `http_client()` helper (30s timeout) in new `api/src/http_util.rs`; replaced ALL bare `Client::new()` in stripe/icpay/oauth/cloudflare/invoices/llm/chatwoot/embeddings/price-cache/vies/telegram/sms + 12 api-cli sites + `api_cli/client.rs`; `.timeout(120s)` on both dc-agent upgrade builders. |
| Silent hex-decode in receipts (refund + accept notifications) | Robustness | Shipped in `c66bd3f9`: `receipts.rs:297/418` `if let Ok`=hex::decode → `match` + `tracing::warn!` (contract id, parse error, bad value). |
| typst subprocess no timeout | Robustness | Shipped in `5ad502a5`: `invoices.rs` typst `.output()` wrapped in `tokio::time::timeout(30s)`. |
| Silent dispute-hex fallthrough | Robustness | Shipped in `707f0d97`: `webhooks.rs:779` → `match` + warn. |
| Hardcoded Stripe URL | DRY | Shipped in `f4357348`: `const STRIPE_API_BASE`. |
| Dead `network_metrics` module | Tech debt | Shipped in `4b472e73`: deleted unreferenced module (`load_ledger_metrics`). |
| Inconsistent hex::decode path boilerplate (~40 sites) | DRY | Shipped in `164bbdb4`: shared `decode_hex_path`/`decode_pubkey` in `openapi/common.rs`; unified terse→detailed error msgs. |
| Reputation "Poor" badge for zero health checks | UX | Shipped in `6df2155e`: neutral "No health checks yet" badge when `totalChecks===0`. Same class as #435. |
| Stale `© 2025` footer | UX | Shipped in `4b6659c0`: dynamic `{new Date().getFullYear()}`. |
| Breadcrumb "Dashboard" → /dashboard/rentals mismatch | UX | Shipped in `757bd79b`: relabeled "My Rentals". |
| Orphaned `/dashboard/user/[id]` route | UX | Shipped in `4292bdc9`: 307 redirect to reputation page (matches marketplace pattern). |
| Command palette had zero provider actions | UX | Shipped in `165b6720`: Create Offering/My Offerings/Agent Pools/Billing Settings gated on auth. |
| ALL native `confirm()` dialogs (6 dashboard + 5 components) | UX + e2e | Shipped across `1077dd33`,`fa82ec0e`,`41491746`,`b4ad6b61`,`d6acdf94`,`938d6c83`,`24924b51`,`d6425c10`,`d2bd52c3`,`8e348415`,`dc8ee2f3`: every native `confirm()` → inline two-step (request/confirm/cancel + pendingId). `rg "confirm\(" website/src` = 0 live calls. Unblocks headless e2e + mobile UX + consistency. |
| Create-offering 400 on every UI create (#440) | Bug (critical) | Shipped in `ebebff02`: poem-openapi ignores `#[serde(default)]`; applied `#[oai(default)]` to `Offering.pubkey` so missing field deserializes, then handler overwrites from URL path. |
| E2E coverage: 7 documented gaps closed | Coverage | add-device `@smoke` (`0730350e`), compare `@smoke` (`18b4a35b`), agent-pool (`f7b38826`), earnings (`dc84a706`), onboarding (`5f2ca8d4`), admin mutations ❌→✅ (`157ec457`), create-offering (`ebebff02`). New seed-helpers: `deleteContractsByProvider`, `deleteAgentPoolsByProvider`, `deleteProviderProfileByPubkey`, `signedApiCall`, `identityFromSeedPhrase`. |
| Stale test assertion (unified pubkey error msg) | Test | Shipped in `54c1e54d`: `provider-response-metrics.spec` asserted old terse msg; updated to `toContain('Invalid pubkey hex')` + echoed bad value. |
| Full suite baseline | — | **300 passed, 0 failed, 5.6m, 4 workers** (was 267 at session start; +33 tests from coverage closures + confirm-conversion specs). |

### 2026-07-23 session (money-safety hardening + route audit + UX review)

| Fix | Severity | Resolution |
|-----|----------|------------|
| R1: provider can drive requested→active unpaid | Critical | Shipped in `e6b5441e`: `update_contract_status` gates Provisioned/Active on `payment_status='succeeded'` OR `payment_amount_e9s=0`. Migration 048 DB CHECK. |
| R2/R3: refund+release unbounded + TOCTOU | Critical | Shipped in `45d40d82`: migration 049 CHECK `released+refund<=payment`; conditional UPDATE release path; `reject_contract`→`calculate_net_refund_e9s`. |
| R5: "refunded" with no money returned | Critical | Shipped in `6b3ad47e`: callers treat `Ok(None)` as "refund NOT performed"; `STRIPE_SECRET_KEY` required when `ENVIRONMENT=prod`. |
| R9: dispute-lost refund over-pays released funds | High | Shipped in `46edc93c`: `process_dispute_lost_refund` uses `calculate_net_refund_e9s` (subtracts `total_released_e9s`). |
| R10: `payment_status` accepts any string | High | Shipped in `220c2a82`: allow-list in code + migration 047 DB CHECK. |
| SSE auth double-prefix bug (`/api/v1/api/v1/...`) | Critical | Shipped in `d5a2e019`: SSE handlers verified against REAL request path. Was masked by env var bug. |
| Cluster A: SSE 404s (wrong env var `VITE_API_BASE_URL`) | High | Shipped in `02affbf7`: import `API_BASE_URL` from `api.ts` (2 pages). |
| B1: contract usage 401 (wrong signature path) | Medium | Shipped in `f40e35eb`: sign for correct `/contracts/{id}/usage` path. |
| B2: pending-password-reset 401 (agent-only auth) | Medium | Shipped in `e7519ee4`: `AgentAuthenticatedUser` → `ProviderOrAgentAuth`. |
| B3/B4: user activity 401 (own-only endpoint on public pages) | Medium | Shipped in `6b4d36e2`: new `GET /users/:pk/public-profile` with `PublicContractSummary` (no payment/SSH/gateway). |
| Command palette keyboard nav completely broken | High | Shipped in `09097cda`: arrows/Enter/Escape now work; visible Cmd/Ctrl+K trigger in sidebar. |
| Provider shown as raw hex pubkey on rentals | Medium | Shipped in `1921474b`: contracts query LEFT JOINs username; UI shows `@username`. |
| Test-infra: hardcoded 49-entry migration array | Tech debt | Shipped in `51131bfd`: replaced with `sqlx::migrate!()` (+37/−290 lines). |
| E2e smoke tier: only 4 tests | Coverage | Shipped in `0c033da2`+`4ead5c05`: 17 `@smoke` tests in 18s; scripts scan via `--grep`. |
| E2e: no flow catalog | Coverage | Shipped in `517b97cb`: `FLOWS.md` — 74 flows cataloged. |
| E2e: provider accept/reject uncovered | Coverage | Shipped in `dd955f4f`: 4 serial tests (see pending, accept, reject, auto-accept toggle). |
| Full suite baseline | — | **264 passed, 0 failed, 3.8m, 4 workers.** |

### 2026-07-23 session (e2e radical overhaul + issue sweep + sharding harness)

| Fix | Severity | Resolution |
|-----|----------|------------|
| `reputation-detail.spec.ts` hardcoded `uxaudit` pubkey (drifted after re-seed) | Fragile | Shipped in `c8e25e3a`: self-contained test seeds its own account, derives pubkey, asserts, cleans up. Was the 1 failure in a re-baselined 201/1 suite. |
| F1: `/dashboard` 'Get Started' CTA → `/dashboard/provider` 404s (no such route) | High | Shipped in `9dad0734`: href → `/dashboard/provider/support` (the setup wizard). The only broken link in the dashboard; 19/20 internal hrefs resolve. TDD RED→GREEN. |
| F2: onboarding modal gated on `sessionStorage` (reappears each browser session) + always said 'Complete your profile' even when complete | Medium | Shipped (F2): switched to `localStorage` (`WelcomeModal.svelte`); dynamic copy 'Your profile is ready' when username+email both set. Fixtures + existing onboarding tests updated. |
| CLI: dead `dialoguer` dependency (never used) | Tech debt | Shipped in `c29173b5`: removed from `cli/Cargo.toml` + workspace `Cargo.toml`. |
| CLI: 20 fake string-literal tests (asserted Display strings, never invoked the binary) | Tech debt | Shipped in `db5997cd`: replaced with 10 real `assert_cmd` subprocess smoke tests (`--help`/`-V`, keygen generate/import, ledger-local list, network dispatch, clap validation). Binary e2e coverage 0%→real. Net 39→29 tests. |
| saved-offerings + offering-detail-save hardcoded seed_data IDs/names | Fragile | Shipped (fragile commit): both specs now seed their own offerings under a random pubkey. |
| `account.spec.ts` seeded account with no cleanup (orphaned rows/run) | Fragile | Shipped (fragile commit): added `deleteAccountByUsername` in finally. |
| `recovery-flow.spec.ts` 2× `waitForTimeout(100)` sleeps | Fragile | Shipped (fragile commit): replaced with `waitForResponse` on the recovery API. |
| Sharding harness built + two blockers it exposed | Infra | Shipped in `297009d9`: dev CORS now allows any `localhost/127.0.0.1:*` origin (was a static list — shard ports 403'd); service worker no longer intercepts non-navigate fetches (was masking real API errors as 503); new `fixtures/api-base.ts` resolves API URL from stack port (4 specs hardcoded 59011). |
| Offering EDIT flow `/dashboard/offerings/[id]/edit` — zero coverage | Coverage | Shipped in `c97a497d`: 4 e2e tests (pre-fill, live diff panel, submit+redirect+DB persistence, validation). No source bug found. |
| Full suite baseline | — | **209 passed, 0 failed, 0 skipped, 0 networkidle, ~4.5m, 4 workers** (single warm stack). |

### 2026-07-23 session (e2e harness hardening + skip-gap closure + UX audit)

| Fix | Severity | Resolution |
|-----|----------|------------|
| `npx playwright test` defaulted to Docker port (59000), not warm stack | Test | Shipped in `3f7f9512`: `baseURL` now defaults to warm-stack 59010; Docker mode sets env explicitly. Bare `npx playwright test` Just Works. |
| 4 always-skipping e2e tests (payment-flows ×3, post-rental-welcome, marketplace-empty-state) | Test | Shipped in `6b8bafad`/`b7c05d17`/`b64effe0`/`0978c404`: new `seedRentableOffering` fixture (self_provisioned → always online). payment-flows root cause was a stale selector ("Rent Resource" button never existed; button reads "Rent"). post-rental-welcome rewritten against real `?welcome=true` banner + seeded contract (dropped first-party verify-checkout mock). marketplace-empty-state rewritten against the real default-hide path. **0 skipped now (was 4).** |
| ~19 active `networkidle` calls across 6 specs (prior "0" claim was inaccurate) | Test | Shipped in `e59e76d4`: replaced all with deterministic waits via new `clickAndRetry` helper (SSR-hydration-safe click loop) + `waitForResponse`. **Suite now genuinely 0 networkidle.** |
| payment-flows webhook helpers POST to wrong server | Test | Shipped in `b7c05d17`: `baseURL.replace('59000','59001')` was a no-op against warm stack 59010 → would POST webhooks to the web server. Now uses `PLAYWRIGHT_API_URL \|\| 59011`. |
| search-dsl `type:gpu` test flaked under parallel load | Test | Shipped in `995ac799`: `count()` immediately after `waitForResponse` hit a render gap. Gated on a GPU row rendering first. |
| Full suite baseline | — | **202 passed, 0 failed, 0 skipped, 0 networkidle, 152s, 4 workers.** |
| Live UX audit (10 pages, no mocks) | — | No actionable defects. zai-vision's 4 flags were all false positives (dark-theme contrast is 7.3:1 = WCAG AAA; truncation is intentional). Console clean apart from known dev warnings. |

### 2026-07-22 session (e2e harness overhaul + UX fixes)

| Fix | Severity | Resolution |
|-----|----------|------------|
| C1: Marketplace shows 0 offerings (demo/offline hidden by default, dead-end empty state) | Critical | Shipped in `a2ed9fd1`: split filter chain into `userFiltered` + `defaultHiddenCount`. Empty state now shows one-click 'Show N offerings' reveal button when defaults are the only cause. |
| C2: Profile page crashes ('No account username found' race) | Critical | Shipped in `67efb570`: `UserProfileEditor` takes `username` prop (no throw). Profile page guards on `currentIdentity?.account`. |
| H1: Billing spending-alerts renders raw 'not found' (endpoints missing) | High | Shipped in `6d589c5f`: removed `#[cfg(test)]` from `upsert/delete_spending_alert`, added GET/PUT/DELETE `/users/:pubkey/spending-alert` routes in `users.rs`. `api.ts` treats 404 as null. |
| H5: Login lacks discoverable registration path (Generate New hidden) | High | Shipped in `39962212`: added 'New here? Create an account' CTA on login page; `initialSeedMode` state jumps directly to generate step. |
| M1: Dashboard shows provider metrics (Trust 90, Red Flags) to non-providers | Medium | Shipped in `63d0ac4a`: TrustDashboard gated on `userRole === 'provider'` via `detectUserRole()`. |
| M2: Marketplace 'Category:' label mislabeled (holds regions/price) | Medium | Shipped in `4665ada2`: renamed to 'Quick filters:'. |
| M4: Email+seed banners clutter 19/22 pages | Medium | Shipped in `dda320cf`: email banner now has per-session dismiss button (same pattern as seed banner). |
| Invoices parallelism flake (DB state shared via testAccount pubkey) | Test | Shipped in `85cd37ec`: added `test.describe.configure({ mode: 'serial' })` to invoices.spec.ts. |
| E2E suite: 0 `networkidle` calls, 0 `registerNewAccount` in API tests | Test | 13 commits: replaced 14 networkidle + 4 registerNewAccount with deterministic waits/seedAccountDirect. Extracted 4 DRY helpers. Consolidated 12 tests→4. |
| E2E coverage: 17 GAP routes, 4 THIN flows | Test | 8 new spec files (18 tests): verify-email, agents-pricing, become-provider, reputation-detail, account-subscription, provider-pages-smoke (8 routes), account-profile-edit, provider-requests-auth. |
| UX: marketplace `/` keyboard shortcut for search focus | UX | Shipped in `dda320cf`: `/` focuses marketplace search (with visible `<kbd>` hint). Ignores when already in input/textarea. |

### 2026-07-21 session (prior)

| # | Title | Resolution |
| 437 | Marketplace: click-to-cycle visibility/stock buttons are surprising | Shipped in `e2d28e6e`: new `OfferingStatusMenu.svelte` (button + conditional panel, per-card mutual exclusion via `globalThis.__offeringStatusMenus` registry, click-outside/ESC/Enter/Space a11y, `role="menu"` + `role="menuitemradio"`). Panel auto-flips up to avoid overlapping the wrapped stock trigger. Load switched to `getMyOfferings` so owners see shared/private offerings (was filtered out by the public endpoint). E2E in `offerings-status-menus.spec.ts` (4 tests). |
| 438 | Dashboard layout: email banner preempts seed-phrase backup banner (recovery risk) | Shipped in `29efa840`: banners now render as static-block siblings inside one fixed container (each independently dismissable). `mainTopPadding` derived expr picks the right offset for both/one/none. `EmailVerificationBanner`/`SeedPhraseBackupBanner` lost their per-component positioning. E2E in `dashboard-banners.spec.ts` (4 tests). |
| 439 | Marketplace: sort UI hidden on mobile (`hidden md:flex`) | Shipped in `698329f7`: added `<select aria-label="Sort offerings">` next to the pill row. Pills unchanged on desktop; select is the mobile-only affordance + a11y alternative. Both bind to the same `sortField`/`sortDir` state and reuse `syncFiltersToUrl()`. E2E in `marketplace-sort.spec.ts` (3 tests). |
| 435 | Offering detail SLA chart renders empty gray bars when provider has no SLA data | Shipped in `ccfcb1b0`: chart now gated on `reports30d > 0`. When a provider has set an SLA target but submitted zero SLI reports, the card shows a friendly empty state ('No SLA reports in the last 30 days') instead of 30 misleading gray bars. Target stays visible in the card header. E2E in `offering-sla-empty-state.spec.ts`. |
| 433 | No UI to top up account balance — `/dashboard/transfers` only shows history | **Small-fix path** shipped in `9df37443`: balance card gained explanatory subtitle (P2P transfer units; rentals are per-transaction at checkout). E2E in `transfers.spec.ts`. Larger pre-pay deposit CTA remains out of scope. |
| 410 | Stripe: cleanup stale pending contracts (payment timeout) | Shipped in `8ca5e070`: `Pending → Expired` transition allowed, `find_stale_pending`/`expire_pending` with money-safety guard `AND payment_status != 'succeeded'`, wired into `TimeoutCleanupService` via env `PENDING_TIMEOUT_SECONDS` (default 3600). Partial index `046`. |
| 434 | Flaky test: `account-notifications.spec.ts` in parallel runs (workers>1) | False alarm — fixed in `81615b77` (P3.5 mock audit). |

## In-repo known issues (not on GitHub)

### Triaged as non-bugs (2026-07-22 UX audit re-verification)

| ID | Finding | Status |
|----|---------|--------|
| H3 | Create-offering step 2 Hetzner dead-end | **False positive** — `<a href="/dashboard/cloud/accounts">` link exists at `offerings/create/+page.svelte:570`. Next button skips to step 3. Was stale build in audit. |
| H4 | Rentals 404 on `/contract-events` | **Resolved by server rebuild** — route always existed at `main.rs:1372`. Audit's 404 was from stale binary. |
| M3 | Create-offering placeholders-as-labels | **False positive** — all inputs/selects have proper `<label for="...">` elements. Was stale build in audit. |
| H6 | All offerings unrentable (Provider Offline) | **Operational** — correct behavior (disables Rent for offline provider). Dev seed data has offline provider; bring online via `node scripts/dc-auth.js seed-ux-data` keepalive daemon. |
| L2 | Account ID reads as placeholder (`aaaa00…000001`) | **Test data artifact** — the `uxaudit` test account's ID was hand-set to `aaaa…0001` for debugging. Not a UI bug. |
| L3 | Landing stats all 0 vs populated hero card | **Dev environment** — `GET /api/v1/stats` returns zeros against empty dev DB. Populated in production. |

### Deferred product decisions

| ID | Finding | Status |
|----|---------|--------|
| H2 | Transfers page has no Send/Receive UI | **Feature gap** — P2P send needs IC canister integration (product decision, not a bug). Balance card already explains rentals are per-transaction at checkout. Related: #433 (closed, small-fix), #420 (closed 2026-07-24 — ICPay rail retired). |
| M5 | Billing VAT country EU-only | **Known limitation** — global country list needs server-side VAT rule changes. Low priority pre-launch. |
| L1 | Security page: seed-login device is 'Unnamed Device' | **Minor UX** — consider prompting device name on first login. |

### E2E harness tech debt (in-repo, surfaced 2026-07-23)

| Finding | Status |
|---------|--------|
| Full suite 192s for 205 tests; <60s goal needs multi-stack sharding | **Empirically investigated — sharding does NOT help on this box.** Built full harness (`scripts/e2e-shard.sh`, `dev-server.sh` STACK_INDEX, `fixtures/api-base.ts`). Root cause: 3 shard stacks share ONE Postgres → competing pools = worse DB contention than single-stack's single pool (3×4w=22 fails/4m30s; 3×2w=4 flakes). **Single stack 4 workers = 205/0 green ~192s = proven optimum.** For sharding to truly help, each shard needs its own Postgres instance (future CI-runner work). As a side benefit, dev CORS now correctly allows any localhost origin and the service worker no longer masks API errors. |
| `scripts/browser.js eval --seed <phrase>` throws "UtilityScript.evaluate" | **Minor tooling** — `authenticatePage` (browser.js:332-336) does an extra `goto`+`networkidle`+300ms after seed inject; a SvelteKit client-side redirect/WelcomeModal likely destroys the eval context. `snap`/`shot`/`errs`/`html`/`tour` all work with `--seed`; only `eval` is affected. For authed JS eval, use the e2e framework. |
| `scripts/browser.js --seed` greedily consumes positional args | **Minor tooling** — `--seed <phrase> <url>` fails ("Got 14 words") because the parser consumes all subsequent non-flag args as seed words. Documented usage `snap <url> --seed "$SEED"` (seed last) works. One-line fix possible but it's a test helper, not product. |
| Coverage gap: rent→pay→view→cancel happy path (UI-created contract, not DB-seeded) | **CLOSED (2026-08-01)** — `rent-flow.spec.ts` (4 serial tests) drives the REAL marketplace Rent dialog → signed `POST /api/v1/contracts` → rentals list → detail page → signed `PUT .../cancel`, all against the warm stack. The contract commits at `requested` (cancellable) during create, before Stripe checkout, so the flow is drivable without `STRIPE_SECRET_KEY`. Cancel asserted from BOTH the detail page and the rentals-list card, with DB verification. Only the Stripe SDK script load (external boundary) is mocked. |
| Coverage gap: provider agent-pool mgmt `/dashboard/provider/agents/[pool_id]` | **PARTIALLY CLOSED (2026-07-25, `2d82a6d5` C2)** — pool create + rename PUT + detail-page render now covered in `agent-pool-edit.spec.ts`. Remaining gap: pool revoke/delete UI path (low priority). |

### Deferred product decisions (surfaced 2026-07-23 UX review)

| Finding | Status |
|---------|--------|
| No `?` keyboard-shortcut help overlay | **RESOLVED (2026-07-23; stale entry corrected 2026-07-24)** — `KeyboardHelpOverlay.svelte` exists and is covered by 3 tests in `keyboard-shortcuts.spec.ts` (`? opens help overlay listing all shortcuts` is `@smoke`). This row was stale; corrected. |
| Dashboard shows provider-monitoring stats to brand-new renters | **Design judgment** — fresh renters see "Infrastructure Uptime", "Contracts Monitored", "Red Flags Detected" cards. Non-Providers may find this confusing. Needs product input on conditional rendering. |

### 2026-08-03 session (staging → k8s `dc-stage` consolidation — Track 3, product-repo prep)

Product-repo half of the staging→k8s migration (Track 3 of the 3-track split in
the plan's Appendix B). Tracks 1 (k8s manifests) + 2 (`dc-stage` live on
cluster) were done by sibling agents in parallel; this session did the pushable
product-repo prep + the operator cutover runbook. **Operator cutover pending** —
see `docs/MIGRATION-CUTOVER.md`. The destructive deletions (retired dev-deploy
stack + age secret store) are deliberately NOT shipped (they break the live dev
host's next `git pull`); they are runbook Step G, a separate post-cutover commit.

| Change | Area | Detail |
|--------|------|--------|
| `deploy stage` target | cf/deploy.py | New `python3 cf/deploy.py deploy stage [--tag <tag>]`: builds the api image natively, pushes `git.kalaj.org/decent-stuff/decent-cloud-api:<tag>` (default floating `:stage`), bumps the k8s stage overlay `images:` entry, commits the k8s repo LOCALLY, prints the operator `git push` + ArgoCD refresh + health-check commands. Stage is k8s/ArgoCD (ns `dc-stage`), NOT docker-compose. Reuses `build_rust_binaries_natively`/`calculate_binary_hash`/`check_docker` (DRY). Legacy `dev` path intact. |
| `config stage` target | cf/deploy.py | Read-only introspection of the live dc-stage cluster stores (dc-stage-config ConfigMap + dc-stage-secret Secret), mirroring `config prod`. Refactored `_read_prod_stores` into a generic `_read_cluster_stores(namespace, configmap, secret)` (DRY) used by both prod + stage. |
| Image-tag bumper tests | cf/test_deploy.py | 8 unit tests for `_update_stage_image_tag`: update existing newTag (indented + column-0 list), insert when absent, idempotent no-op, no website-image false-match, missing-file/section/target loud errors, byte-for-byte preservation of the rest of the manifest. |
| Cutover runbook | docs/MIGRATION-CUTOVER.md | Authoritative operator guide: prerequisites, Step 0 verify, Steps A–G (push the k8s repo → encrypt secret → ship :stage → public cutover → enable api-sync → tear down dev host → delete retired files), rollback. Copy-pasteable real commands. |
| Plan status + decisions | docs/plans/2026-08-03-staging-to-…md | Status line → "Tracks 1+2+3 done; operator cutover pending". Open Decisions all RESOLVED: DB = separate `decent_cloud_stage`; image = floating `:stage` (pins prod tag until shipped); hostname = `stage-*` overlay, operator may keep `dev-*`. |
| Issue inventory | docs/OPEN_ISSUES.md | New "Infrastructure — staging → k8s consolidation" section: status + the 7 operator-gated steps (runbook A–G) as the remaining open items. |
| Deploy/secrets docs | AGENTS.md (+ cf/*) | Staging is now `dc-stage` on k8s (not docker-compose dev); canonical secret store is the outer `secrets/shared/env.yaml`; `repo/secrets/shared/` + `scripts/dc-secrets` are RETIRED pending post-cutover deletion. Links to the runbook. |

Gates: `python3 -m py_compile cf/deploy.py` clean; `python3 cf/test_deploy.py` →
8/8; `cf/deploy.py --help` / `deploy --help` / `config --help` render the new
`stage` choices. No Rust/website code touched; no cluster/k8s repo mutation.

### 2026-08-03 session (Track 2 PoC verified → docs recorded on PR #454)

Track 2 (live `dc-stage` PoC on the cluster) completed + was verified end-to-end.
This session recorded the verified results + the remaining operator gates into the
migration docs and pushed them to the open PR branch `staging-k8s-dc-stage-track3`
(PR #454). **PoC verified, cutover pending** — nothing was overstated as "done".

| Change | Area | Detail |
|--------|------|--------|
| Verified-results block | docs/MIGRATION-CUTOVER.md | New "Pre-cutover status (VERIFIED 2026-08-03)" section: HTTP 200 health body, stage DB (role `decent_cloud_stage` + 52 migrations / 86 tables, latest `52 | drop account subscription feature`), shared pgsql discovery (pod `pgsql-857cbb44d8-lbzw4`, `pgsql.apps.svc.cluster.local:5432`, `pgvector/pgvector:pg18`), namespace state (api 1/1, api-sync 0/0, redis/website 1/1, ClusterIP-only), Stripe `sk_test_`, the 2 bugs found + fixed (SMTP overlay `deb4018`, hostPath `chown 1000:1000`), non-fatal warnings (CHATWOOT 401, rate-limiting off, CF_* unwired in sync). |
| Step A note | docs/MIGRATION-CUTOVER.md | Push BOTH k8s repo commits `7013258` + `deb4018` together (else ArgoCD re-applies the broken SMTP patch). |
| Step B CRITICAL | docs/MIGRATION-CUTOVER.md | DB-password-reconciliation warning: the `decent_cloud_stage` pw lives only in the live `dc-stage-secret`; extract+commit OR `ALTER ROLE` before the first ArgoCD sync, else the SOPS value overwrites it and breaks DB auth. |
| Step E + minor | docs/MIGRATION-CUTOVER.md | api-sync re-enable uses `scale --replicas=1` (hostPath perms already fixed); new "Minor follow-ups" section (TWILIO empty, stale CHATWOOT token, optional SMTP_PASSWORD drop). |
| Plan doc | docs/plans/2026-08-03-…md | APPENDIX B Track 2 → "COMPLETE — VERIFIED LIVE" with results summary; top Status line updated; "Operator-gated" expanded to the 8 remaining items. |
| Issue inventory | docs/OPEN_ISSUES.md | Migration section → "PoC VERIFIED, cutover pending"; completed items marked ✅; open items expanded to 8 (incl. the CRITICAL DB-pw reconciliation + minor follow-ups). |

Docs-only session: no Rust/website code, no cluster/k8s repo mutation, no secrets
touched. Pushed over HTTPS+PAT to `staging-k8s-dc-stage-track3` (PR #454).
