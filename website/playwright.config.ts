import { defineConfig, devices } from '@playwright/test';

// Auto-start servers when E2E_AUTO_SERVER=1 (for development container)
// Uses dedicated ports (59010/59011) to avoid conflicts with Docker dev (59000/59001)
const autoStartServers = process.env.E2E_AUTO_SERVER === '1';
const baseURL = process.env.PLAYWRIGHT_BASE_URL || (autoStartServers ? 'http://localhost:59010' : 'http://localhost:59000');
const apiURL = autoStartServers ? 'http://localhost:59011' : 'http://localhost:59001';
// In agent container, PostgreSQL runs on hostname 'postgres' (docker-compose service)
const databaseUrl = process.env.DATABASE_URL || 'postgres://test:test@postgres:5432/test';
const canisterId = process.env.CANISTER_ID || 'ggi4a-wyaaa-aaaai-actqq-cai';

/**
 * Playwright E2E Test Configuration
 *
 * DEFAULT: npm run test:e2e - Auto-starts API server (port 59011) + website (port 59010)
 *
 * Alternative run modes:
 *   npm run test:e2e:docker  - Expect Docker containers running (ports 59000/59001)
 *   PLAYWRIGHT_BASE_URL=http://localhost:5173 npm run test:e2e:docker - Custom servers
 *
 * The API server is built with SQLX_OFFLINE=true and uses PostgreSQL.
 */
export default defineConfig({
	testDir: './tests/e2e',
	fullyParallel: true,
	forbidOnly: !!process.env.CI,
	retries: process.env.CI ? 2 : 0,
	// Default 4 workers: measured 8 workers gives no improvement (2.9m vs 2.7m).
	// Not CPU-bound (box is 64% idle at 8 workers); not Vite-dev-bound (preview
	// mode measured same speed); not DB-pool-bound (larger pool regressed). The
	// bottleneck is the sequential nature of browser-driven page loads against a
	// single API+Postgres stack. Override per-run with E2E_WORKERS=N.
	workers: process.env.CI ? 2 : (process.env.E2E_WORKERS ? parseInt(process.env.E2E_WORKERS, 10) : 4),
	// Per-test timeout. The fast-auth fixture lands on /dashboard in <2s; 30s
	// leaves plenty of headroom for actual test body work under parallel load.
	timeout: 30_000,
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
					`bash -lc 'set -a; [ -f ./.env.local ] && . ./.env.local; set +a; CARGO_BIN="$(command -v cargo || true)"; [ -z "$CARGO_BIN" ] && CARGO_BIN="/usr/local/cargo/bin/cargo"; DATABASE_URL="${databaseUrl}" API_SERVER_PORT=59011 CANISTER_ID="${canisterId}" FRONTEND_URL=http://localhost:59010 SQLX_OFFLINE=true RATE_LIMIT_ENABLED=false "$CARGO_BIN" run --bin api-server -- serve'`,
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
