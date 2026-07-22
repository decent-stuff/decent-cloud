import { test, expect } from './fixtures/test-account';
import type { Page } from '@playwright/test';

/**
 * E2E coverage for the /dashboard overview page (top-level dashboard index).
 *
 * The "My Resources" card always renders for an authenticated identity, even
 * with zero offerings — its subtitle copy is therefore stable and safe to
 * assert against without seeding.
 */

// Mirrors the helper in transfers.spec.ts: the test-account fixture injects
// the seed phrase via addInitScript, then the dashboard layout renders the
// "Logout" button once auth state propagates. Waiting on that button IS the
// auth-ready signal.
async function waitForAuthReady(page: Page) {
	await page.getByRole('button', { name: 'Logout' }).waitFor({ state: 'visible', timeout: 15000 });
}

test.describe('/dashboard overview', () => {
	test('My Resources subtitle uses unambiguous self-test copy (audit #2)', async ({ page }) => {
		// Audit #2: the previous subtitle read "Your infrastructure offerings -
		// rent for free (self-rental)" with a "Rent Free" button, which left
		// providers unsure what "free" meant or why they'd rent their own
		// offering. The copy was renamed to make the self-test semantics
		// explicit.
		await page.goto('/dashboard');
		await waitForAuthReady(page);

		const myResources = page.locator('h2', { hasText: 'My Resources' }).locator('..');
		// New wording must explicitly reference provisioning a test instance —
		// the literal "rent for free" phrasing must be gone.
		await expect(myResources).toContainText(/provision a test instance/i);
		await expect(myResources).not.toContainText(/rent for free/i);
	});

	test('dashboard loads all sections via the single combined /provider/dashboard call', async ({
		page,
	}) => {
		// The dashboard previously fanned out to 5 endpoints on every load. It
		// now makes ONE authenticated call to /provider/dashboard. This test
		// asserts (a) that combined call happens and returns all five sections,
		// and (b) the old fan-out endpoints are NOT called from the dashboard.
		const combinedResponse = page.waitForResponse(
			(resp) => resp.url().includes('/api/v1/provider/dashboard') && resp.status() === 200,
			{ timeout: 15000 },
		);

		// Track any call to the old per-section endpoints — there must be none.
		const oldEndpoints: string[] = [];
		page.on('request', (req) => {
			const url = req.url();
			if (
				url.includes('/trust-metrics') ||
				url.includes('/response-metrics') ||
				url.includes('/health-summary') ||
				url.includes('/provider/my-offerings') ||
				url.includes('/users/') && url.includes('/activity')
			) {
				oldEndpoints.push(url);
			}
		});

		await page.goto('/dashboard');
		await waitForAuthReady(page);

		const resp = await combinedResponse;
		const body = await resp.json();
		expect(body.success).toBe(true);
		// All five dashboard sections must be present in the combined payload.
		for (const key of ['trustMetrics', 'responseMetrics', 'healthSummary', 'offerings', 'activity']) {
			expect(body.data).toHaveProperty(key);
		}

		// Give the page a beat to issue any straggler requests, then assert the
		// old fan-out endpoints were never hit by the dashboard load.
		await page.waitForTimeout(500);
		expect(oldEndpoints, 'dashboard must not call the old per-section endpoints').toEqual([]);
	});
});
