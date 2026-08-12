import { defineConfig, devices } from '@playwright/test';

// Auto-start servers when E2E_AUTO_SERVER=1 (for development container).
// The warm stack (the default dev workflow per repo/AGENTS.md) runs on 59010/59011.
// Docker mode (59000/59001) is opt-in: `npm run test:e2e:docker` sets PLAYWRIGHT_BASE_URL.
const autoStartServers = process.env.E2E_AUTO_SERVER === '1';
const baseURL = process.env.PLAYWRIGHT_BASE_URL || 'http://localhost:59010';
const apiURL = process.env.PLAYWRIGHT_API_URL || 'http://localhost:59011';
// In agent container, PostgreSQL runs on hostname 'postgres' (docker-compose service)
const databaseUrl = process.env.DATABASE_URL || 'postgres://test:test@postgres:5432/test';
const canisterId = process.env.CANISTER_ID || 'ggi4a-wyaaa-aaaai-actqq-cai';

/**
 * Playwright E2E Test Configuration
 *
 * DEFAULT: bare `npx playwright test` or `npm run test:e2e:fast` hits the warm stack
 * at http://localhost:59010 (brought up via `npm run e2e:up`). No env required.
 *
 * Auto-spawn mode: `npm run test:e2e` (sets E2E_AUTO_SERVER=1) spawns its own
 * API (59011) + website (59010) and tears them down afterwards.
 *
 * Docker mode: `npm run test:e2e:docker` expects Docker containers on 59000/59001
 * (sets PLAYWRIGHT_BASE_URL=http://localhost:59000). Override any URL via env.
 *
 * The API server is built with SQLX_OFFLINE=true and uses PostgreSQL.
 */
export default defineConfig({
	testDir: './tests/e2e',
	fullyParallel: true,
	forbidOnly: !!process.env.CI,
	retries: process.env.CI ? 2 : 0,
	// 4 workers locally + CI. Previously capped at 2 because 4 Chromium workers
	// contended under the dev box's persistent CPU baseline (agent harness + MCP
	// servers) and produced intermittent auth-settle timeouts (~1 flake/run).
	// That flake was fixed at the source by hardening the auth waits: pages that
	// fetch signed `/api/v1/` data now gate on `waitForResponse` (or the Logout
	// button via waitForAuthReady) instead of ad-hoc content-text visibility,
	// so signed-API-gated renders no longer depend on a quiet CPU. 4 workers
	// cuts the full suite ~40% vs 2. Override per-run with E2E_WORKERS=N.
	workers: process.env.E2E_WORKERS ? parseInt(process.env.E2E_WORKERS, 10) : 4,
	// Per-test timeout. The fast-auth fixture lands on /dashboard in <2s; 30s
	// leaves plenty of headroom for actual test body work under parallel load.
	timeout: 30_000,
	// Expect (auto-retry) timeout. The default 5s is too tight on the dev box,
	// which runs the agent harness + MCP servers as a persistent CPU baseline:
	// under 2-worker contention, signed-API-gated page renders (rentals,
	// invoices, …) intermittently exceed 5s and flake (~1/run, all green in
	// isolation / on an idle box). 10s absorbs that load variance with ZERO
	// cost to green runs (elements appear in <1s when uncontended). Per-test
	// deterministic waits (waitForResponse) remain the preferred pattern; this
	// is the load-tolerant safety net for assertions not yet hardened.
	expect: { timeout: 10_000 },
	reporter: process.env.CI ? 'github' : 'list',

	use: {
		baseURL,
		trace: 'on-first-retry',
		screenshot: 'only-on-failure',
		video: 'retain-on-failure',
		permissions: ['clipboard-read', 'clipboard-write'],
	},

	projects: [
		{
			name: 'chromium',
			use: {
				...devices['Desktop Chrome'],
				permissions: ['clipboard-read', 'clipboard-write'],
			},
		},
	],

	webServer: autoStartServers
		? [
			{
				command:
					`bash -lc 'set -a; [ -f ./.env.local ] && . ./.env.local; set +a; CARGO_BIN="$(command -v cargo || true)"; [ -z "$CARGO_BIN" ] && CARGO_BIN="/usr/local/cargo/bin/cargo"; DATABASE_URL="${databaseUrl}" API_SERVER_PORT=59011 CANISTER_ID="${canisterId}" FRONTEND_URL=http://localhost:59010 SQLX_OFFLINE=true RATE_LIMIT_ENABLED=false STRIPE_WEBHOOK_SECRET=whsec_test_secret "$CARGO_BIN" run --bin api-server -- serve'`,
				cwd: '../api',
				url: apiURL,
				// Reuse a warm server if one is already responding. CI gets a fresh
				// spawn (nothing running yet); local dev reuses the long-running
				// stack so test iterations take seconds, not minutes.
				reuseExistingServer: true,
				timeout: 120_000,
			},
			{
				command: 'VITE_DECENT_CLOUD_API_URL=http://localhost:59011 VITE_CHATWOOT_WEBSITE_TOKEN= VITE_CHATWOOT_BASE_URL= npm run dev -- --host 127.0.0.1 --port 59010 --strictPort',
				url: baseURL,
				reuseExistingServer: true,
				timeout: 30_000,
			},
		]
		: undefined,
});
