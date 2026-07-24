import { test, expect } from './fixtures/test-account';
import {
	seedRentableOffering,
	deleteOfferingsByProvider,
} from './fixtures/seed-helpers';

// Dev DB ships 10 demo offerings (IDs 1-10). Use two of them as real compare
// fixtures — the test only asserts URL canonicalization + clipboard content,
// never the specific offering data, so the real seeded data is sufficient.
const OFFERING_A_ID = 1;
const OFFERING_B_ID = 2;

test.describe('Marketplace compare sharing', () => {
	test('@smoke copies canonical comparison URL and shows success feedback', async ({ page }) => {
		await page.goto(`/dashboard/marketplace/compare?ids=${OFFERING_B_ID},${OFFERING_A_ID},${OFFERING_B_ID}`);
		await expect(page).toHaveURL(`/dashboard/marketplace/compare?ids=${OFFERING_B_ID},${OFFERING_A_ID}`);

		await page.getByRole('button', { name: 'Share comparison' }).click();
		await expect(page.getByText('Comparison link copied to clipboard')).toBeVisible();

		const clipboardText = await page.evaluate(async () => navigator.clipboard.readText());
		expect(clipboardText).toBe(`${new URL(page.url()).origin}/dashboard/marketplace/compare?ids=${OFFERING_B_ID},${OFFERING_A_ID}`);
	});
});

/**
 * Full multi-offering compare view (FLOWS.md "Compare offerings" — was ⚠️:
 * share-URL only; the side-by-side table rendering was never asserted).
 *
 * Seeds two rentable offerings under fresh random provider pubkeys (no shared
 * testAccount pubkey → no serial mode needed) with distinct names, then asserts
 * the compare page fetches both via getOffering and renders them as column
 * headers in the comparison table. Dev-DB demo offerings are deliberately
 * avoided (offline/hidden — OPEN_ISSUES H6) in favour of self-contained rows.
 */
test.describe('Marketplace compare full view', () => {
	test('renders the side-by-side comparison table for two seeded offerings', async ({ page }) => {
		const a = await seedRentableOffering({ name: 'E2E Compare Alpha' });
		const b = await seedRentableOffering({ name: 'E2E Compare Beta' });
		try {
			await page.goto(`/dashboard/marketplace/compare?ids=${a.offeringNumericId},${b.offeringNumericId}`);

			// The comparison <h1> is the page-loaded signal.
			await expect(page.getByRole('heading', { name: 'Compare Offerings' })).toBeVisible({ timeout: 15000 });

			// Both offering names render as column-header links inside the table.
			// The page only renders the table once getOffering resolves for every id,
			// so the presence of both names IS the "data loaded" assertion.
			await expect(page.getByRole('link', { name: a.offeringName }).first()).toBeVisible({ timeout: 15000 });
			await expect(page.getByRole('link', { name: b.offeringName }).first()).toBeVisible();

			// Section headers prove the full comparison table rendered (Pricing +
			// Compute rows), not just the header row.
			await expect(page.locator('table').getByText('Pricing', { exact: true })).toBeVisible();
			await expect(page.locator('table').getByText('Compute', { exact: true })).toBeVisible();
		} finally {
			await deleteOfferingsByProvider(a.providerPubkeyHex);
			await deleteOfferingsByProvider(b.providerPubkeyHex);
		}
	});
});
