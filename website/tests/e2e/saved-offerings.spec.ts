import { test, expect, waitForAuthReady } from './fixtures/test-account';
import {
	pubkeyHexFromSeed,
	sql,
	nowNs,
	deleteSavedOfferingsForUser,
	seedOffering,
	deleteOfferingsByProvider,
	randomHex,
} from './fixtures/seed-helpers';

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
 *
 * Self-contained: offerings are seeded under a fresh provider pubkey via
 * seedOffering (not the demo seed_data.sql rows) so the test never depends on
 * externally-maintained seed rows drifting.
 */

const PROVIDER_PUBKEY = randomHex(32);
const OFFERING_NAMES = ['E2E Saved Alpha', 'E2E Saved Beta', 'E2E Saved Gamma'];
let offeringIds: string[] = [];

/** Seed the 3 offerings once for the whole spec (parallel-safe: random pubkey). */
test.beforeAll(async () => {
	for (const name of OFFERING_NAMES) {
		const id = await seedOffering(PROVIDER_PUBKEY, { name, offeringSource: 'self_provisioned' });
		offeringIds.push(id);
	}
});

test.afterAll(async () => {
	await deleteOfferingsByProvider(PROVIDER_PUBKEY);
});

/** Insert a saved_offering row for the test user. */
async function seedSavedOffering(requesterPubkeyHex: string, offeringId: string): Promise<void> {
	await sql(`
		INSERT INTO saved_offerings (user_pubkey, offering_id, saved_at)
		VALUES (decode('${requesterPubkeyHex}', 'hex'), ${offeringId}, ${nowNs().toString()})
		ON CONFLICT (user_pubkey, offering_id) DO NOTHING
	`);
}

test.describe('/dashboard/saved', () => {
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
			await seedSavedOffering(pubkey, offeringIds[0]);
			await seedSavedOffering(pubkey, offeringIds[1]);

			await page.goto('/dashboard/saved');
			await waitForAuthReady(page);

			// Both offerings visible by their names
			await expect(page.getByRole('link', { name: OFFERING_NAMES[0] })).toBeVisible();
			await expect(page.getByRole('link', { name: OFFERING_NAMES[1] })).toBeVisible();

			// Each links to its marketplace detail page
			await expect(page.getByRole('link', { name: OFFERING_NAMES[0] })).toHaveAttribute('href', `/dashboard/marketplace/${offeringIds[0]}`);
			await expect(page.getByRole('link', { name: OFFERING_NAMES[1] })).toHaveAttribute('href', `/dashboard/marketplace/${offeringIds[1]}`);

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
			await seedSavedOffering(pubkey, offeringIds[0]);
			await seedSavedOffering(pubkey, offeringIds[1]);

			await page.goto('/dashboard/saved');
			await waitForAuthReady(page);

			// Initially both offerings are present
			await expect(page.getByRole('link', { name: OFFERING_NAMES[0] })).toBeVisible();
			await expect(page.getByRole('link', { name: OFFERING_NAMES[1] })).toBeVisible();

			// Click the unsave button (the bookmark icon button next to the first offering)
			const firstRow = page.locator('div.card', { hasText: OFFERING_NAMES[0] });
			const unsaveButton = firstRow.locator('button[title="Remove from saved"]');
			await unsaveButton.click();

			// The first offering row disappears (optimistic UI update)
			await expect(page.getByRole('link', { name: OFFERING_NAMES[0] })).toHaveCount(0);
			// Second offering still present
			await expect(page.getByRole('link', { name: OFFERING_NAMES[1] })).toBeVisible();
		} finally {
			await deleteSavedOfferingsForUser(pubkey);
		}
	});

	test('bulk action: Select all + Remove N selected deletes all saved', async ({ page, testAccount }) => {
		const pubkey = pubkeyHexFromSeed(testAccount.seedPhrase);
		try {
			await seedSavedOffering(pubkey, offeringIds[0]);
			await seedSavedOffering(pubkey, offeringIds[1]);
			await seedSavedOffering(pubkey, offeringIds[2]);

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
			await seedSavedOffering(pubkey, offeringIds[0]);
			await seedSavedOffering(pubkey, offeringIds[1]);

			await page.goto('/dashboard/saved');
			await waitForAuthReady(page);

			// No bulk-remove button initially
			await expect(page.getByRole('button', { name: /Remove.*selected/ })).toHaveCount(0);

			// Check the row-level checkbox for the first offering
			const firstRow = page.locator('div.card', { hasText: OFFERING_NAMES[0] });
			await firstRow.locator('input[type="checkbox"]').check();

			// Bulk-remove button now appears with "1 selected"
			await expect(page.getByRole('button', { name: /Remove 1 selected/ })).toBeVisible();

			// Uncheck — button disappears
			await firstRow.locator('input[type="checkbox"]').uncheck();
			await expect(page.getByRole('button', { name: /Remove.*selected/ })).toHaveCount(0);
		} finally {
			await deleteSavedOfferingsForUser(pubkey);
		}
	});
});
