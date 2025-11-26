import { test as base } from '@playwright/test';
import {
	registerNewAccount,
	signOut,
	setupConsoleLogging,
	type AuthCredentials,
} from './auth-helpers';

/**
 * Custom fixture for creating a test account once per worker.
 * This replaces the problematic test.beforeAll({ browser }) pattern.
 */
export const test = base.extend<{}, { testAccount: AuthCredentials; testAccountLoggedOut: AuthCredentials }>({
	// Worker-scoped fixture that creates one test account per worker
	testAccount: [
		async ({ browser }, use) => {
			// Create test account
			const page = await browser.newPage();
			setupConsoleLogging(page);
			const credentials = await registerNewAccount(page);
			await page.close();

			// Provide credentials to all tests
			await use(credentials);

			// Cleanup if needed
		},
		{ scope: 'worker' },
	],

	// Worker-scoped fixture that creates a test account and signs out
	testAccountLoggedOut: [
		async ({ browser }, use) => {
			// Create test account and sign out
			const page = await browser.newPage();
			setupConsoleLogging(page);
			const credentials = await registerNewAccount(page);
			await signOut(page);
			await page.close();

			// Provide credentials to all tests
			await use(credentials);

			// Cleanup if needed
		},
		{ scope: 'worker' },
	],
});

export { expect } from '@playwright/test';
