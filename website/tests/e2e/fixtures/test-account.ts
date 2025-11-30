import { test as base } from '@playwright/test';
import {
	registerNewAccount,
	signIn,
	setupConsoleLogging,
	type AuthCredentials,
} from './auth-helpers';

// Worker-scoped credentials shared by all test variants
const baseFixtures = base.extend<{}, { testAccount: AuthCredentials }>({
	// Worker-scoped fixture that creates one test account per worker
	testAccount: [
		async ({ browser }, use) => {
			// Create test account in worker scope
			const setupPage = await browser.newPage();
			setupConsoleLogging(setupPage);
			const credentials = await registerNewAccount(setupPage);
			await setupPage.close();

			// Provide credentials to all tests
			await use(credentials);

			// Cleanup if needed
		},
		{ scope: 'worker' },
	],
});

/**
 * Test fixture for authenticated tests.
 * Creates account once per worker and signs in before each test.
 * Use this when testing features that require authentication.
 */
export const test = baseFixtures.extend<{}>({
	// Override page fixture to sign in before each test
	page: async ({ page, testAccount }, use) => {
		setupConsoleLogging(page);

		// Sign in with test account credentials
		await signIn(page, testAccount);

		// Wait for page to be fully hydrated and auth state ready
		await page.waitForLoadState('networkidle');
		await page.locator(`text=@${testAccount.username}`).waitFor({ state: 'visible', timeout: 10000 });

		// Use the authenticated page
		await use(page);
	},
});

/**
 * Test fixture for logged-out tests.
 * Creates account once per worker but does NOT auto-sign in.
 * Use this for testing sign-in/sign-up flows.
 */
export const testLoggedOut = baseFixtures.extend<{ testAccountLoggedOut: AuthCredentials }>({
	// Test-scoped fixture that provides credentials without auto-sign-in
	testAccountLoggedOut: async ({ testAccount }, use) => {
		await use(testAccount);
	},

	// Override page fixture to just set up logging (no auto-sign-in)
	page: async ({ page }, use) => {
		setupConsoleLogging(page);
		await use(page);
	},
});

export { expect } from '@playwright/test';
