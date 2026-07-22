import { test, expect } from './fixtures/test-account';

/**
 * E2E coverage for the /dashboard/account/subscription route (GAP route).
 *
 * Every account — including a freshly-seeded one — is provisioned with a
 * default "free" subscription by the backend, so the page renders BOTH a
 * "Current Plan" card (showing Free) AND the "Upgrade Your Plan" catalog with
 * the paid tiers. This spec pins that real state: the default free plan is
 * surfaced, and the upgrade catalog lists paid tiers with real prices.
 */
test.describe('/dashboard/account/subscription route', () => {
	test('renders current free plan plus the upgrade catalog with paid tiers', async ({ page }) => {
		await page.goto('/dashboard/account/subscription');

		// Scope to <main> so assertions ignore the sidebar/settings tabs.
		const main = page.locator('main');

		// Page heading — pins that the subscription page (not a redirect) loaded.
		await expect(
			main.getByRole('heading', { name: 'Subscription', exact: true }),
		).toBeVisible({ timeout: 10000 });

		// Must NOT show the auth-required card — the fixture is authenticated.
		await expect(page.getByText('Login Required')).toBeHidden();

		// The "Current Plan" card must render — every account has the default
		// free subscription. Its absence would mean the subscription lookup
		// broke or returned null unexpectedly.
		await expect(main.getByRole('heading', { level: 2, name: 'Current Plan' })).toBeVisible({
			timeout: 10000,
		});

		// The upgrade catalog heading must render. Because plan_id === 'free',
		// the h2 reads "Upgrade Your Plan" (the non-free branch reads
		// "Available Plans") — asserting this exact text pins the free-plan
		// default logic, not just that some heading rendered.
		await expect(
			main.getByRole('heading', { level: 2, name: 'Upgrade Your Plan' }),
		).toBeVisible();

		// A paid tier must render with its price point. The Pro plan's $29/mo
		// is produced by formatPrice(monthlyPriceCents=2900); asserting it pins
		// that the pricing logic rendered a real number from the plans API.
		await expect(main.getByRole('heading', { level: 3, name: 'Pro' })).toBeVisible();
		await expect(main.getByText('$29')).toBeVisible();
	});
});
