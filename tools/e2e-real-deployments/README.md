# decent-cloud real-deployment e2e harness

A standalone, parametrized end-to-end test harness that fires against **real**
decent-cloud deployments — dev, stage, or prod — by providing API keys +
endpoints. It is reusable by anyone: it launches its **own headless Chromium**
(not the shared operator CDP), signs requests with the captured account identity,
and reports `[PASS]/[FAIL]` per flow with an aggregate summary.

> **Non-negotiable loud-fail contract.** If a selected flow needs a config field
> that is missing / empty / `PLACEHOLDER`, the harness prints a precise stderr
> error naming the field **and** its env var, then exits `2`. It never reports
> success when required inputs are missing. No silent failures, ever.

## Quick start

```bash
# From the repo root. No `npm install` needed — the harness reuses the repo's
# Playwright / @noble / bip39 under website/node_modules. (Or run `npm install`
# here for a fully standalone install.)

# List available flows:
node tools/e2e-real-deployments/run.js --list

# Keyless dry-run against any target's PUBLIC endpoints (health + marketplace):
DC_E2E_TARGET=prod DC_E2E_API_URL=https://api.decent-cloud.org \
  node tools/e2e-real-deployments/run.js --target prod --flows health
```

## Running against an env

```bash
# Stage, all non-gated flows (health, marketplace, signup, provider-onboard-path-a):
DC_E2E_HETZNER_TOKEN=<real token> DC_E2E_ACCOUNT_EMAIL_PREFIX=e2e \
  node tools/e2e-real-deployments/run.js --target stage

# Just the public, keyless flows:
node tools/e2e-real-deployments/run.js --target prod --flows health,marketplace

# The money flow (rent→provision→SSH→cancel) — OFF by default:
DC_E2E_HETZNER_TOKEN=<token> DC_E2E_ACCOUNT_EMAIL_PREFIX=e2e DC_E2E_INCLUDE_PROVISION=1 \
  node tools/e2e-real-deployments/run.js --target stage --include-provision
```

Config precedence (highest wins): **env var > `--config <path>` file > `targets/<target>.json`**.

### Config fields & env vars

| field               | env var                        | required by                                           |
|---------------------|--------------------------------|-------------------------------------------------------|
| `target`            | `DC_E2E_TARGET`                | health                                                |
| `apiUrl`            | `DC_E2E_API_URL`               | every flow                                            |
| `webUrl`            | `DC_E2E_WEB_URL`               | signup, provider-onboard, rent-provision-cancel       |
| `hetznerToken`      | `DC_E2E_HETZNER_TOKEN`         | provider-onboard-path-a, rent-provision-cancel        |
| `accountEmailPrefix`| `DC_E2E_ACCOUNT_EMAIL_PREFIX`  | signup (and its dependents)                           |
| `includeProvision`  | `DC_E2E_INCLUDE_PROVISION`     | optional gate for rent-provision-cancel               |
| `expectedEnvironment` | `DC_E2E_EXPECTED_ENVIRONMENT`| optional — asserts `/health` environment (else just recorded) |

See [`targets/README.md`](./targets/README.md) for the file format. `PLACEHOLDER`
in a file is treated as **missing** so you cannot accidentally run a flow with an
unset secret.

## Flows

List with `--list`. Each is independently runnable via `--flows <name,...>`;
prerequisites are auto-included (e.g. `--flows provider-onboard-path-a` pulls in
`signup` automatically).

