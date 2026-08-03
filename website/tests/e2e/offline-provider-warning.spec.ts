import { test, expect } from './fixtures/test-account';
import {
	seedMarketplaceOffering,
	deleteOfferingById,
} from './fixtures/seed-helpers';

/**
 * E2E coverage for the offline-provider warning UI on the offering detail page.
 *
 * After the drop-demos pivot (migration 053) the catalog is honestly empty, so
 * this spec self-seeds an OFFLINE offering (a plain offering with no agent pool
 * → provider_online=false) and navigates to its detail page by the known id,
 * instead of dynamically discovering a leftover demo offering. Self-seeding also
 * avoids cross-worker flakiness: the old discovery picked ANY offline offering,
 * which could be another worker's row cleaned up mid-test.
 */
test.describe('Offline Provider Warning', () => {
	let offlineId: string | undefined;
	test.beforeAll(async () => {
		const handle = await seedMarketplaceOffering({ name: 'E2E Offline Warning Offering' });
		offlineId = handle.offeringNumericId;
	});
	test.afterAll(async () => {
		if (offlineId) await deleteOfferingById(offlineId);
	});

	test('should show offline badge next to offering title', async ({ page }) => {
		await page.goto(`/dashboard/marketplace/${offlineId}`);

		// Offering detail renders an "Offline" status pill next to the title
		// when provider_online === false (see marketplace/[id]/+page.svelte).
		await page.waitForSelector('h1', { timeout: 10000 });

		const offlineBadge = page.locator('span').filter({ hasText: 'Offline' }).first();
		await expect(offlineBadge).toBeVisible();
	});

	test('should disable Rent button and explain why when provider is offline', async ({ page }) => {
		await page.goto(`/dashboard/marketplace/${offlineId}`);
		await page.waitForSelector('h1', { timeout: 10000 });

		// Rent button is replaced with a disabled "Provider Offline" button whose
		// title attribute explains the queueing behaviour.
		const offlineButton = page.getByRole('button', { name: 'Provider Offline' }).first();
		await expect(offlineButton).toBeVisible();
		await expect(offlineButton).toBeDisabled();

		const title = await offlineButton.getAttribute('title');
		expect(title).toContain('currently offline');
		expect(title).toContain('queued');
	});

	test('should not show offline warning for online provider offering', async ({ page }) => {
		await page.goto('/dashboard/marketplace');

		await page.waitForSelector('h1:has-text("Marketplace")', { timeout: 10000 });

		// Default marketplace view filters out offline offerings; if any card is
		// visible, navigating into it must NOT show the offline UI.
		const onlineOffering = page.locator('a[href^="/dashboard/marketplace/"]').first();
		if (await onlineOffering.isVisible()) {
			await onlineOffering.click();
			await page.waitForSelector('h1', { timeout: 10000 });

			const offlineButton = page.getByRole('button', { name: 'Provider Offline' });
			await expect(offlineButton).toHaveCount(0);
		}
	});
});
