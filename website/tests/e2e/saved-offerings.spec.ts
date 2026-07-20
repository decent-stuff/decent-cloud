import { test, expect } from './fixtures/test-account';
import { pubkeyHexFromSeed, sql, nowNs } from './fixtures/seed-helpers';

/**
 * E2E coverage for /dashboard/saved.
 *
 * The saved-offerings page (wishlist) shows offerings the user has bookmarked
 * for later. Data lives in the `saved_offerings` table keyed by
 * (user_pubkey bytea, offering_id bigint).
 *
 * Coverage:
 *  - Empty state for a fresh user.
 *  - Populated state via DB-seeded saved_offerings rows.
 *  - Interactive action: unsave an offering (optimistic UI update).
 *  - Bulk action: select-all + remove.
 *  - Compare-saved CTA appears when >=2 offerings are saved.
 */

/** Insert a saved_offering row for the test user. */
async function seedSavedOffering(requesterPubkeyHex: string, offeringId: number): Promise<void> {
	await sql(`
		INSERT INTO saved_offerings (user_pubkey, offering_id, saved_at)
		VALUES (decode('${requesterPubkeyHex}', 'hex'), ${offeringId}, ${nowNs().toString()})
		ON CONFLICT (user_pubkey, offering_id) DO NOTHING
	`);
}

/** Remove all saved offerings for a user (cleanup). */
async function deleteSavedOfferingsForUser(requesterPubkeyHex: string): Promise<void> {
	await sql(`DELETE FROM saved_offerings WHERE user_pubkey = decode('${requesterPubkeyHex}', 'hex')`);
}

test.describe('/dashboard/saved', () => {
	// Helper: see transfers.spec.ts waitForAuthReady for rationale.
	async function waitForAuthReady(page: import('@playwright/test').Page) {
		await page.getByRole('button', { name: 'Logout' }).waitFor({ state: 'visible', timeout: 15000 });
	}

	test('empty state: fresh user sees empty message and Browse Marketplace CTA', async ({ page }) => {
		await page.goto('/dashboard/saved');
		await waitForAuthReady(page);

		await expect(page.getByRole('heading', { name: 'Saved Offerings' })).toBeVisible();
		await expect(page.getByText("Offerings you've saved for later", { exact: true })).toBeVisible();

		// Empty-state card
		await expect(page.getByText('No saved offerings yet.', { exact: true })).toBeVisible();
		await expect(page.getByText('Browse the marketplace to save offerings for later.', { exact: true })).toBeVisible();

		// CTA button
		const browseCta = page.getByRole('link', { name: /Browse Marketplace/ });
		await expect(browseCta).toBeVisible();
		await expect(browseCta).toHaveAttribute('href', '/dashboard/marketplace');
	});

	test('populated state: shows saved offerings with links to marketplace detail', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			// Save offerings 1 and 2 (compute-001 Basic VPS, compute-002 Performance VPS).
			await seedSavedOffering(pubkey, 1);
			await seedSavedOffering(pubkey, 2);

			await page.goto('/dashboard/saved');
			await waitForAuthReady(page);

			// Both offerings visible by their names
			await expect(page.getByRole('link', { name: 'Basic VPS' })).toBeVisible();
			await expect(page.getByRole('link', { name: 'Performance VPS' })).toBeVisible();

			// Each links to its marketplace detail page
			await expect(page.getByRole('link', { name: 'Basic VPS' })).toHaveAttribute('href', '/dashboard/marketplace/1');
			await expect(page.getByRole('link', { name: 'Performance VPS' })).toHaveAttribute('href', '/dashboard/marketplace/2');

			// 'Compare Saved' CTA appears when >=2 offerings are saved
			await expect(page.getByRole('link', { name: /Compare Saved/ })).toBeVisible();

			// 'Select all' checkbox appears when offerings are present
			await expect(page.getByText('Select all', { exact: true })).toBeVisible();
		} finally {
			await deleteSavedOfferingsForUser(pubkey);
		}
	});

	test('action: unsave a single offering removes it from the list', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			await seedSavedOffering(pubkey, 1);
			await seedSavedOffering(pubkey, 2);

			await page.goto('/dashboard/saved');
			await waitForAuthReady(page);

			// Initially both offerings are present
			await expect(page.getByRole('link', { name: 'Basic VPS' })).toBeVisible();
			await expect(page.getByRole('link', { name: 'Performance VPS' })).toBeVisible();

			// Click the unsave button (the bookmark icon button next to 'Basic VPS')
			// Each row has a 'View' link and an unsave button. Scope to the row.
			const basicVpsRow = page.locator('div.card', { hasText: 'Basic VPS' });
			const unsaveButton = basicVpsRow.locator('button[title="Remove from saved"]');
			await unsaveButton.click();

			// The Basic VPS row disappears (optimistic UI update)
			await expect(page.getByRole('link', { name: 'Basic VPS' })).toHaveCount(0);
			// Performance VPS still present
			await expect(page.getByRole('link', { name: 'Performance VPS' })).toBeVisible();
		} finally {
			await deleteSavedOfferingsForUser(pubkey);
		}
	});

	test('bulk action: Select all + Remove N selected deletes all saved', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			await seedSavedOffering(pubkey, 1);
			await seedSavedOffering(pubkey, 2);
			await seedSavedOffering(pubkey, 3);

			await page.goto('/dashboard/saved');
			await waitForAuthReady(page);

			// Click "Select all"
			const selectAllLabel = page.locator('label', { hasText: 'Select all' });
			await selectAllLabel.locator('input[type="checkbox"]').check();

			// Bulk-remove button appears
			const removeButton = page.getByRole('button', { name: /Remove 3 selected/ });
			await expect(removeButton).toBeVisible();
			await removeButton.click();

			// All offerings removed
			await expect(page.getByText('No saved offerings yet.', { exact: true })).toBeVisible({ timeout: 10000 });
		} finally {
			await deleteSavedOfferingsForUser(pubkey);
		}
	});

	test('row selection: clicking checkbox on a single row toggles bulk button', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			await seedSavedOffering(pubkey, 1);
			await seedSavedOffering(pubkey, 2);

			await page.goto('/dashboard/saved');
			await waitForAuthReady(page);

			// No bulk-remove button initially
			await expect(page.getByRole('button', { name: /Remove.*selected/ })).toHaveCount(0);

			// Check the row-level checkbox for the first offering (id=1)
			const basicVpsRow = page.locator('div.card', { hasText: 'Basic VPS' });
			await basicVpsRow.locator('input[type="checkbox"]').check();

			// Bulk-remove button now appears with "1 selected"
			await expect(page.getByRole('button', { name: /Remove 1 selected/ })).toBeVisible();

			// Uncheck — button disappears
			await basicVpsRow.locator('input[type="checkbox"]').uncheck();
			await expect(page.getByRole('button', { name: /Remove.*selected/ })).toHaveCount(0);
		} finally {
			await deleteSavedOfferingsForUser(pubkey);
		}
	});
});
