import { test, expect } from './fixtures/test-account';
import {
	seedMarketplaceOffering,
	deleteOfferingById,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the marketplace default-hide empty state.
 *
 * Demo offerings (is_example) and offline offerings are hidden by default
 * (showDemoOfferings / showOfflineOfferings both default to false). When every
 * offering is hidden this way, the marketplace renders an empty state with a
 * one-click "Show N offerings" reveal action — distinct from "Clear all
 * filters", which only appears when user filters are narrowing the results.
 *
 * After the drop-demos pivot (migration 053) the catalog is honestly empty, so
 * this spec self-seeds an is_example (demo) offering to reproduce the
 * default-hide empty state — the very condition the reveal action exists for.
 * The offering is self_provisioned (so it clears the marketplace query's
 * pool/self-provisioned filter) and under the example provider pubkey (so
 * is_example=true → hidden by the default showDemoOfferings=false filter). It is
 * NOT testing a totally empty catalog (that would have no hidden rows and thus
 * no reveal button).
 */
test.describe('Marketplace default-hide empty state', () => {
	let seededId: string | undefined;
	test.beforeAll(async () => {
		// is_example + self_provisioned → hidden by the default demo filter
		// (showDemoOfferings=false), so it counts toward defaultHiddenCount
		// without appearing in the default view.
		const handle = await seedMarketplaceOffering({
			isExample: true,
			online: true,
			name: 'E2E Hidden Demo Offering',
		});
		seededId = handle.offeringNumericId;
	});
	test.afterAll(async () => {
		if (seededId) await deleteOfferingById(seededId);
	});

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
