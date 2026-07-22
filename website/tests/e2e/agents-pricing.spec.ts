import { test, expect } from '@playwright/test';

/**
 * E2E coverage for the /agents/pricing route (GAP route).
 *
 * The page is a static marketing page. The meaningful behavior to pin is that
 * the single-tier pricing card renders its price point and bullets, and that a
 * CTA + back-link let the visitor progress to sign-up or back to the overview.
 * Anonymous access is the intended audience (pre-signup evaluation).
 */
test.describe('/agents/pricing route', () => {
	test('renders the single pricing tier with a price point and CTAs', async ({ page }) => {
		await page.goto('/agents/pricing');

		// Page heading — pins that the pricing page (not a 404 or fallback) loaded.
		await expect(
			page.getByRole('heading', { level: 1, name: /CHF 49/i }),
		).toBeVisible();

		// The PricingCard component renders the exact price point. This is the
		// core unit of information a visitor comes to this page to read.
		await expect(page.getByText('CHF 49', { exact: true })).toBeVisible();
		await expect(page.getByText('/ month', { exact: true })).toBeVisible();

		// At least one plan bullet (the included-capacity list) must render.
		// These bullets are what distinguish the tier; a card with a price but
		// no bullets would be a regression.
		await expect(page.getByText('Up to 20 active agent-hours / month')).toBeVisible();

		// The CTA must be an anchor that points into the agents funnel.
		const cta = page.getByRole('link', { name: /Start beta/i });
		await expect(cta).toBeVisible();
		await expect(cta).toHaveAttribute('href', '/agents#waitlist');

		// The back-to-overview link must be present and target the agents page.
		const backLink = page.getByRole('link', { name: /Back to Decent Agents overview/i });
		await expect(backLink).toBeVisible();
		await expect(backLink).toHaveAttribute('href', '/agents');
	});
});
