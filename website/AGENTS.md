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
| E2E flow | `playwright.config.ts`, `tests/e2e/` | Auto-starts API and web on `59011/59010` |

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
npm run dev
npm run check
npm test
E2E_AUTO_SERVER=1 npm run test:e2e
```

## NOTES
- `src/lib/index.ts` is effectively empty; the real frontend map is under `routes/`, `services/`, `stores/`, and `utils/`.
