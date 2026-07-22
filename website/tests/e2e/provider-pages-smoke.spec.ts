import { test, expect } from './fixtures/test-account';

/**
 * E2E smoke coverage for the /dashboard/provider/* sub-pages (GAP routes).
 *
 * The fixture account is a non-provider with zero offerings, so each page must
 * render its authenticated chrome (page heading) plus its provider-setup /
 * empty-state messaging — NOT the auth-required gate, and NOT a crash. These
 * are smoke tests: each pins that the route renders a meaningful, route-
 * specific state for the no-offerings user rather than a generic shell.
 */

// Pages whose empty state is a single distinctive sentence. Asserting that
// sentence proves the page loaded its data branch (not just the heading).
const EMPTY_STATE_PAGES = [
	{
		url: '/dashboard/provider/analytics',
		heading: 'Offering Analytics',
		emptyText: 'No offerings found. Create offerings to see conversion data.',
	},
	{
		url: '/dashboard/provider/feedback',
		heading: 'Tenant Feedback',
		emptyText: 'No feedback received yet.',
	},
	{
		url: '/dashboard/provider/password-resets',
		heading: 'Password Resets',
		emptyText: 'No pending password reset requests.',
	},
	{
		url: '/dashboard/provider/reseller',
		heading: 'Reseller Program',
		emptyText: 'You are not reselling for any providers yet',
	},
	{
		url: '/dashboard/provider/sla',
		heading: 'SLA Monitor',
		emptyText: 'No offerings found.',
	},
	{
		url: '/dashboard/provider/ssh-key-rotations',
		heading: 'SSH Key Rotations',
		emptyText: 'No pending SSH key rotation requests.',
	},
] as const;

for (const p of EMPTY_STATE_PAGES) {
	test.describe(`/dashboard/provider/* smoke`, () => {
		test(`${p.url} renders heading "${p.heading}" and its empty state`, async ({ page }) => {
			await page.goto(p.url);
			const main = page.locator('main');

			// Authenticated chrome: heading present, auth gate absent.
			await expect(main.getByRole('heading', { name: p.heading, exact: true })).toBeVisible({
				timeout: 10000,
			});
			await expect(page.getByText('Login Required')).toBeHidden();

			// Route-specific empty-state text — proves the page's data branch
			// executed for a zero-offerings provider, not a fallback shell.
			await expect(main.getByText(p.emptyText)).toBeVisible({ timeout: 10000 });
		});
	});
}

// The agents page has no single empty-state sentence; its meaningful
// authenticated chrome is the "+ New Pool" action button, which only renders
// once auth + data load settle (guarded by `isAuthenticated && !loading`).
test('/dashboard/provider/agents renders heading and the "New Pool" action', async ({ page }) => {
	await page.goto('/dashboard/provider/agents');
	const main = page.locator('main');

	await expect(main.getByRole('heading', { name: 'Agents', exact: true })).toBeVisible({
		timeout: 10000,
	});
	await expect(page.getByText('Login Required')).toBeHidden();

	// The "+ New Pool" button is the authenticated, data-loaded signal.
	await expect(main.getByRole('button', { name: '+ New Pool' })).toBeVisible({ timeout: 10000 });
});

// The earnings page renders a revenue overview panel regardless of data; the
// meaningful zero-provider state is that the overview shows with the panel
// heading (the numbers default to zero, which we can't assert without a
// fixed-format check). Pin the panel heading + absence of the auth gate.
test('/dashboard/provider/earnings renders heading and revenue overview panel', async ({ page }) => {
	await page.goto('/dashboard/provider/earnings');
	const main = page.locator('main');

	await expect(main.getByRole('heading', { name: 'Provider Earnings', exact: true })).toBeVisible({
		timeout: 10000,
	});
	await expect(page.getByText('Login Required')).toBeHidden();

	// The "Revenue Overview" panel heading only renders after auth + data load
	// settle — it's the authenticated-data-branch signal for this page.
	await expect(
		main.getByRole('heading', { level: 2, name: 'Revenue Overview' }),
	).toBeVisible({ timeout: 10000 });
});
