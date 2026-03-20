# SCRIPTS KNOWLEDGE BASE

## OVERVIEW
`repo/scripts/` holds developer automation: browser verification, auth/session bootstrapping, local helpers, and one-off tooling used across the monorepo.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Browser smoke checks | `browser.js` | Fresh browser per invocation; preferred UI verification tool |
| Auth/session bootstrap | `dc-auth.js` | Creates accounts, injects seed phrases, seeds UX data |
| API/CLI helpers | `*.js`, `*.sh` | One-off local tooling; inspect before adding new scripts |

## BROWSER COMMANDS
```bash
# Visible text/structure snapshot — use for most verification (fast, cheap)
node scripts/browser.js snap <url>
# Screenshot — use for visual layout checks (saves to file, then Read it)
node scripts/browser.js shot <url> [/tmp/out.png]
# Evaluate a JS expression in page context
node scripts/browser.js eval <url> "document.title"
# Console errors/warnings only — use to check for JS errors after a change
node scripts/browser.js errs <url>
# Raw HTML — use when you need to inspect full DOM structure
node scripts/browser.js html <url>
# Click an element and return snapshot
node scripts/browser.js click <url> <selector>
# Fill an input field and return snapshot
node scripts/browser.js fill <url> <selector> <value>
# Wait for selector to appear and return snapshot
node scripts/browser.js wait <url> <selector>
# Health check — returns JSON with page title, errors, warnings
node scripts/browser.js health <url>
# Page tour — visit key routes, capture snapshots, save JSON report to /tmp/dc-ux-tour.json
node scripts/browser.js tour --seed <phrase>
```

### Options
- `--seed <phrase>` — Inject seed phrase for authenticated testing
- `--viewport mobile` — Use 375x812 (iPhone X) viewport for mobile testing
- `--timeout <ms>` — Override navigation timeout (default: 20000)
- `--wait-api` — Wait for /api/v1/ response after navigation

### Environment Variables
- `DC_WEB_URL` — Frontend URL (default: `http://localhost:5173`)
- `BROWSER_TIMEOUT` — Navigation timeout in ms (default: 20000)
- `BROWSER_ENGINE` — "chromium" or "firefox" (default: chromium)

## AUTH HELPERS
```bash
# Create a new account + log in browser (generates a random seed phrase)
# -> outputs: { username, email, seed, pubkey }
node scripts/dc-auth.js create-user [username] [email]

# Log in as an existing user (inject seed, navigate to dashboard)
# -> outputs: { username, pubkey }
node scripts/dc-auth.js login-user <seed phrase words...>

# Create a draft offering (become provider) + open provider offerings page
# -> outputs: { pubkey, offeringId, offeringName }
node scripts/dc-auth.js create-provider <seed phrase words...>

# Bootstrap a fully online UX test provider with 3 rentable KVM offerings
# -> outputs: { seed, agentSeed, pubkey, poolId, offeringIds: [id1, id2, id3] }
node scripts/dc-auth.js seed-ux-data [seed phrase words...]

# Create 1-3 test contracts against public offerings
# -> outputs: { seed, pubkey, contractIds: [...], contractStates: [{id, state}, ...] }
node scripts/dc-auth.js seed-contracts [seed phrase words...]

# Create an offline provider with offerings (no agent heartbeat) — shows as "Offline" in marketplace
# -> outputs: { seed, pubkey, offeringId }
node scripts/dc-auth.js seed-edge-cases
```

### Authenticating individual browser.js calls
Pass `--seed` to authenticate a single `browser.js` invocation without a persistent session:
```bash
SEED="word1 word2 ... word12"
node scripts/browser.js snap https://dev.decent-cloud.org/dashboard/rentals --seed "$SEED"
```
The `--seed` flag: injects the seed into `localStorage`, navigates to `/dashboard` and waits for the `/api/v1/accounts` API response (so auth store is fully settled), then navigates to the target URL.

Local dev server (port 5173): set `DC_WEB_URL=http://localhost:5173`.
Remote dev environment: omit `DC_WEB_URL` (defaults to remote).

## CONVENTIONS
- `browser.js` and `dc-auth.js` use separate browser instances; use `--seed` when a one-off browser invocation needs auth.
- Prefer extending existing scripts over adding near-duplicate helpers.
- `snap` is the default verification mode; `shot` is only for visual layout evidence.
- Local frontend default is `DC_WEB_URL=http://localhost:5173`.

## KNOWN PITFALLS
- UX test providers created by `seed-ux-data` go offline when the heartbeat daemon stops.
- For Playwright-like clicks, prefer `role=button[name='Rent']` over `button:has-text('Rent')` because of `inert` behavior.

## ANTI-PATTERNS
- Reimplementing browser flows directly in ad hoc shell snippets when `browser.js` already covers the use case.
- Assuming auth state persists between `dc-auth.js` and `browser.js` runs.
