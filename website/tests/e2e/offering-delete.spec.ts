import { test, expect } from './fixtures/test-account';
import { pubkeyHexFromSeed, seedOffering, deleteOfferingsByProvider } from './fixtures/seed-helpers';

/**
 * E2E coverage for the offering delete flow (two-step inline confirm).
 *
 * The offering delete previously used a native confirm() dialog — which both
 * blocks headless e2e (Playwright auto-dismisses it, so the delete never
 * fires) and is a poor mobile UX. It now uses the inline two-step pattern
 * (first click reveals an inline Confirm button, second click deletes),
 * mirroring the proven revoke/delete pattern on other dashboard pages.
 *
 * Serial mode: all testAccount users share one pubkey, and this spec mutates
 * (seeds + deletes) provider_offerings rows for that pubkey.
 */
test.describe.configure({ mode: 'serial' });

test.describe('Offering delete (inline two-step confirm)', () => {
	test('first Delete click reveals an inline confirm; second click deletes the offering', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const cardId = await seedOffering(pubkey, { name: 'E2E Delete Offering' });
		try {
			await page.goto('/dashboard/offerings');
			const card = page.locator(`[data-offering-id="${cardId}"]`);
			await expect(card).toBeVisible({ timeout: 10000 });

			// First click: no native dialog — it reveals an inline Confirm button.
			await card.getByRole('button', { name: 'Delete' }).click();
			const confirmBtn = card.getByRole('button', { name: 'Confirm' });
			await expect(confirmBtn).toBeVisible();

			// A Cancel button is offered so the user can back out.
			await expect(card.getByRole('button', { name: 'Cancel' })).toBeVisible();

			// Second click: performs the deletion.
			await confirmBtn.click();

			// The card must disappear after the list refetches.
			await expect(page.locator(`[data-offering-id="${cardId}"]`)).toHaveCount(0, { timeout: 10000 });
		} finally {
			await deleteOfferingsByProvider(pubkey);
		}
	});

	test('Cancel aborts the deletion and keeps the offering', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		const keepId = await seedOffering(pubkey, { name: 'E2E Keep Offering' });
		try {
			await page.goto('/dashboard/offerings');
			const card = page.locator(`[data-offering-id="${keepId}"]`);
			await expect(card).toBeVisible({ timeout: 10000 });

			await card.getByRole('button', { name: 'Delete' }).click();
			await card.getByRole('button', { name: 'Cancel' }).click();

			// Confirm button is gone, offering card remains.
			await expect(card.getByRole('button', { name: 'Confirm' })).toHaveCount(0);
			await expect(card).toBeVisible();
		} finally {
			await deleteOfferingsByProvider(pubkey);
		}
	});
});
