import { defineConfig, devices } from '@playwright/test';

// Auto-start servers when E2E_AUTO_SERVER=1 (for development container)
// Uses dedicated ports (59010/59011) to avoid conflicts with Docker dev (59000/59001)
const autoStartServers = process.env.E2E_AUTO_SERVER === '1';
const baseURL = process.env.PLAYWRIGHT_BASE_URL || (autoStartServers ? 'http://localhost:59010' : 'http://localhost:59000');
const apiURL = autoStartServers ? 'http://localhost:59011' : 'http://localhost:59001';

/**
 * Playwright E2E Test Configuration
 *
 * Run modes:
 *   E2E_AUTO_SERVER=1 npm run test:e2e  - Auto-start API + website servers
 *   npm run test:e2e                     - Expect Docker containers running
 *   PLAYWRIGHT_BASE_URL=http://localhost:5173 npm run test:e2e - Manual servers
 */
export default defineConfig({
	testDir: './tests/e2e',
	fullyParallel: true,
	forbidOnly: !!process.env.CI,
	retries: process.env.CI ? 2 : 0,
	workers: process.env.CI ? 2 : 16,
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
					'DATABASE_URL="sqlite:./e2e-test.db?mode=rwc" API_SERVER_PORT=59011 CANISTER_ID=ggi4a-wyaaa-aaaai-actqq-cai FRONTEND_URL=http://localhost:59010 SQLX_OFFLINE=true cargo run --bin api-server -- serve',
				cwd: '../api',
				url: apiURL,
				reuseExistingServer: !process.env.CI,
				timeout: 120_000,
			},
			{
				command: 'VITE_DECENT_CLOUD_API_URL=http://localhost:59011 npm run dev -- --port 59010',
				url: baseURL,
				reuseExistingServer: !process.env.CI,
				timeout: 30_000,
			},
		]
		: undefined,
});
