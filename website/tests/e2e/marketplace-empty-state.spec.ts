import { test, expect } from './fixtures/test-account';

/**
 * E2E coverage for the marketplace default-hide empty state.
 *
 * Demo offerings (is_example) and offline offerings are hidden by default
 * (showDemoOfferings / showOfflineOfferings both default to false). When every
 * offering is hidden this way, the marketplace renders an empty state with a
 * one-click "Show N offerings" reveal action — distinct from "Clear all
 * filters", which only appears when user filters are narrowing the results.
 *
 * In the warm stack every seed offering comes from the example provider
 * (is_example=true), so the default-hide empty state appears on a fresh visit
 * with no filters applied — no fixture seeding needed.
 */
test.describe('Marketplace default-hide empty state', () => {
	test('offers a reveal action when all offerings are hidden by default', async ({ page }) => {
		await page.goto('/dashboard/marketplace');

		// Empty state: no visible offerings after the default-hide filters run.
		await expect(page.getByText('No offerings found')).toBeVisible({ timeout: 10000 });
		await expect(page.locator('[id^="offering-"]')).toHaveCount(0);

		// The reveal button (not "Clear all filters") surfaces the hidden
		// demo/offline offerings. It reads "Show N offering(s)".
		const reveal = page.getByRole('button', { name: /^Show \d+ offering/ });
		await expect(reveal).toBeVisible();
		await expect(
			page.getByText('hidden because no providers are currently online'),
		).toBeVisible();

		// Clicking reveal surfaces the previously-hidden offerings.
		await reveal.click();
		await expect(page.locator('[id^="offering-"]').first()).toBeVisible({ timeout: 10000 });
	});
});
