import { test, expect, waitForAuthReady } from './fixtures/test-account';

/**
 * E2E coverage for the /dashboard overview page (top-level dashboard index).
 *
 * The "My Resources" card always renders for an authenticated identity, even
 * with zero offerings — its subtitle copy is therefore stable and safe to
 * assert against without seeding.
 */

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

	test('@smoke dashboard loads all sections via the single combined /provider/dashboard call', async ({
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

		// Wait for the dashboard to finish rendering (any follow-up requests
		// from the old fan-out pattern would have been issued by now).
		// Deterministic element wait replaces a fixed 500ms sleep.
		await expect(page.locator('h2', { hasText: 'My Resources' })).toBeVisible({ timeout: 5000 });
		expect(oldEndpoints, 'dashboard must not call the old per-section endpoints').toEqual([]);
	});

	test('"Get Started" CTA links to the live provider setup wizard, not a 404 (F1)', async ({
		page,
	}) => {
		// A fresh fixture account has zero offerings, so the "My Resources"
		// empty state renders a "Get Started" CTA. It must link to the setup
		// wizard (/dashboard/provider/support), which is a real route — NOT
		// /dashboard/provider (a bare directory with no +page.svelte → 404).
		await page.goto('/dashboard');
		await waitForAuthReady(page);

		// Wait for the empty state to render (My Resources section visible).
		await expect(page.locator('h2', { hasText: 'My Resources' })).toBeVisible({ timeout: 10000 });

		const cta = page.getByRole('link', { name: /Get Started/i });
		// The CTA only renders in the zero-offerings empty state. If it is not
		// present the account already has offerings and this test is a no-op
		// for that branch — but a fresh fixture account always has zero.
		await expect(cta).toBeVisible({ timeout: 10000 });

		// Clicking must navigate to the setup wizard (a live route), never a 404.
		await cta.click();
		await page.waitForURL('**/dashboard/provider/support**', { timeout: 15000 });
		await expect(page).not.toHaveURL(/.*\/error.*/i);
	});

	test('financial summary is visible for new users with $0.00 balances', async ({ page }) => {
		// A fresh fixture account has no contracts and no offerings, so
		// detectUserRole returns 'new'. The dashboard must STILL surface the
		// financial position (Spending + Earnings) at $0.00 so users can see
		// the spending/earnings surfaces from the very first visit — not a
		// blank dashboard with only CTAs.
		await page.goto('/dashboard');
		await waitForAuthReady(page);

		// Wait for the dashboard data to load so role-gated content renders.
		await expect(page.locator('h2', { hasText: 'My Resources' })).toBeVisible({ timeout: 10000 });

		// Spending metric card with $0.00 + a link to set a spending cap.
		const spendingCard = page.locator('div.metric-card').filter({ hasText: 'Spending' });
		await expect(spendingCard).toBeVisible({ timeout: 10000 });
		await expect(spendingCard).toContainText('0.00');
		await expect(spendingCard.getByRole('link', { name: 'Set cap' })).toHaveAttribute('href', '/dashboard/account/billing');

		// Earnings metric card with $0.00 + a link to the earnings detail page.
		const earningsCard = page.locator('div.metric-card').filter({ hasText: 'Earnings' });
		await expect(earningsCard).toBeVisible({ timeout: 10000 });
		await expect(earningsCard).toContainText('0.00');
		await expect(earningsCard.getByRole('link', { name: 'Details' })).toHaveAttribute('href', '/dashboard/provider/earnings');
	});
});
