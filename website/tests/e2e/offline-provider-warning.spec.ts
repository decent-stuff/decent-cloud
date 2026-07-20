import { test, expect } from './fixtures/test-account';

const API_BASE_URL = process.env.VITE_DECENT_CLOUD_API_URL || 'http://localhost:59011';

/**
 * Find the id of the first offline offering in the dev DB.
 *
 * The dev DB ships demo offerings (IDs 1-10 at time of writing) all with
 * `provider_online: false`. We dynamically discover an offline id rather than
 * hard-coding one (e.g. the previous `45`) so the spec survives seed churn.
 * Throws if no offline offering exists — the suite cannot exercise the
 * offline-warning UI without one.
 */
async function firstOfflineOfferingId(): Promise<number> {
	const res = await fetch(`${API_BASE_URL}/api/v1/offerings?offline=1&demo=1&per_page=100`);
	const json = await res.json();
	const offs = Array.isArray(json) ? json : (json.data ?? json.offerings ?? []);
	const offline = offs.find((o: { provider_online?: boolean }) => o.provider_online === false);
	if (!offline) {
		throw new Error('No offline offering found in dev DB; cannot exercise offline-warning UI');
	}
	return offline.id as number;
}

test.describe('Offline Provider Warning', () => {
	test('should show offline badge next to offering title', async ({ page }) => {
		const id = await firstOfflineOfferingId();
		await page.goto(`/dashboard/marketplace/${id}`);

		// Offering detail renders an "Offline" status pill next to the title
		// when provider_online === false (see marketplace/[id]/+page.svelte).
		await page.waitForSelector('h1', { timeout: 10000 });

		const offlineBadge = page.locator('span').filter({ hasText: 'Offline' }).first();
		await expect(offlineBadge).toBeVisible();
	});

	test('should disable Rent button and explain why when provider is offline', async ({ page }) => {
		const id = await firstOfflineOfferingId();
		await page.goto(`/dashboard/marketplace/${id}`);
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