| # | flow                    | what it asserts |
|---|-------------------------|-----------------|
| 1 | `health`                | `GET /api/v1/health` → 200 + `success` + `environment` (matches `expectedEnvironment` if set); `GET /auth/capabilities` → 200 + parses (`google_oauth`). A clean 404 on capabilities is a **finding**, not a fail. |
| 2 | `marketplace`           | `GET /stats` + `GET /offerings` return the documented shape; records counts. Honest-empty is a PASS; **prod with 0 offerings is a finding**. Also asserts catalog honesty: a non-empty catalog that is ALL `is_example` demos, or has ZERO rentable (`provider_online`) offerings, **fails on prod / finds elsewhere** — naming total/example/online counts + the example-pubkey ASCII so the operator can act. |
| 3 | `console-errors`        | Loads `/` + `/dashboard/marketplace` headlessly and collects severe browser-console errors (uncaught exceptions, `ERR_BLOCKED_BY_RESPONSE`, X-Frame-Options refusals, first-party asset 4xx/5xx). Catches defects invisible to the API-only flows (e.g. a broken Chatwoot widget). One FINDING per severe error; a navigation failure or a ≥3-error pile-up **fails**. |
| 4 | `drift`                 | Diffs key read-only endpoints between this target and prod (the reference): `/health` environment, `/auth/capabilities` status + `google_oauth`, `/stats`, and `/offerings` currency/payment_methods (flags retired ICP/BTC). One FINDING per meaningful diff; a DNS failure reaching either side is itself a finding, never a crash. When the target IS prod, there is nothing to diff. Override the reference with `DC_E2E_DRIFT_REFERENCE_API`. |
| 5 | `stats-honesty`         | Heuristic: `active_providers` vs the count of `provider_online` offerings (`/offerings?limit=100`). `active_providers>0` with zero online offerings is a **finding** pointing at the retired-table stat bug. Findings only — never a hard fail. |
| 6 | `signup`                | Drives the website sign-up flow headlessly, captures the 12-word seed phrase, asserts the logged-in (dashboard) state. Cleans up by **noting** the test account email (no public delete-account API). |
| 7 | `provider-onboard-path-a` | The core reseller flow, as the signed-up user, via **signed API calls**: register provider_profile → `POST /cloud-accounts` (Hetzner token validated **live**) → `GET /cloud-accounts/:id/catalog` (live-fetches server types) → create ONE offering from the catalog (cheapest / `cx23`) → assert it appears. Cleans up the offering + cloud account. |
| 8 | `rent-provision-cancel` | **[GATED, costs money]** Creates a real rental contract on the offering, waits for provisioning, asserts SSH `:22` reachable, then cancels + asserts cleanup. Forced to `cx23`. Default OFF; needs `--include-provision` / `DC_E2E_INCLUDE_PROVISION=1`. |

### How signing works (no API key to register — the website is the only path)

There is no "register by API key" endpoint; accounts are created via the website
sign-up flow (seed phrase). The harness captures that seed phrase, derives the
Ed25519 identity exactly as the website does (`identity.ts`), and signs provider
API calls with the same scheme the server verifies (`common/api_auth.rs` +
`api/auth.rs`): canonical message `timestamp‖nonce‖METHOD‖/api/v1…path‖body`,
Ed25519ph with context `b"decent-cloud"`. The signer is in `src/crypto.js`.

## Output & exit codes

Each flow prints `[PASS]/[FAIL] flowname — detail`. A FAIL detail includes the
failing assertion + the last HTTP status/body excerpt (truncated) so it is
debuggable from the log. `[FINDING]` lines are non-fatal warnings.

- exit `0` — all selected flows passed (findings are allowed)
- exit `1` — at least one flow failed
- exit `2` — configuration validation failed (missing required input)
- exit `3` — unhandled harness error

## Quality / safety

- Every HTTP request is bounded by a timeout (`AbortController`) — no unbounded fetches.
- No silent `.catch(() => {})`; errors are surfaced (as findings or fails).
- Provider-onboarding cleanup runs in a `finally` — the offering + cloud account are deleted best-effort even on failure.
- The rent-provision-cancel flow cancels the contract unconditionally and confirms a terminal `cancelled`/`terminated` state.
- `MINIMIZE-CLOUD-SPENDING`: provisioning is forced to the cheapest server type (`cx23` when available; `cx22` was retired by Hetzner).

## Layout

```
tools/e2e-real-deployments/
├── run.js                 # CLI entrypoint (arg parsing, validation, aggregation)
├── package.json           # standalone project (deps also resolvable from website/node_modules)
├── src/
│   ├── deps.js            # resolves deps from ./node_modules then website/node_modules
│   ├── config.js          # TargetConfig, loadConfig, validateConfig (loud-fail)
│   ├── http.js            # DRY timed-out fetch helper + failDetail/excerpt
│   ├── crypto.js          # seed→identity + Ed25519ph signed-request builder
│   ├── browser.js         # own headless Chromium + website sign-up driver
│   ├── assert.js          # AssertionError primitive
│   ├── runner.js          # flow registry, dependency expansion, aggregation
│   └── flows/             # health, marketplace, consoleErrors, drift, statsHonesty, signup, providerOnboardPathA, rentProvisionCancel
└── targets/               # dev.json, stage.json, prod.json (+ README)
```
