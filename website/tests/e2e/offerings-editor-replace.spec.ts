import { test, expect } from './fixtures/test-account';
import type { Page } from '@playwright/test';
import { assertNoNativeDialog } from './fixtures/auth-helpers';
import { pubkeyHexFromSeed, seedOffering, deleteOfferingsByProvider } from './fixtures/seed-helpers';

/**
 * E2E coverage for the OfferingsEditor "load example data" replace guard
 * (inline two-step confirm).
 *
 * OfferingsEditor previously used a native confirm() dialog to guard replacing
 * existing spreadsheet data with example data — which both blocks headless
 * e2e (Playwright auto-dismisses it, so the replace never fires) and is a poor
 * mobile UX. It now uses the inline two-step pattern: when the sheet already
 * holds data, the first "Load Example Data" click reveals an inline
 * Confirm/Cancel pair; the replace runs only after Confirm.
 *
 * Serial mode: the spec seeds + deletes provider_offerings rows for the shared
 * testAccount pubkey, so tests must not run in parallel.
 */
test.describe.configure({ mode: 'serial' });

/** Read the "N data rows × M columns" header from the spreadsheet editor. */
async function dataRowCount(page: Page): Promise<number> {
	const text = (await page.getByText('data rows').first().textContent()) ?? '';
	const m = text.match(/(\d+) data rows/);
	return m ? parseInt(m[1], 10) : -1;
}

/** The inline replace-confirm bar (scoped so it doesn't match the footer Cancel). */
function replaceBar(page: Page) {
	return page
		.locator('div.flex.items-center.gap-3.flex-wrap')
		.filter({ hasText: 'Replace existing data' });
}

test.describe('Offerings editor replace guard (inline two-step confirm)', () => {
	test('first Load click reveals an inline confirm; Confirm replaces existing data', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		await seedOffering(pubkey, { name: 'E2E Replace Guard' });
		try {
			// A native dialog must never appear — fail loudly if it does.
			assertNoNativeDialog(page);

			await page.goto('/dashboard/offerings');
			// Provider has >=1 offering, so "Edit Offerings" opens the editor dialog.
			await page.getByRole('button', { name: 'Edit Offerings' }).click();
			await page.getByText('Product Type:').waitFor({ state: 'visible', timeout: 15000 });

			// Select compute (the seeded offering's product type) so the sheet shows data.
			await page.getByRole('button', { name: 'Compute' }).click();
			const loadBtn = page.getByRole('button', { name: 'Load Example Data' });
			await loadBtn.waitFor({ state: 'visible', timeout: 10000 });

			const before = await dataRowCount(page);
			expect(before).toBeGreaterThanOrEqual(1);

			// First click reveals inline Confirm/Cancel (sheet is non-empty → guard fires).
			await loadBtn.click();
			const confirmBtn = page.getByRole('button', { name: 'Confirm' });
			await expect(confirmBtn).toBeVisible();
			await expect(replaceBar(page).getByRole('button', { name: 'Cancel' })).toBeVisible();

			// Confirm replaces the data; the confirm UI disappears and the row count grows.
			await confirmBtn.click();
			await expect(page.getByRole('button', { name: 'Confirm' })).toHaveCount(0, { timeout: 10000 });
			await expect.poll(async () => dataRowCount(page), { timeout: 10000 }).toBeGreaterThan(before);
		} finally {
			await deleteOfferingsByProvider(pubkey);
		}
	});

	test('Cancel aborts the replace and keeps existing data', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		await seedOffering(pubkey, { name: 'E2E Replace Keep' });
		try {
			assertNoNativeDialog(page);

			await page.goto('/dashboard/offerings');
			await page.getByRole('button', { name: 'Edit Offerings' }).click();
			await page.getByText('Product Type:').waitFor({ state: 'visible', timeout: 15000 });
			await page.getByRole('button', { name: 'Compute' }).click();
			const loadBtn = page.getByRole('button', { name: 'Load Example Data' });
			await loadBtn.waitFor({ state: 'visible', timeout: 10000 });

			const before = await dataRowCount(page);

			// First click reveals the guard; Cancel hides it and keeps the data.
			await loadBtn.click();
			await replaceBar(page).getByRole('button', { name: 'Cancel' }).click();

			await expect(page.getByRole('button', { name: 'Confirm' })).toHaveCount(0);
			expect(await dataRowCount(page)).toBe(before);
		} finally {
			await deleteOfferingsByProvider(pubkey);
		}
	});
});
