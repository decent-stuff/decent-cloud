# E2E Tests

Playwright end-to-end specs for the Decent Cloud website (`tests/e2e/`).

> **`FLOWS.md` is the single source of truth** for the flow catalog (every
> user-facing flow → its spec/test), the `@smoke` coverage matrix, the mock
> policy, and the smoke selection rules. Read it before adding or changing a
> test. This file is only a quickstart pointer — it deliberately does not
> duplicate the catalog.

## Warm-stack workflow (preferred)

The fast dev loop reuses a **warm stack**: bring it up once, then iterate on the
suite in seconds (no per-run `cargo run`, no health-check wait).

| Service | URL |
|---------|-----|
| Website | `http://localhost:59010` |
| API     | `http://localhost:59011` |

```bash
# from the website/ directory
npm run e2e:up        # bring up the warm stack (scripts/dev-server.sh start --e2e)
npm run e2e:status    # check health (api:59011, web:59010)
npm run e2e:down      # tear down when done
```

> The stack starter disables the API rate limiter (`RATE_LIMIT_ENABLED=false`)
> so parallel test workers (all on 127.0.0.1) don't trip the shared 429 bucket.

### Running tests against the warm stack

```bash
npm run test:e2e:fast          # full suite (all projects, all specs)
npm run test:e2e:fast:smoke    # smoke tier only (--project=chromium --grep @smoke), ~26 tests, <35s
```

Both set `PLAYWRIGHT_BASE_URL=http://localhost:59010`. The API base is resolved
by `tests/e2e/fixtures/api-base.ts` (defaults to the web port + 1 → `59011`,
override with `PLAYWRIGHT_API_URL`).

### One-shot mode (slower)

```bash
E2E_AUTO_SERVER=1 npm run test:e2e   # spawns + tears down its own stack
```

### Interactive / debug

```bash
npm run test:e2e:ui       # Playwright UI mode
npm run test:e2e:debug    # Playwright Inspector (step through)
npm run test:e2e:headed   # visible browser window
```

## Running a subset

```bash
# One spec file (filename substring)
npm run test:e2e:fast -- account-page.spec.ts

# A filename glob — the way to run a "category" (specs are grouped by file)
npm run test:e2e:fast -- "*rental*"

# One test by title
npm run test:e2e:fast -- -g "should sign in successfully"
```

> **About tags.** The only tag matched at runtime is **`@smoke`** (it lives in
> test titles, e.g. `test('@smoke ...')`). The category labels in `FLOWS.md`
> (`@auth`, `@marketplace`, `@rental`, `@provider`, …) are **documentation-only**
> — they are NOT in test titles, so `--grep @rental` matches nothing. Run a
> category by **spec-file pattern** (above), not by `--grep`.

## Auth in tests

Most specs use the worker-scoped fast-auth fixture
`tests/e2e/fixtures/test-account.ts` (`import { test, expect } from './fixtures/test-account'`).
It skips UI sign-in by injecting the seed phrase into `localStorage` via
`addInitScript` before the first navigation, and seeds/tears a test account per
worker. Specs that need to exercise the real UI sign-in flow import `signIn`
from `fixtures/auth-helpers.ts` instead. See `website/AGENTS.md` → "E2E BEST
PRACTICES" for the full fixture/mock conventions.

## Debugging tips

- Playwright UI (`npm run test:e2e:ui`) is the fastest way to see selectors,
  network, and console output live.
- Failure artifacts (screenshots, video, traces) land in `test-results/`.
- Add `page.pause()` to drop into the Playwright Inspector mid-test.

## Connection trouble?

```bash
curl -s -o /dev/null -w "web %{http_code}\n" http://localhost:59010/
curl -s -o /dev/null -w "api  %{http_code}\n" http://localhost:59011/api/v1/health
npm run e2e:status   # same check, with health detail
```

If the ports are busy (stale `vite`/`api-server`), run `npm run e2e:down` or
kill the stale processes first.
