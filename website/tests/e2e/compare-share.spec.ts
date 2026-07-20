import { test, expect } from './fixtures/test-account';

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
