import { test, expect } from '@playwright/test';
import { API_BASE_URL } from './fixtures/api-base';

/**
 * #436: when Google OAuth is not configured, the login page must default to the
 * credential (seed-phrase) sign-in surface with no extra click. The server is
 * the source of truth — a public /api/v1/auth/capabilities endpoint reports
 * whether OAuth is wired, and the login page adapts.
 *
 * The warm e2e stack ships with Google OAuth DISABLED (no
 * GOOGLE_OAUTH_CLIENT_ID/SECRET), so this spec asserts the real disabled-branch
 * default. The enabled branch is unit-tested in Rust (openapi::auth::tests).
 */
test.describe('auth capabilities (#436)', () => {
	test('capability endpoint reports google_oauth=false on the e2e stack @smoke', async ({
		request,
	}) => {
		const res = await request.get(`${API_BASE_URL}/api/v1/auth/capabilities`);
		expect(res.ok()).toBe(true);
		const body = await res.json();
		expect(body).toEqual({ google_oauth: false });
	});

	test('login page defaults to the seed-phrase form when OAuth is off (no extra click) @smoke', async ({
		page,
	}) => {
		await page.goto('/login');

		// The credential form is expanded by default: the seed-phrase mode
		// chooser (Import Existing / Generate New) is visible WITHOUT first
		// clicking "Sign in with seed phrase instead".
		await expect(
			page.getByRole('button', { name: 'Import Existing' }),
		).toBeVisible({ timeout: 10_000 });
		await expect(
			page.getByRole('button', { name: 'Generate New' }),
		).toBeVisible();

		// The OAuth-first CTA is absent, and so is the "instead" toggle that
		// only makes sense when Google sign-in is offered.
		await expect(page.getByText('Sign in with Google')).toHaveCount(0);
		await expect(
			page.getByRole('button', { name: 'Sign in with seed phrase instead' }),
		).toHaveCount(0);

		// Copy reflects reality: the subtitle no longer mentions Google.
		await expect(page.getByText('Use your Google account or seed phrase')).toHaveCount(0);
	});
});
