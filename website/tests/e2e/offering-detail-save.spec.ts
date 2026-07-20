import { test, expect } from './fixtures/test-account';
import { pubkeyHexFromSeed, sql, nowNs } from './fixtures/seed-helpers';

/**
 * E2E coverage for the offering-detail save (bookmark) flow.
 *
 * Other specs cover the saved-offerings page (read/unsave/bulk). This spec
 * pins the *write* path: a user lands on an offering's detail page and saves
 * it with a single click on the visible bookmark toggle (previously the only
 * path was 'More options' → 'Save', a 2-click flow).
 */

async function deleteSavedOfferingsForUser(requesterPubkeyHex: string): Promise<void> {
	await sql(`DELETE FROM saved_offerings WHERE user_pubkey = decode('${requesterPubkeyHex}', 'hex')`);
}

test.describe('Offering detail save flow', () => {
	test('bookmark toggle on offering detail page saves in a single click', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			// Demo offering 1 (Basic VPS) is hidden by default filters, but the
			// detail route loads it directly regardless of marketplace filter state.
			await page.goto('/dashboard/marketplace/1');

			// The bookmark toggle is visible alongside the Rent button.
			const bookmark = page.getByRole('button', { name: /Save Basic VPS/i });
			await expect(bookmark).toBeVisible();
			await expect(bookmark).toHaveAttribute('aria-pressed', 'false');

			// Single click saves. The aria-label flips to 'Remove … from saved'
			// and the button reflects the pressed state.
			await bookmark.click();
			const savedToggle = page.getByRole('button', { name: /Remove Basic VPS from saved/i });
			await expect(savedToggle).toBeVisible();
			await expect(savedToggle).toHaveAttribute('aria-pressed', 'true');

			// Saved listing page reflects the new save.
			await page.goto('/dashboard/saved');
			await expect(page.getByRole('link', { name: 'Basic VPS' })).toBeVisible();
		} finally {
			await deleteSavedOfferingsForUser(pubkey);
		}
	});
});
