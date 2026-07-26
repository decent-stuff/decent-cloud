import { test, expect } from '@playwright/test';
import { API_BASE_URL } from './fixtures/api-base';

/**
 * #436: the login page must default to the credential (seed-phrase) sign-in
 * surface when Google OAuth is NOT configured — no extra click required. The
 * server is the single source of truth: a public /api/v1/auth/capabilities
 * endpoint reports whether OAuth is wired, and the login page adapts.
 *
 * This spec asserts the REAL contract regardless of how the warm stack is
 * configured: it reads the capability first, then asserts the login page's
 * default surface matches it. (Earlier revision hardcoded `google_oauth=false`,
 * which broke once the secrets sync populated GOOGLE_OAUTH_CLIENT_ID — the
 * server owns this value, not the test.)
 */
test.describe('auth capabilities (#436)', () => {
	test('capability endpoint returns a well-formed boolean @smoke', async ({ request }) => {
		const res = await request.get(`${API_BASE_URL}/api/v1/auth/capabilities`);
		expect(res.ok()).toBe(true);
		const body = await res.json();
		expect(body).toHaveProperty('google_oauth');
		expect(typeof body.google_oauth).toBe('boolean');
	});

	test('login page default surface matches the server capability @smoke', async ({ page }) => {
		// Read the single source of truth, then drive the page and assert the
		// contract: when OAuth is OFF the seed-phrase form is the default; when
		// OAuth is ON the Google CTA is the default and seed-phrase is one click away.
		const capsRes = await page.request.get(`${API_BASE_URL}/api/v1/auth/capabilities`);
		const { google_oauth: googleOAuthEnabled } = await capsRes.json();

		await page.goto('/login');

		if (googleOAuthEnabled) {
			// OAuth is the default surface.
			await expect(page.getByText('Sign in with Google')).toBeVisible({ timeout: 10_000 });
			// The seed-phrase chooser is NOT expanded until the user opts in.
			await expect(page.getByRole('button', { name: 'Import Existing' })).toHaveCount(0);
			await expect(page.getByRole('button', { name: 'Generate New' })).toHaveCount(0);
			// The "instead" toggle is the one affordance that reveals the seed-phrase path.
			await expect(
				page.getByRole('button', { name: 'Sign in with seed phrase instead' }),
			).toBeVisible();
		} else {
			// No Google OAuth: the credential form is expanded by default — the
			// seed-phrase mode chooser is visible WITHOUT first clicking anything.
			await expect(
				page.getByRole('button', { name: 'Import Existing' }),
			).toBeVisible({ timeout: 10_000 });
			await expect(page.getByRole('button', { name: 'Generate New' })).toBeVisible();
			// The OAuth-first CTA is absent, and so is the "instead" toggle that
			// only makes sense when Google sign-in is offered.
			await expect(page.getByText('Sign in with Google')).toHaveCount(0);
			await expect(
				page.getByRole('button', { name: 'Sign in with seed phrase instead' }),
			).toHaveCount(0);
			// Copy reflects reality: the subtitle no longer mentions Google.
			await expect(page.getByText('Use your Google account or seed phrase')).toHaveCount(0);
		}
	});
});
