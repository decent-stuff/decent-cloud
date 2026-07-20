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
			const setupContext = await browser.newContext();
			const setupPage = await setupContext.newPage();
			setupConsoleLogging(setupPage);
			const credentials = await registerNewAccount(setupPage);
			await setupContext.close();

			// Provide credentials to all tests
			await use(credentials);
		},
		{ scope: 'worker' },
	],
});

/**
 * Test fixture for authenticated tests.
 * Creates account once per worker. Each test starts on /dashboard with the
 * session pre-seeded via localStorage — no per-test UI sign-in flow.
 *
 * The fast path (addInitScript + goto /dashboard) replaces ~5s of UI clicks
 * per test ("Sign in with seed phrase instead" → "Import Existing" → fill →
 * "Continue" → "Go to Dashboard") with a single navigation. Under 16 parallel
 * workers this is the difference between a 4-minute suite and a 1-minute one.
 */
export const test = baseFixtures.extend({
	// Override context: pre-seed seed_phrases in localStorage before any page
	// navigation. The website reads this on load to authenticate silently.
	// Also dismiss the first-login WelcomeModal so it doesn't intercept clicks
	// on the underlying dashboard chrome. Tests that explicitly exercise the
	// modal (first-login-onboarding.spec.ts) clear sessionStorage to opt back in.
	context: async ({ context, testAccount }, use) => {
		const seed = testAccount.seedPhrase;
		await context.addInitScript((s: string) => {
			localStorage.setItem('seed_phrases', JSON.stringify([s]));
			sessionStorage.setItem('first_login_onboarding_completed', 'true');
		}, seed);
		await use(context);
	},

	// Override page: skip UI sign-in; land directly on /dashboard authenticated.
	page: async ({ page }, use) => {
		setupConsoleLogging(page);
		await page.goto('/dashboard');
		// Logout button visibility IS the auth-ready signal; do not wait for
		// networkidle (vite HMR keeps the network busy and tanks parallel runs).
		await page.getByRole('button', { name: 'Logout' }).waitFor({ state: 'visible', timeout: 15000 });
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

/**
 * signIn() is re-exported for tests that explicitly need to exercise the UI
 * sign-in flow (e.g. signin-flow.spec.ts). Most authenticated tests should NOT
 * call this — the `test` fixture already lands them authenticated.
 */
export { signIn };

export { expect } from '@playwright/test';
