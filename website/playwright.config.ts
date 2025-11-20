import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright E2E Test Configuration
 *
 * Prerequisites:
 * 1. Docker containers must be running (via deploy.py or docker compose)
 *    - API server: http://localhost:59001
 *    - Website: http://localhost:59000
 * 2. OR manually run servers:
 *    - API server at http://localhost:8080
 *    - Dev server at http://localhost:5173
 *
 * Run tests:
 *   npm run test:e2e         - Run all E2E tests headless
 *   npm run test:e2e:ui      - Run with Playwright UI
 *   npm run test:e2e:debug   - Run in debug mode
 */
export default defineConfig({
	testDir: './tests/e2e',
	fullyParallel: false, // Run tests sequentially to avoid DB conflicts
	forbidOnly: !!process.env.CI,
	retries: process.env.CI ? 2 : 0,
	workers: 1, // Single worker to avoid race conditions
	reporter: process.env.CI ? 'github' : 'list',

	use: {
		// Use Docker ports by default, can be overridden with PLAYWRIGHT_BASE_URL env var
		baseURL: process.env.PLAYWRIGHT_BASE_URL || 'http://localhost:59000',
		trace: 'on-first-retry',
		screenshot: 'only-on-failure',
		video: 'retain-on-failure',
	},

	projects: [
		{
			name: 'chromium',
			use: { ...devices['Desktop Chrome'] },
		},
	],

	// Don't auto-start servers - require manual setup for now
	// This ensures API is properly configured with test database
	webServer: undefined,
});
