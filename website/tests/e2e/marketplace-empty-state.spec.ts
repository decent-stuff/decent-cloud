import { test, expect } from '@playwright/test';

test.describe('Marketplace empty-state defaults', () => {
	test('when all offerings are hidden by demo/offline defaults, offers a reveal action', async ({
		page,
	}) => {
		await page.goto('/dashboard/marketplace');

		// Wait for the offerings grid/empty-state to settle (not the spinner).
		const spinner = page.locator('.animate-spin');
		await expect(spinner).toBeHidden({ timeout: 10000 });

		const emptyState = page.locator('text=No offerings found');
		const offeringCount = page.locator(
			'[id^="offering-"], [data-testid="offering-card"]',
		);

		// If there ARE offerings visible, the default-hide issue doesn't apply
		// right now — nothing to assert. This keeps the test robust to seed state.
		if ((await offeringCount.count()) > 0) {
			test.skip(true, 'offerings already visible; default-hide scenario N/A');
			return;
		}

		// The empty state must exist and offer a one-click reveal.
		await expect(emptyState).toBeVisible();

		// "Clear all filters" should NOT be the only escape — it resets to the
		// same hiding defaults. A distinct reveal action must exist.
		const reveal = page.locator(
			'button:has-text("Show "), button:has-text("Show demo"), button:has-text("Show all")',
		);
		await expect(reveal).toBeVisible();

		// Clicking reveal must surface the hidden offerings.
		await reveal.first().click();
		await expect(offeringCount.first()).toBeVisible({ timeout: 10000 });
	});
});
