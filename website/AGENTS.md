# WEBSITE KNOWLEDGE BASE

## OVERVIEW
`repo/website/` is the SvelteKit frontend for landing pages, marketplace, dashboard flows, provider tooling, checkout, and browser-based tests.

## STRUCTURE
```text
website/
|- src/routes/            # SvelteKit pages and layouts
|- src/lib/services/      # API clients; `api.ts` is the big central one
|- src/lib/types/generated/ # generated Rust-derived TS types
|- src/lib/utils/         # frontend-only helpers
|- tests/e2e/             # Playwright specs and fixtures
`- src/test/              # Vitest setup
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Route/UI entrypoints | `src/routes/` | Standard SvelteKit layout/page structure |
| Backend integration | `src/lib/services/api.ts` | Central fetch layer and exported frontend types |
| Shared UI state/helpers | `src/lib/stores/`, `src/lib/utils/` | Website-local only |
| Generated API types | `src/lib/types/generated/` | Do not hand-edit |
| Unit test setup | `vitest.config.ts`, `src/test/setup.ts` | jsdom + globals |
| E2E flow | `playwright.config.ts`, `tests/e2e/` | Two modes: warm-stack `test:e2e:fast` (preferred) or one-shot `E2E_AUTO_SERVER=1` (slower). See `repo/AGENTS.md` for the warm-stack workflow. |
| E2E fixtures | `tests/e2e/fixtures/` | `test-account.ts` (fast-auth via `addInitScript` seed injection), `test-admin-account.ts` (DB-direct admin grant), `seed-helpers.ts` (DB-direct psql seeding), `auth-helpers.ts` (UI sign-in helpers), `api-base.ts` (resolves the API base URL from `PLAYWRIGHT_API_URL`→baseURL port+1→59011; use in specs making direct API calls so they work on any shard stack), `stripe-mock.ts` (Stripe SDK mock — external-dep boundary only). |

## CONVENTIONS
- Keep API access centralized in `src/lib/services/` instead of ad hoc fetches inside pages.
- `src/lib/types/generated/` is generated from Rust-facing contracts; adjust the source generator path, not the generated files.
- Unit tests live under `src/**/*.{test,spec}.{js,ts}`; E2E lives in `tests/e2e/`.
- Playwright local mode uses dedicated ports `59010/59011`; Docker mode uses `59000/59001`.

## ANTI-PATTERNS
- Editing generated TS types directly.
- Smuggling API URL logic into components instead of reusing the shared service layer.
- Adding test flows under `src/` when they belong in `tests/e2e/`.

## COMMANDS
```bash
npm run dev                       # vite dev server
npm run check                     # svelte-check typecheck
npm test                          # vitest unit tests
npm run e2e:up                    # bring up warm stack via ../scripts/dev-server.sh start --e2e
npm run test:e2e:fast             # full E2E suite against warm stack (no auto-spawn)
npm run test:e2e:fast:smoke       # smoke subset (--grep @smoke)
npm run e2e:down                  # tear down warm stack
npm run e2e:status                # check stack health
E2E_AUTO_SERVER=1 npm run test:e2e  # one-shot mode (spawns + tears down its own stack)
```

## NOTES
- The fast-auth fixture (`tests/e2e/fixtures/test-account.ts`) skips UI sign-in by injecting
  `localStorage['seed_phrases']` via a context-level `addInitScript` before the first navigation.
  The per-test `page` fixture then goes to `/dashboard` and waits for the Logout button. Tests
  that explicitly exercise the UI sign-in flow can still import `signIn` from `auth-helpers.ts`.
- `first_login_onboarding_completed` is pre-set in **`localStorage`** at the context level so
  the WelcomeModal doesn't intercept clicks on underlying dashboard chrome; tests that exercise
  the WelcomeModal remove that key via a page-level `addInitScript` (page-level runs after context-level).
  (Was `sessionStorage` until 2026-07-23 — switched so returning users don't see the modal each
  new browser session.)
- Dev iteration target: smoke 4 tests in ~20 s against a warm stack; full suite ~200 tests in ~2.9 m.
- See `repo/AGENTS.md` → "Playwright E2E (repo-local)" for the full warm-stack workflow and the
  `RATE_LIMIT_ENABLED` note (parallel workers need it disabled to avoid mass 429s).

## E2E BEST PRACTICES (learned the hard way)

- **Never use `waitForLoadState('networkidle')`** — Vite HMR keeps the network busy, causing
  workers to contend on network settle. Use deterministic waits: `waitForResponse`, `waitForURL`,
  element visibility, or the shared `clickAndRetry(page, target, success)` helper
  (`fixtures/auth-helpers.ts`) for SSR'd buttons whose onclick binds on hydration. Zero
  `networkidle` calls in the suite as of 2026-07-23.
- **Never use `registerNewAccount()` in API-only tests** — it runs a 10-15s UI registration flow.
  Use `seedAccountDirect()` (DB-direct INSERT) or the `testAccount` fixture instead.
- **Serial mode for shared-pubkey DB tests**: all `testAccount` users share the same pubkey.
  Specs that seed/delete DB rows for that pubkey (e.g. invoices.spec.ts) must use
  `test.describe.configure({ mode: 'serial' })` to avoid parallel cleanup nuking other tests' data.
- **SvelteKit hydration**: on SSR'd routes (e.g. `/login`), SSR renders the UI immediately but
  client-side event handlers attach only after hydration. `expect(button).toBeVisible()` passes
  on SSR output, but `.click()` is a no-op if hydration hasn't completed. For routes where you
  need to interact immediately after `goto`, either wait for a specific client-rendered element
  or use `waitForResponse` on an API call the page makes.
- **Shared helpers** (DRY): `waitForAuthReady(page)` in test-account.ts, `seedOffering()` /
  `deleteSavedOfferingsForUser()` / `sql()` in seed-helpers.ts. Reuse, don't copy-paste.
- **Mock policy**: only Stripe SDK and outbound external HTTP may be mocked. Never mock
  first-party API code — if you need error-path injection, do it DB-side or document an exception.
- **Every I/O path needs an explicit timeout — especially in fixture teardown** (A2). Worker-scoped
  fixture setup/teardown (e.g. the `testAccount` fixture's `deleteAccountByUsername`) and
  `beforeAll`/`afterAll` hooks run OUTSIDE the per-test `timeout`, so an unbounded op there hangs
  the whole suite for minutes with no output (one stalled worker blocks every test queued on it).
  The historical bug: `sql()`/`psql` had no timeout, so a teardown `DELETE FROM accounts` that
  blocked on a row lock held by an in-flight API transaction (FOR-KEY-SHARE via the
  `signature_audit` FK) waited forever under 2+ workers (serial mode never hit the race). Rules:
  - All `psql`/DB calls go through `sql()` / `psqlExec` (bounded by `DEFAULT_PSQL_TIMEOUT_MS`,
    overridable via `sql(query, { timeoutMs })`) — never spawn a bare `execFile('psql', …)`.
  - All `fetch()` calls (e.g. `signedApiCall`) use `AbortSignal.timeout(ms)`.
  - All `page.waitForResponse`/`waitForSelector`/`waitForURL` pass an explicit `{ timeout }`.
  Regression guard: `tests/e2e/db-helper-bounded.spec.ts`.
