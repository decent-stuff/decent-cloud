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
| E2E fixtures | `tests/e2e/fixtures/` | `test-account.ts` (fast-auth via `addInitScript` seed injection), `test-admin-account.ts` (DB-direct admin grant), `seed-helpers.ts` (DB-direct psql seeding), `auth-helpers.ts` (UI sign-in helpers), `stripe-mock.ts` (Stripe SDK mock — external-dep boundary only). |

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
- `first_login_onboarding_completed` is also pre-set in `sessionStorage` at the context level so
  the WelcomeModal doesn't intercept clicks on underlying dashboard chrome; tests that exercise
  the WelcomeModal remove that key via a page-level `addInitScript` (page-level runs after context-level).
- Dev iteration target: smoke 4 tests in ~20 s against a warm stack; full suite ~135 tests in ~2.7 m.
- See `repo/AGENTS.md` → "Playwright E2E (repo-local)" for the full warm-stack workflow and the
  `RATE_LIMIT_ENABLED` note (parallel workers need it disabled to avoid mass 429s).
