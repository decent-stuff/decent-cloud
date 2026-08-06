# Real-Deployment Audit — Findings (2026-08-03 / 2026-08-04)

> **Scope:** a CDP-driven audit of the REAL production deployment of decent-cloud
> (`https://decent-cloud.org`, k8s namespace `dc-prod`) plus the Hetzner Cloud operator
> console, performed 2026-08-03 / 2026-08-04. This document maps every finding for triage in
> a **separate session** — it does NOT contain code fixes. Two findings are P0 (one resolved but
> needing a permanence fix; one open and the single biggest product blocker).
>
> **Companion inventory:** this doc is linked from `docs/OPEN_ISSUES.md` ("Real-deployment audit
> (2026-08-04)"). GitHub Issues remain the canonical live source; this is the categorized snapshot
> for the real-deployment surface specifically.

---

## Methodology

- Drove the **real, logged-in Hetzner Chrome** at `192.168.0.13:9223` over CDP via Playwright
  `connectOverCDP` (Chrome 146). The operator's master Hetzner tab was left intact throughout
  (verified alive at the end: `console.hetzner.com/projects`).
- Created **real** accounts / provider profile / cloud account / offering on **prod**
  (`https://decent-cloud.org`). **No VM was rented** — provisioning was never triggered; cloud
  spend = **$0**.
- Prod Hetzner token (`HETZNER_API_TOKEN_PROD`, 64-char Read & Write, Default project `12666465`)
  was validated live against `GET https://api.hetzner.cloud/v1/servers` → HTTP 200. The token
  value was **never printed or echoed**; the temp file holding it was `shred -u`'d after use.
- All HTTP statuses were captured via `page.waitForResponse` on the exact endpoint URL + method.
  Browser console errors were captured via `page.on('console')` (type `error`), `page.on('pageerror')`,
  and `page.on('requestfailed')`. Verbatim error text is preserved below.

### Recovery artifacts (operator)

- **Prod provider account public key** (Ed25519, hex):
  `1ed6136dddb93c70c6624d96822220b3d23d32b3f7c4d311d088ae4b41d2f53d`
- **Seed phrase** (12 words, BIP39) for that account is saved at `/tmp/.dc-prod-seed` (`chmod 600`).
  This is the operator-recovery credential.
- Account username `hetzner-reseller` (account id `83f40c509a8148d69846d3d4f3abd2ea`); offering id
  **11** (`hetzner-cx22-resold`); cloud-account id `1516a104-8daf-48b3-b739-24843e22c34a`.

---

## Prioritized table of contents

| ID | Sev | Title | Status |
|----|-----|-------|--------|
| **P0-A** | **P0** | Prod outage — `dc-secret`→`dc-prod-secret` rename pushed without re-applying the secret in-cluster | **Resolved out-of-band; PERMANENCE FIX NEEDED** |
| **P0-B** (= A2 #1) | **P0** | Path-A Hetzner offerings silently hidden from the public marketplace (the big product blocker) | **OPEN** |
| A1 #1 | P1 | `GET /v1/projects` does not exist — task verify step wrong | Documented |
| A2 #2 | P1 | `cx22` does not exist in the live Hetzner catalog | Documented |
| A2 #3 | P1 | No client-side guard for Hetzner `server_type × location` compatibility | OPEN |
| A2 #4 | P1 | Create-offering POSTs to `/providers/<me>/offerings`, NOT `/offerings` | Documented |
| Staging | P1 | `dc-stage` healthy internally but not publicly reachable (tunnel cutover pending) | Blocked on operator |
| A1 #2 | P2 | Project ambiguity — account has TWO projects | Documented |
| A1 #3 | P2 | Created-token value hidden behind a click-to-reveal overlay | Documented |
| A1 #4 | P2 | `sops set` requires the value to be valid JSON | Documented |
| A2 #5 | P2 | Persistent console errors on every dashboard page | OPEN (cosmetic) |
| A2 #6 | P2 | Email verification never triggered / not required | OPEN (confirm intent) |
| A2 #7 | P2 | Provider "registration" split across 3 surfaces; hub mis-sequences | OPEN |
| A1 #5 | P3 | Orphaned tokens created during iterative UI discovery (all deleted) | Resolved |
| A1 #6 | P3 | Cosmetic: token-list rows appear duplicated in DOM scrape | Documented |
| A2 #8 | P3 | Username `+`-tagged email accepted | Documented |
| A2 #9 | P3 | Playwright `waitForFunction` non-boolean return dumps process `env` | Documented (driver hygiene) |
| A2 #10 | P3 | Seed phrase recoverable from `localStorage.seed_phrases` post-registration | Documented (by design) |
| A2 #11 | P3 | Wizard step gating not enforced server-side (`?step=3` works) | Documented |

---

## P0 — Production

### P0-A — Prod outage: `dc-secret`→`dc-prod-secret` symmetry rename pushed without re-applying the secret

- **Severity:** P0 (acute prod outage).
- **Title:** nuc-k3s symmetry rename (`dc-secret`→`dc-prod-secret`) + `HETZNER_API_TOKEN`
  secretKeyRef stub pushed without re-applying the secret under its new name in-cluster.
- **Symptom:** ArgoCD re-synced the renamed manifests (referencing `dc-prod-secret`), but
  `dc-prod-secret` never existed in-cluster → ALL `dc-prod` pods went
  **`CreateContainerConfigError: secret "dc-prod-secret" not found`** for **~3.5 hours** → prod
  fully DOWN (**HTTP 530**, then **HTTP 502**).
- **Root cause (operator process, k8s repo — separate GitOps repo):** the operator pushed the
  symmetry rename (commit `86b1422`) + a `HETZNER_API_TOKEN` secretKeyRef stub (commit `4ac1b80`)
  **without** re-applying the secret under its new name in-cluster. This is **exactly** the
  cutover risk documented in `docs/MIGRATION-CUTOVER.md` and the symmetry commit body, but the
  secret-reapply step was skipped.
- **Reproduction:** rename a Secret in a manifest that Deployments reference by name, push, let
  ArgoCD sync → pods fail `CreateContainerConfigError` because the old name no longer exists and
  the new name was never created.
- **Recovery (applied out-of-band via kubectl, 2026-08-04):**
  1. Copied the live `dc-secret` → `dc-prod-secret` (37 keys).
  2. Patched `dc-prod-secret` to add `HETZNER_API_TOKEN` (the manifest references it but the old
     secret lacked it).
  3. Deleted stuck pods.
  - **Result:** prod is now **HTTP 200**, `dc-api` **1/1 Running**, `environment:prod`.
- **Permanence fix NEEDED (see "Blocked on operator"):**
  1. Operator runs `python3 third_party/k8s/scripts/manage-secrets.py` (in the **k8s repo**) so
     `dc-prod-secret` is reconciled from the renamed SOPS file — otherwise the next prune/rotation
     breaks prod again.
  2. **REMOVE the `HETZNER_API_TOKEN` secretKeyRef from `base/dc-api.yaml`** (k8s repo). It is
     **UNUSED** by the api-server: verified — the api-server never reads `HETZNER_API_TOKEN` from
     env; it is a per-provider-in-DB field (`cloud_accounts.credentials_encrypted`). The ONLY place
     `HETZNER_API_TOKEN` is read from env is the test binary `api/src/bin/api-cli/e2e.rs:203`
     (`env::var("HETZNER_API_TOKEN").context("HETZNER_API_TOKEN env var must be set for cloud
     provisioning E2E test")`). A deploy-time env ref that nothing consumes acutely blocked prod.
- **Suggested fix:** add a pre-push gate / atomic secret-bootstrap script so a manifest rename
  cannot reference a non-existent secret; OR keep both Secret names (`dc-secret` + `dc-prod-secret`)
  until the old one is explicitly retired.
- **Status:** **RESOLVED (out-of-band) — permanence fix pending.** Surfaced by the new
  real-deployment e2e harness (PR #459).

> **Note on file locations:** `base/dc-api.yaml`, `manage-secrets.py`, and the
> `dc-secret`/`dc-prod-secret` manifests live in the **separate k8s GitOps repo** (per
> `repo/AGENTS.md`: prod/stage deploy via ArgoCD from the k8s repo; these are NOT vendored into
> this product repo — `third_party/k8s/` does not exist here). The verified in-repo ref is
> `api/src/bin/api-cli/e2e.rs:203`.

### P0-B — Path-A Hetzner offerings silently hidden from the public marketplace (the BIG product blocker)

- **Severity:** P0 (product-architecture gap; directly blocks the "OpenRouter for cloud" vision).
- **Title:** A Hetzner reseller provider can create a cloud account + offering, but the offering
  never appears in the public marketplace (`GET /api/v1/offerings`). The entire "Resell a managed
  cloud" Path A is a dead end — you can list an offering nobody can see or rent.
- **Symptom (live, prod):** offering **11** (`hetzner-cx22-resold`) is `visibility:public`,
  `is_draft:false`, the provider profile exists, and it shows in the provider's OWN list
  `GET /api/v1/providers/<me>/offerings` (count 1). **BUT** `GET /api/v1/offerings?limit=100`
  does NOT return it (only the example offerings appear). A tenant cannot discover or rent it.
- **Root cause (code, this repo):** `Database::search_offerings` and `Database::search_offerings_dsl`
  in `api/src/database/offerings.rs` **post-filter** the SQL result:
  - `search_offerings` — `api/src/database/offerings.rs:691-699`:
    ```rust
    // Filter to only include offerings that have a matching pool or are self-provisioned
    let filtered: Vec<Offering> = with_status
        .into_iter()
        .filter(|o| {
            o.resolved_pool_id.is_some()
                || o.offering_source.as_deref() == Some("self_provisioned")
        })
        .take(params.limit as usize)
        .collect();
    ```
  - `search_offerings_dsl` — `api/src/database/offerings.rs:1048-1055` (identical predicate):
    ```rust
    .filter(|o| {
        o.resolved_pool_id.is_some()
            || o.offering_source.as_deref() == Some("self_provisioned")
    })
    ```
  A Path-A Hetzner offering has **NEITHER**: `agent_pool_id` is NULL (Path A is pool-free by
  design — VMs are provisioned by the central api with public IPs, no dc-agent), and
  `offering_source` is NULL because
  `website/src/routes/dashboard/offerings/create/+page.svelte:273` submits
  `offering_source: undefined` for the Hetzner path. `compute_provider_online_status`
  (`offerings.rs:689`/`:1046`) only sets `resolved_pool_id` when a pool matches by `agent_pool_id`
  or by `country_to_region(datacenter_country)` — and a Hetzner provider has zero pools.
- **Contrast:** example offerings appear because their rows carry a non-null pool association;
  `cloud_resources` listing (`api/src/openapi/cloud.rs`) correctly sets
  `offering_source:"self_provisioned"`, but the manual `create` path the UI uses does not.
- **Reproduction (verbatim click-path):**
  1. `/login` → Create account → `/dashboard/cloud/accounts` → Add Account (backend Hetzner, paste
     token → "Valid · Hetzner Cloud").
  2. `/dashboard/offerings/create` → step1 (name, visibility `public`, draft OFF) → step2 (pick
     cloud account, server type `cx23`, location `fsn1`, image `ubuntu-22.04`) → step3 (accept
     price) → "Create Offering" (POST `/api/v1/providers/<me>/offerings` → 200 `{"success":true,"data":11}`).
  3. `GET /api/v1/offerings?limit=100` → the new offering is **absent**.
  - URLs: marketplace `https://api.decent-cloud.org/api/v1/offerings`; create UI
    `https://decent-cloud.org/dashboard/offerings/create`.
- **Suggested fix (for the next session):** set `offering_source:"self_provisioned"` for any
  offering with `provisioner_type IN ('hetzner','vultr')` at create time; **OR** add
  `provisioner_type IN ('hetzner','vultr')` as an OR-branch in the marketplace filter predicate
  above. Without this, the hub's own promise ("Your offering appears in the marketplace catalog")
  is false.
- **Status:** **OPEN.** This is the single biggest blocker to decent-cloud being "OpenRouter for
  cloud."

---

## P1

### A1 #1 — `GET /v1/projects` does not exist (task's verify step is wrong)

- **Severity:** P1.
- **Title:** `/v1/projects` endpoint does not exist — the task's token-verify step is wrong.
- **Symptom:** `curl -H "Authorization: Bearer <TOKEN>" https://api.hetzner.cloud/v1/projects`
  returns **404** `{"error":{"message":"api route not found","code":"not_found"}}` for *every*
  token, including the operator's known-good master token. Expected (per task): HTTP 200.
- **Root cause:** task assumption. The Hetzner Cloud API v1 has **no `/v1/projects` route** —
  projects are a console-only concept, and Cloud API v1 tokens are silently scoped to one project.
- **Reproduction:** verify the tokens with `GET /v1/servers` → **200** for all 3 new tokens + the
  master. (`/v1/actions` → 410 — also not a valid verification target.) URL:
  `https://api.hetzner.cloud/v1/projects`.
- **Suggested fix:** verify with `GET /v1/servers` (expect 200), **never** `/v1/projects`.
- **Status:** documented (no code change).

### A2 #2 — `cx22` does not exist in the live Hetzner catalog

- **Severity:** P1.
- **Title:** `cx22` does not exist in the live Hetzner catalog — task/server-type assumption wrong.
- **Symptom:** the offering-creation catalog dropdown (`GET /cloud-accounts/<id>/catalog` → Hetzner
  `/server_types`) returns 25 types; **`cx22` is absent**. The cheapest shared x86 instance is
  **`cx23`** ($5.93/mo); cheapest ARM is `cax11` ($6.48/mo). (`cpx11` appears at $18.91/mo — the
  catalog returns USD-converted monthly prices, not raw EUR.)
- **Root cause:** the task hardcoded `cx22`. Impact: any doc/script that hardcodes `cx22` silently
  picks a non-existent type. The UI happily let the operator type an offer NAME containing "CX22"
  while selecting `cx23` — name and `provisioner_config` diverged.
- **Reproduction:** call the catalog endpoint; observe `cx22` is not in the 25 returned types.
- **Suggested fix:** update docs/scripts to `cx23` (or `cax11` for ARM).
- **Status:** documented.

### A2 #3 — No client-side guard for Hetzner `server_type × location` compatibility

- **Severity:** P1.
- **Title:** No client-side guard for Hetzner `server_type × location` compatibility — wrong combo
  fails only at final submit.
- **Symptom:** the location dropdown shows ALL 6 locations (fsn1, nbg1, hel1, ash, hil, sin), but
  `cx23` is only available in fsn1/hel1/nbg1. Selecting `cx23` + `ash` (Ashburn) passes every
  client check and only fails on `POST /providers/<me>/offerings` with the verbatim error (HTTP 200,
  body `success:false`):
  > `"Hetzner offering validation failed: Server type 'cx23' is not available in location 'ash'.
  > Available locations for 'cx23': fsn1, hel1, nbg1"`
- **Root cause:** the dropdowns do not filter each other, even though the catalog already knows
  per-type locations (Hetzner `/server_types/{id}`). Impact: provider fills 3 wizard steps, hits
  Create, gets bounced with no inline guidance.
- **Reproduction:** `https://decent-cloud.org/dashboard/offerings/create` step 2 → pick `cx23` +
  `ash` → Create → error above.
- **Suggested fix:** filter the location dropdown by the selected server_type's available
  locations (catalog already exposes them).
- **Status:** OPEN.

### A2 #4 — Create-offering POSTs to `/providers/<me>/offerings`, NOT `/offerings`

- **Severity:** P1 (architectural note, not a bug).
- **Title:** `createProviderOffering` POSTs to `/providers/<me>/offerings`, NOT `/offerings`.
- **Symptom:** `createProviderOffering` (`website/src/lib/services/api.ts:655`) POSTs to
  `/api/v1/providers/<pubkey>/offerings` (authenticated, per-provider). The top-level
  `/api/v1/offerings` is GET-only (public search). Worth recording: any automation expecting
  `POST /offerings` will 404. Also: the create response body is just `{"success":true,"data":11}`
  — the new offering id is a bare integer, no object. The UI then redirects to
  `/dashboard/offerings` and the provider must find the new row themselves.
- **Root cause:** by design (per-provider auth), but the endpoint name is misleading vs the
  architecture note.
- **Reproduction:** observe the POST target in the create flow.
- **Suggested fix:** none required (document the shape); optionally return the created object.
- **Status:** documented.

### Staging — `dc-stage` healthy internally but not publicly reachable

- **Severity:** P1 (blocks a stage account/provider live run).
- **Title:** stage (`dc-stage` namespace) is healthy internally but NOT publicly reachable.
- **Symptom:** stage is healthy internally (HTTP 200, `environment:stage`), but the
  `decent-cloud-dev` tunnel cutover to `dc-stage` is **pending** (operator action per
  `docs/MIGRATION-CUTOVER.md`). The legacy public-dev host (`dev-api.decent-cloud.org`) IS
  reachable and serves real data (96 offerings, 64 providers) but runs **STALE code**:
  `/api/v1/auth/capabilities` → **404**, retired ICP currency present on seed offerings, route
  drift. So a stage account/provider live run is BLOCKED on the tunnel cutover.
- **Root cause:** operator cutover not yet performed (Step D of the migration runbook).
- **Reproduction:** `curl https://dev-api.decent-cloud.org/api/v1/auth/capabilities` → 404; stage
  health via port-forward → 200.
- **Suggested fix:** perform the tunnel cutover (Step D); populate stage Chatwoot tokens (copy
  prod's valid ones).
- **Status:** blocked on operator.

---

## P2

### A1 #2 — Project ambiguity — account has TWO projects, not one

- **Severity:** P2.
- **Title:** Project ambiguity — account has TWO projects, not one.
- **Symptom:** `https://console.hetzner.com/projects` lists **Default** (`12666465`) and
  **Aiccelera** (`14791896`). Task assumed one project.
- **Root cause / resolution:** chose **Default (`12666465`)** because the operator's existing
  master `HETZNER_API_TOKEN` is scoped to it — `GET /v1/servers` with the master token returns
  exactly one server (`ubuntu-4gb-hel1-3`, id `124263379`), and the console shows that server lives
  in the Default project. The Aiccelera project's server is `aicellera-ubuntu-4gb-hel1`
  (different). Hypothesis: decent-cloud operator automation uses the Default project; the 3 env
  tokens mirror the master token's scope.
- **Reproduction:** list projects in the console; cross-check master-token scope via `/v1/servers`.
- **Suggested fix:** none (document the Default-project assumption).
- **Status:** documented.

### A1 #3 — Created-token value hidden behind a click-to-reveal overlay

- **Severity:** P2.
- **Title:** Created-token value is hidden behind a click-to-reveal overlay; synthetic click is
  insufficient.
- **Symptom:** after creating a token, the result dialog (`generated-token-dialog`) renders the
  token inside an `<hc-click-to-show>` element with a `.click-to-show__blur` overlay
  ("Klicken um anzuzeigen" + eye icon) over a fixed dummy placeholder ("Some random text that is
  long"). The real 64-char token is NOT in the DOM until reveal.
- **Root cause:** a synthetic `el.click()` on `.click-to-show` only revealed a truncated/masked
  51-char string (`exact:false`). A **trusted Playwright `locator('hc-click-to-show').click()`**
  was required to reveal the full 64-char value, which then appears as `hc-click-to-show`'s
  `innerText`.
- **Reproduction:** `https://console.hetzner.com/projects/<id>/security/tokens` → "API-Token
  hinzufügen" → after confirm, the `<hc-click-to-show>` block. Selector: `hc-click-to-show`.
- **Suggested fix / impact:** any automation that creates a token and reads the dialog text without
  a trusted reveal click will silently capture a wrong/truncated value — or capture nothing and lose
  the token (shown only once).
- **Status:** documented (driver hygiene).

### A1 #4 — `sops set` requires the value to be valid JSON

- **Severity:** P2.
- **Title:** `sops set` requires the value to be valid JSON.
- **Symptom:** `sops set env.yaml '["KEY"]' "$(cat file)"` with a bare alphanumeric string fails
  with `Value for --set is not valid JSON` (exit 7).
- **Root cause:** `sops set` parses the value as JSON.
- **Reproduction / fix:** pass a JSON string literal — write `JSON.stringify(token)` (→
  `"token..."`) to the temp file, then:
  `SOPS_AGE_KEY_FILE=.../secrets/.age-identity sops set .../secrets/shared/env.yaml '["HETZNER_API_TOKEN_DEV"]' "$(cat /tmp/.tk)"`.
- **Status:** documented.

### A2 #5 — Persistent console errors on every dashboard page

- **Severity:** P2 (cosmetic noise, not fatal).
- **Title:** Persistent console errors on every dashboard page (noise, not fatal).
- **Symptom (verbatim, captured via `page.on('console')` across all 4 dashboard pages):**
  - `Failed to load resource: the server responded with a status of 404` (repeated — some
    asset/route 404s; at least one from the support widget).
  - `Refused to display 'https://support.decent-cloud.org/' in a frame because it set
    'X-Frame-Options' to 'sameorigin'.` + `net::ERR_BLOCKED_BY_RESPONSE` — the embedded Chatwoot
    support widget is blocked by its own X-Frame-Options header on prod.
  - `Failed to load resource: the server responded with a status of 429` — a dashboard polling
    endpoint gets rate-limited (appeared on multiple pages).
- **Root cause:** support-widget iframe blocked by X-Frame-Options; asset 404s; polling 429s.
- **Impact:** clutters the console; the support-widget iframe never renders in the dashboard
  (provider can't see chat inline). Functionality otherwise unaffected.
- **Suggested fix:** relax Chatwoot X-Frame-Options for the dashboard origin; fix the 404 asset(s);
  back off the polling endpoint.
- **Status:** OPEN (cosmetic).

### A2 #6 — Email verification never triggered / not required

- **Severity:** P2.
- **Title:** Email verification never triggered / not required — account is `emailVerified:false`
  and nothing blocks.
- **Symptom:** registration returned `emailVerified:false`. No verification email arrived in the
  captured flow, and the dashboard, cloud-account add, offering create, and provider onboarding ALL
  succeeded with `emailVerified:false`. The success screen mentions "Check your email to verify
  your account" but it's not enforced anywhere in the provider path.
- **Root cause / impact:** a provider can list real billable offerings with an unverified email.
  May be intentional (seed-phrase auth is the real gate), but worth confirming product intent.
- **Reproduction:** register → observe `emailVerified:false` throughout the provider onboarding.
- **Suggested fix:** confirm product intent; if email should gate provider publish, enforce it.
- **Status:** OPEN (confirm intent).

### A2 #7 — Provider "registration" split across 3 surfaces; hub mis-sequences

- **Severity:** P2.
- **Title:** Provider "registration" has no single explicit step — it's split across 3 surfaces
  and the hub mis-sequences them.
- **Symptom:** `/dashboard/provider/start` presents Path A as "Add a cloud account" (step 1),
  "Create an offering" (step 2), "Complete your support profile" (step 3). But:
  - (a) the `provider_profiles` row is created only by the support onboarding PUT — so the TRUE
    prerequisite order is account → (display_name optional) → cloud-account → offering →
    support-onboarding; the hub lists support profile LAST even though it's what makes you a
    "provider" record.
  - (b) there is no separate "register provider / set provider name" form — the provider display
    name is silently inherited from `account.display_name` (fallback username) at onboarding-PUT
    time (`api/src/openapi/providers.rs` `update_provider_onboarding`).
- **Root cause / impact:** a provider following the hub order can create an offering before the
  provider row exists (the operator did). The offering then has no `provider_name` until onboarding
  is done. Not fatal, but the hub's step numbering implies a cleaner sequence than reality.
- **Suggested fix:** auto-create the provider row on first offering, OR reorder the hub so the
  support profile is step 1 (it's the actual registration).
- **Status:** OPEN.

---

## P3

### A1 #5 — Orphaned tokens created during iterative UI discovery (all deleted)

- **Severity:** P3.
- **Title:** Orphaned tokens created during iterative UI discovery; all deleted.
- **Symptom:** several "decent-cloud dev" tokens were created before the reveal mechanism (#3) and
  sops-JSON quirk (#4) were solved; their values were unrecoverable (shown once, dialog closed).
- **Resolution:** all orphans were deleted via the console (hover row → dropdown → Löschen → OK
  confirm; confirm button is `hc-button[data-test=testAcceptButton]`). Net console state: exactly
  the 3 intended tokens + the 2 pre-existing ones (`cli` master, `dev-dc`).
- **Root cause:** the click-to-reveal + JSON-value pitfalls are undocumented, so first attempts
  created dead tokens.
- **Status:** resolved.

### A1 #6 — Cosmetic: token-list rows appear duplicated in DOM scrape

- **Severity:** P3.
- **Title:** Cosmetic: token-list rows appear duplicated in DOM scrape.
- **Symptom:** querying `hc-data-view-multi-select-row, [class*=multi-select-row]` returns each row
  twice (nested matching element). Not a real duplicate — deduping by description shows the true
  count (5 unique tokens).
- **Suggested fix:** for future runs, prefer `.tokens-table__description` cells for exact
  descriptions.
- **Status:** documented.

### A2 #8 — Username `+`-tagged email is accepted

- **Severity:** P3.
- **Title:** Username `+`-tagged email is accepted; verify intended.
- **Symptom:** email `decent-cloud-prod+hetzner-reseller@decent-cloud.org` (with `+`) was accepted
  by both the client validator and `POST /api/v1/accounts`. Convenient for plus-addressing, but one
  mailbox can register many "distinct" accounts.
- **Status:** documented (likely fine).

### A2 #9 — Playwright `waitForFunction` non-boolean return dumps process `env`

- **Severity:** P3 (driver hygiene).
- **Title:** Playwright `page.waitForFunction` returning a non-boolean serializes to a JSHandle
  whose `console.log` deep-print can dump the Node process `env`.
- **Symptom:** in the registration driver, `waitForFunction` returned a string outcome; logging the
  returned JSHandle caused Playwright-core to print its internal `_platform.env` object, which
  exposed the driver process's environment (including non-prod dev secrets present in the agent env
  — e.g. `HETZNER_API_TOKEN` (DEV), `CREDENTIAL_ENCRYPTION_KEY`, various API keys). The PROD
  Hetzner token was NOT in env (it was piped via a temp file + HTOKEN and shredded), so no prod
  secret leaked — but any driver that echoes a JSHandle risks leaking whatever is in `process.env`.
- **Suggested fix (for future drivers):** always coerce `waitForFunction` results to boolean, or
  `.jsonValue()` before logging; never `console.log` a raw Handle.
- **Status:** documented.

### A2 #10 — Seed phrase recoverable from `localStorage.seed_phrases` post-registration

- **Severity:** P3.
- **Title:** Seed phrase is recoverable from `localStorage.seed_phrases` post-registration (by
  design, but worth noting).
- **Symptom:** after registration the 12-word seed is persisted in `localStorage` under key
  `seed_phrases` (JSON array) so reload keeps you signed in. Anyone with browser access to the
  operator's profile can read it. This matches the product model (seed-phrase = the credential),
  but it's worth flagging vs. a session-only design.
- **Status:** documented (by design).

### A2 #11 — Wizard step gating not enforced server-side (`?step=3` works)

- **Severity:** P3.
- **Title:** Provider onboarding help-center deep-link `?step=3` works — wizard step gating is not
  enforced server-side.
- **Symptom:** navigating straight to `/dashboard/provider/support?step=3` (skipping the Support
  Portal + Contacts steps) rendered the Help-Center form and the PUT succeeded.
  `wizard-logic.ts::parseStepParam` honors the query param over the persisted step. Convenient for
  automation, but means a provider can publish a help article without ever setting up the support
  portal / notifications (steps 1–2). Likely intentional (progressive onboarding), recorded for
  completeness.
- **Status:** documented.

---

## Reference: Console UI token-creation click-path (A1)

- Go to: `https://console.hetzner.com/projects/<PROJECT_ID>/security/tokens` (sidebar: Sicherheit →
  API-Tokens).
- Click **"API-Token hinzufügen"** (page-level button).
- In dialog: fill **Beschreibung** (description input) → under **Berechtigungen** select
  **"Lesen & Schreiben"** (Read & Write radio) → click **"API-Token hinzufügen"** (dialog confirm;
  enabled once desc is non-empty).
- Result dialog "API-Token wurde hinzugefügt": the value is hidden behind `<hc-click-to-show>` —
  **click the token box to reveal**, then copy immediately (shown once).
- Delete a token: hover its row → click the row dropdown (⋮) → **Löschen** → confirm with **OK**
  (`hc-button[data-test=testAcceptButton]`). No 2FA / no typed confirmation required.

## Reference: prod reproduction click-path (A2)

1. `/login` → "Create an account" → seed auto-generated (capture 12 words) → tick "I have saved…"
   → Continue → username + email → "Create Account" → "Go to Dashboard".
2. `/dashboard/account/profile` → set Display Name → "Save Profile".
3. `/dashboard/cloud/accounts` → "Add Account" → backend Hetzner, name, paste token → "Add Account"
   (validates live).
4. `/dashboard/offerings/create` → step1 basics (name "Hetzner CX22 (resold)", visibility `public`,
   draft OFF) → step2 (pick cloud account, server type `cx23`, location `fsn1`/`hel1`/`nbg1` ONLY —
   see A2 #3, image `ubuntu-22.04`) → step3 (accept suggested price) → "Create Offering".
5. `/dashboard/provider/support?step=3` → support-hours + ≥1 channel/region/payment → "Save &
   Publish" (creates provider row + help article).
6. ⚠ Offering will NOT appear in `/api/v1/offerings` until **P0-B** is fixed.

---

## Verification tooling

The findings above (especially P0-A) were surfaced by the new **real-deployment e2e harness** in
**PR #459** (`tools/e2e-real-deployments/`). It is re-runnable:

```bash
# health flow against prod (the flow that caught the dc-prod-secret outage)
<runner> --target prod --flows health
```

The harness **fails loud on missing keys / misconfigurations** — there is no silent-success path.
Run it post-recovery and post-cutover to confirm prod + stage are healthy and reachable.

---

## Blocked on operator

1. **Prod-outage permanence (P0-A):** run `python3 third_party/k8s/scripts/manage-secrets.py` (in
   the **k8s repo**) so `dc-prod-secret` is reconciled from the renamed SOPS file. Without this,
   the next prune/rotation re-breaks prod.
2. **Remove the unused `HETZNER_API_TOKEN` stub (P0-A):** delete the `HETZNER_API_TOKEN`
   `secretKeyRef` from `base/dc-api.yaml` (k8s repo). It is unused by the api-server (verified —
   only `api/src/bin/api-cli/e2e.rs:203` reads it from env) and acutely blocked prod.
3. **Stage tunnel cutover (Staging / P1):** repoint the `decent-cloud-dev` cloudflared tunnel →
   `dc-stage` (Step D of `docs/MIGRATION-CUTOVER.md`) so stage is publicly reachable and a stage
   live run is unblocked.
4. **Populate stage Chatwoot tokens:** copy prod's valid `CHATWOOT_PLATFORM_API_TOKEN` into the
   stage secret (the stale one 401's).
